pub mod table;

mod blueprint;
mod ecosystem;
mod journey;
mod pain_matrix;
mod persona;

use std::collections::HashMap;

use regex::Regex;

use crate::error::ServiceDesignError;
use crate::model::{EmotionalScore, ServiceDesignArtifact};

/// Simple frontmatter representation: a map of string key-value pairs.
/// This avoids coupling to veneer-mdx.
pub type Frontmatter = HashMap<String, String>;

/// Trait for artifact-specific parsers that convert markdown content into
/// typed service design artifacts.
pub trait ArtifactParser: Send + Sync {
    /// Returns true if this parser can handle the given content/frontmatter.
    fn can_parse(&self, content: &str, frontmatter: &Frontmatter) -> bool;

    /// Parse the content into a service design artifact.
    fn parse(
        &self,
        content: &str,
        frontmatter: &Frontmatter,
    ) -> Result<ServiceDesignArtifact, ServiceDesignError>;
}

/// Registry of all built-in artifact parsers. Tries each parser in order
/// and returns the first successful match.
pub struct ArtifactParserRegistry {
    parsers: Vec<Box<dyn ArtifactParser>>,
}

impl ArtifactParserRegistry {
    /// Create a new registry with all built-in parsers registered.
    pub fn new() -> Self {
        let parsers: Vec<Box<dyn ArtifactParser>> = vec![
            Box::new(blueprint::BlueprintParser),
            Box::new(journey::JourneyMapParser),
            Box::new(ecosystem::EcosystemMapParser),
            Box::new(persona::PersonaParser),
            Box::new(pain_matrix::PainPointMatrixParser),
        ];
        Self { parsers }
    }

    /// Attempt to parse the given content using registered parsers.
    /// Returns `Ok(None)` if no parser matches the content.
    pub fn parse(
        &self,
        content: &str,
        frontmatter: &Frontmatter,
    ) -> Result<Option<ServiceDesignArtifact>, ServiceDesignError> {
        for parser in &self.parsers {
            if parser.can_parse(content, frontmatter) {
                let artifact = parser.parse(content, frontmatter)?;
                return Ok(Some(artifact));
            }
        }
        Ok(None)
    }
}

impl Default for ArtifactParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse an emotional score pattern like "Frustrated (-2)" or "Curiosity (+1)"
/// into an `(EmotionalScore, label)` tuple.
///
/// Supports patterns:
/// - "Label (-2)"
/// - "Label (+1)"
/// - "Label (0)"
/// - "Label (-2). Some extra text"
pub fn parse_emotional_score(text: &str) -> Result<(EmotionalScore, String), ServiceDesignError> {
    let re = Regex::new(r"^(.+?)\s*\(([+-]?\d+)\)").map_err(|e| ServiceDesignError::Parse {
        file: String::new(),
        message: format!("regex error: {e}"),
    })?;

    let caps = re
        .captures(text.trim())
        .ok_or_else(|| ServiceDesignError::Parse {
            file: String::new(),
            message: format!("could not parse emotional score from: {text}"),
        })?;

    let label = caps[1].trim().to_string();
    let value: i8 = caps[2].parse().map_err(|_| ServiceDesignError::Parse {
        file: String::new(),
        message: format!("invalid score number in: {text}"),
    })?;

    let score = EmotionalScore::new(value)?;
    Ok((score, label))
}

// -- Shared parsing helpers --

/// Extract bold field values like "**Key:** Value" from text content.
pub(crate) fn extract_bold_field(content: &str, field: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        let prefix = format!("**{field}:**");
        if let Some(rest) = trimmed.strip_prefix(&prefix) {
            return Some(rest.trim().to_string());
        }
        // Also handle the case without trailing colon in the bold
        let prefix_alt = format!("**{field}**:");
        if let Some(rest) = trimmed.strip_prefix(&prefix_alt) {
            return Some(rest.trim().to_string());
        }
    }
    None
}

/// Extract a bulleted list following a given label line (e.g., "**Pain points:**").
pub(crate) fn extract_bulleted_list_after(content: &str, label: &str) -> Vec<String> {
    let mut found = false;
    let mut items = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if !found {
            if trimmed.to_lowercase().contains(&label.to_lowercase()) {
                found = true;
            }
            continue;
        }
        if let Some(item) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            items.push(item.trim().to_string());
        } else if trimmed.is_empty() {
            // Allow blank lines within a list
            continue;
        } else {
            // Non-list content, stop
            break;
        }
    }
    items
}

/// Extract a numbered list from content.
pub(crate) fn extract_numbered_list(content: &str) -> Vec<String> {
    let re = Regex::new(r"^\d+\.\s+(.+)$").expect("valid regex");
    content
        .lines()
        .filter_map(|line| re.captures(line.trim()).map(|caps| caps[1].to_string()))
        .collect()
}

/// Extract blockquotes from content.
pub(crate) fn extract_blockquotes(content: &str) -> Vec<String> {
    let mut quotes = Vec::new();
    let mut current_quote = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(quote_text) = trimmed.strip_prefix("> ") {
            current_quote.push(quote_text.trim().to_string());
        } else if trimmed == ">" {
            current_quote.push(String::new());
        } else if !current_quote.is_empty() {
            quotes.push(current_quote.join(" ").trim().to_string());
            current_quote.clear();
        }
    }
    if !current_quote.is_empty() {
        quotes.push(current_quote.join(" ").trim().to_string());
    }

    quotes
}

/// Find a section's content by heading text (case-insensitive prefix match).
pub(crate) fn find_section_content<'a>(
    sections: &'a [table::Section],
    heading: &str,
) -> Option<&'a str> {
    sections
        .iter()
        .find(|s| {
            s.heading
                .to_lowercase()
                .starts_with(&heading.to_lowercase())
        })
        .map(|s| s.content.as_str())
}

/// Check if frontmatter has a given type value or content contains a heading.
pub(crate) fn detect_by_type_or_heading(
    content: &str,
    frontmatter: &Frontmatter,
    type_value: &str,
    heading_pattern: &str,
) -> bool {
    if let Some(t) = frontmatter.get("type") {
        if t == type_value {
            return true;
        }
    }
    content.contains(heading_pattern)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_emotional_score_negative() {
        let (score, label) = parse_emotional_score("Frustrated (-2)").unwrap();
        assert_eq!(score.value(), -2);
        assert_eq!(label, "Frustrated");
    }

    #[test]
    fn parse_emotional_score_positive() {
        let (score, label) = parse_emotional_score("Curiosity (+1)").unwrap();
        assert_eq!(score.value(), 1);
        assert_eq!(label, "Curiosity");
    }

    #[test]
    fn parse_emotional_score_zero() {
        let (score, label) = parse_emotional_score("Skeptical but interested (0)").unwrap();
        assert_eq!(score.value(), 0);
        assert_eq!(label, "Skeptical but interested");
    }

    #[test]
    fn parse_emotional_score_with_trailing_text() {
        let (score, label) =
            parse_emotional_score("Frustrated (-2). Some extra context here").unwrap();
        assert_eq!(score.value(), -2);
        assert_eq!(label, "Frustrated");
    }

    #[test]
    fn parse_emotional_score_invalid() {
        assert!(parse_emotional_score("No score here").is_err());
    }

    #[test]
    fn parse_emotional_score_out_of_range() {
        assert!(parse_emotional_score("Extreme (5)").is_err());
    }

    #[test]
    fn extract_bold_field_basic() {
        let content = "**Primary Persona:** Developer\n**Trigger:** User clicks button";
        assert_eq!(
            extract_bold_field(content, "Primary Persona"),
            Some("Developer".to_string())
        );
        assert_eq!(
            extract_bold_field(content, "Trigger"),
            Some("User clicks button".to_string())
        );
        assert_eq!(extract_bold_field(content, "Missing"), None);
    }

    #[test]
    fn extract_bulleted_list_basic() {
        let content = "**Pain points:**\n- First issue\n- Second issue\n\nOther text";
        let items = extract_bulleted_list_after(content, "pain points");
        assert_eq!(items, vec!["First issue", "Second issue"]);
    }

    #[test]
    fn extract_numbered_list_basic() {
        let content = "1. First goal\n2. Second goal\n3. Third goal\n";
        let items = extract_numbered_list(content);
        assert_eq!(items, vec!["First goal", "Second goal", "Third goal"]);
    }

    #[test]
    fn extract_blockquotes_basic() {
        let content = "> This is a quote\n\n> Another quote";
        let quotes = extract_blockquotes(content);
        assert_eq!(quotes, vec!["This is a quote", "Another quote"]);
    }

    #[test]
    fn registry_returns_none_for_unknown() {
        let registry = ArtifactParserRegistry::new();
        let result = registry
            .parse("just some random text", &Frontmatter::new())
            .unwrap();
        assert!(result.is_none());
    }
}
