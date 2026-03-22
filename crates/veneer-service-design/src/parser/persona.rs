use crate::error::ServiceDesignError;
use crate::model::{PainPoint, Persona, PersonaOverview, ServiceDesignArtifact};
use crate::parser::table::extract_sections;
use crate::parser::{
    extract_blockquotes, extract_bold_field, extract_numbered_list, find_section_content,
    ArtifactParser, Frontmatter,
};

pub struct PersonaParser;

impl ArtifactParser for PersonaParser {
    fn can_parse(&self, content: &str, frontmatter: &Frontmatter) -> bool {
        if let Some(t) = frontmatter.get("type") {
            if t == "persona" {
                return true;
            }
        }
        // Must have both Overview and Goals headings
        let has_overview = content.contains("## Overview") || content.contains("## Demographics");
        let has_goals = content.contains("## Goals");
        has_overview && has_goals
    }

    fn parse(
        &self,
        content: &str,
        frontmatter: &Frontmatter,
    ) -> Result<ServiceDesignArtifact, ServiceDesignError> {
        let sections = extract_sections(content);

        let overview = parse_overview(content, frontmatter, &sections)?;
        let background = find_section_content(&sections, "Background")
            .unwrap_or("")
            .trim()
            .to_string();

        // Goals from numbered list
        let goals_content = find_section_content(&sections, "Goals").unwrap_or("");
        let goals = extract_numbered_list(goals_content);

        // Pain points / frustrations
        let frustrations_content = find_section_content(&sections, "Frustration")
            .or_else(|| find_section_content(&sections, "Pain Point"))
            .unwrap_or("");
        let pain_points = parse_pain_points(frustrations_content);

        // Behaviors from bulleted list
        let behaviors = find_section_content(&sections, "Behavior")
            .map(extract_bulleted_items)
            .unwrap_or_default();

        // Quotes from blockquotes
        let quotes = extract_blockquotes(content);

        Ok(ServiceDesignArtifact::Persona(Persona {
            overview,
            background,
            goals,
            pain_points,
            current_tools: vec![],
            behavioral_patterns: behaviors,
            technology_expertise: vec![],
            success_metrics: vec![],
            quotes,
        }))
    }
}

fn parse_overview(
    _content: &str,
    frontmatter: &Frontmatter,
    sections: &[crate::parser::table::Section],
) -> Result<PersonaOverview, ServiceDesignError> {
    let demographics_content = find_section_content(sections, "Demographics")
        .or_else(|| find_section_content(sections, "Overview"))
        .unwrap_or("");

    let name = frontmatter
        .get("title")
        .cloned()
        .or_else(|| extract_bold_field(demographics_content, "Name"))
        .unwrap_or_default();

    let role = extract_bold_field(demographics_content, "Role")
        .or_else(|| extract_bold_field(demographics_content, "Title"))
        .unwrap_or_default();

    let age = extract_bold_field(demographics_content, "Age");
    let location = extract_bold_field(demographics_content, "Location");
    let experience = extract_bold_field(demographics_content, "Experience");

    Ok(PersonaOverview {
        name,
        age,
        role,
        location,
        experience,
    })
}

fn parse_pain_points(content: &str) -> Vec<PainPoint> {
    let items = extract_numbered_list(content);
    items
        .into_iter()
        .map(|item| {
            // Try to split on "**Label.** description" pattern
            if let Some((label, problem)) = parse_labeled_item(&item) {
                PainPoint {
                    label,
                    problem,
                    workaround: None,
                    cost: None,
                    evidence: None,
                }
            } else {
                PainPoint {
                    label: item.clone(),
                    problem: item,
                    workaround: None,
                    cost: None,
                    evidence: None,
                }
            }
        })
        .collect()
}

/// Parse "**Label.** Description text" into (label, description).
fn parse_labeled_item(text: &str) -> Option<(String, String)> {
    let trimmed = text.trim();
    if !trimmed.starts_with("**") {
        return None;
    }
    // Find the closing **
    let rest = &trimmed[2..];
    let end = rest.find("**")?;
    let label = rest[..end].trim_end_matches('.').trim().to_string();
    let after = rest[end + 2..].trim_start_matches('.').trim().to_string();
    Some((label, after))
}

fn extract_bulleted_items(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
                .map(|s| s.trim().to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn load_fixture() -> String {
        include_str!("test_fixtures/sample_persona.md").to_string()
    }

    #[test]
    fn can_parse_by_frontmatter() {
        let parser = PersonaParser;
        let mut fm = HashMap::new();
        fm.insert("type".to_string(), "persona".to_string());
        assert!(parser.can_parse("", &fm));
    }

    #[test]
    fn can_parse_by_headings() {
        let parser = PersonaParser;
        let content = "## Overview\nSome overview\n## Goals\n1. Goal one";
        assert!(parser.can_parse(content, &HashMap::new()));
    }

    #[test]
    fn parse_fixture() {
        let parser = PersonaParser;
        let content = load_fixture();
        let mut fm = HashMap::new();
        fm.insert("title".to_string(), "Alex Chen".to_string());
        fm.insert("type".to_string(), "persona".to_string());

        let result = parser.parse(&content, &fm).unwrap();
        if let ServiceDesignArtifact::Persona(persona) = result {
            assert_eq!(persona.overview.name, "Alex Chen");
            assert!(!persona.overview.role.is_empty());
            assert!(!persona.goals.is_empty());
            assert!(!persona.pain_points.is_empty());
            assert!(!persona.behavioral_patterns.is_empty());
            assert!(!persona.quotes.is_empty());
        } else {
            panic!("expected Persona variant");
        }
    }

    #[test]
    fn parse_labeled_item_works() {
        let (label, desc) =
            parse_labeled_item("**Config complexity.** Too many files to manage").unwrap();
        assert_eq!(label, "Config complexity");
        assert_eq!(desc, "Too many files to manage");
    }

    #[test]
    fn parse_labeled_item_none_for_plain() {
        assert!(parse_labeled_item("Just a plain item").is_none());
    }
}
