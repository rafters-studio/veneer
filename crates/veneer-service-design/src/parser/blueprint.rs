use regex::Regex;

use crate::error::ServiceDesignError;
use crate::model::{Blueprint, BlueprintMeta, BlueprintStep, PainPoint, ServiceDesignArtifact};
use crate::parser::table::extract_tables;
use crate::parser::{
    detect_by_type_or_heading, extract_bold_field, extract_bulleted_list_after,
    parse_emotional_score, ArtifactParser, Frontmatter,
};

pub struct BlueprintParser;

impl ArtifactParser for BlueprintParser {
    fn can_parse(&self, content: &str, frontmatter: &Frontmatter) -> bool {
        detect_by_type_or_heading(content, frontmatter, "blueprint", "## Service Blueprint")
    }

    fn parse(
        &self,
        content: &str,
        frontmatter: &Frontmatter,
    ) -> Result<ServiceDesignArtifact, ServiceDesignError> {
        let title = frontmatter
            .get("title")
            .cloned()
            .or_else(|| extract_title_from_h1(content))
            .unwrap_or_default();

        let primary_persona = extract_bold_field(content, "Primary Persona")
            .ok_or_else(|| ServiceDesignError::MissingField("Primary Persona".into()))?;

        let trigger = extract_bold_field(content, "Trigger")
            .ok_or_else(|| ServiceDesignError::MissingField("Trigger".into()))?;

        let scope = extract_bold_field(content, "Scope")
            .ok_or_else(|| ServiceDesignError::MissingField("Scope".into()))?;

        let channels = extract_bold_field(content, "Channels")
            .map(|c| c.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        let secondary_persona = extract_bold_field(content, "Secondary Persona");

        let meta = BlueprintMeta {
            title,
            primary_persona,
            secondary_persona,
            trigger,
            scope,
            channels,
        };

        let steps = parse_steps(content)?;

        let design_decisions = extract_bulleted_list_after(content, "## Design Decisions");
        let open_questions = extract_bulleted_list_after(content, "## Open Questions");

        Ok(ServiceDesignArtifact::Blueprint(Blueprint {
            meta,
            steps,
            dependency_map: vec![],
            design_decisions,
            open_questions,
        }))
    }
}

fn extract_title_from_h1(content: &str) -> Option<String> {
    content
        .lines()
        .find(|line| line.starts_with("# "))
        .map(|line| line.trim_start_matches('#').trim().to_string())
}

fn parse_steps(content: &str) -> Result<Vec<BlueprintStep>, ServiceDesignError> {
    let step_re =
        Regex::new(r"(?m)^### Step \d+:\s*(.+)$").map_err(|e| ServiceDesignError::Parse {
            file: String::new(),
            message: format!("regex error: {e}"),
        })?;

    let step_positions: Vec<(usize, String)> = step_re
        .captures_iter(content)
        .filter_map(|cap| {
            let m = cap.get(0)?;
            Some((m.start(), cap[1].trim().to_string()))
        })
        .collect();

    let mut steps = Vec::new();

    for (i, (start, name)) in step_positions.iter().enumerate() {
        let end = step_positions
            .get(i + 1)
            .map(|(pos, _)| *pos)
            .unwrap_or(content.len());

        let section = &content[*start..end];
        let step = parse_single_step(section, name)?;
        steps.push(step);
    }

    Ok(steps)
}

fn parse_single_step(section: &str, name: &str) -> Result<BlueprintStep, ServiceDesignError> {
    let tables = extract_tables(section);

    // Look for the vertical table with Layer | Detail columns
    let mut evidence = String::new();
    let mut customer_actions = String::new();
    let mut frontstage = String::new();
    let mut backstage = String::new();
    let mut support_processes = String::new();

    for table in &tables {
        if table.headers.iter().any(|h| h.to_lowercase() == "layer") {
            for row in &table.rows {
                if row.len() >= 2 {
                    let layer = row[0].to_lowercase();
                    let detail = &row[1];
                    if layer.contains("evidence") {
                        evidence = detail.clone();
                    } else if layer.contains("customer") {
                        customer_actions = detail.clone();
                    } else if layer.contains("frontstage") {
                        frontstage = detail.clone();
                    } else if layer.contains("backstage") {
                        backstage = detail.clone();
                    } else if layer.contains("support") {
                        support_processes = detail.clone();
                    }
                }
            }
        }
    }

    // Parse pain points
    let pain_point_items = extract_bulleted_list_after(section, "pain points");
    let pain_points: Vec<PainPoint> = pain_point_items
        .into_iter()
        .map(|item| PainPoint {
            label: item.clone(),
            problem: item,
            workaround: None,
            cost: None,
            evidence: None,
        })
        .collect();

    // Parse emotional state
    let (emotional_state, emotional_label) = parse_emotional_state_line(section)?;

    // Parse metrics (optional)
    let metrics = extract_bulleted_list_after(section, "metrics");

    // Parse moments of truth (optional)
    let mot_items = extract_bulleted_list_after(section, "moments of truth");
    let moments_of_truth = mot_items
        .into_iter()
        .map(|item| crate::model::MomentOfTruth {
            moment: item,
            success_state: String::new(),
            failure_state: String::new(),
            why_it_matters: None,
        })
        .collect();

    Ok(BlueprintStep {
        name: name.to_string(),
        evidence,
        customer_actions,
        frontstage,
        backstage,
        support_processes,
        pain_points,
        emotional_state,
        emotional_label,
        metrics,
        moments_of_truth,
    })
}

fn parse_emotional_state_line(
    section: &str,
) -> Result<(crate::model::EmotionalScore, String), ServiceDesignError> {
    for line in section.lines() {
        let trimmed = line.trim();
        // Match "**Emotional state:**" or "**Emotional State:**"
        let lower = trimmed.to_lowercase();
        if lower.starts_with("**emotional state") || lower.starts_with("**emotional state") {
            // Extract text after the bold field
            let after_colon = trimmed
                .find(":**")
                .map(|i| &trimmed[i + 3..])
                .or_else(|| trimmed.find(":** ").map(|i| &trimmed[i + 4..]))
                .unwrap_or(trimmed);
            let clean = after_colon.trim();
            if !clean.is_empty() {
                return parse_emotional_score(clean);
            }
        }
    }

    // Default to neutral if not found
    Ok((
        crate::model::EmotionalScore::new(0).map_err(|e| ServiceDesignError::Parse {
            file: String::new(),
            message: format!("default score failed: {e}"),
        })?,
        "Neutral".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn load_fixture() -> String {
        include_str!("test_fixtures/sample_blueprint.md").to_string()
    }

    #[test]
    fn can_parse_by_frontmatter() {
        let parser = BlueprintParser;
        let mut fm = HashMap::new();
        fm.insert("type".to_string(), "blueprint".to_string());
        assert!(parser.can_parse("", &fm));
    }

    #[test]
    fn can_parse_by_heading() {
        let parser = BlueprintParser;
        assert!(parser.can_parse("## Service Blueprint\nContent", &HashMap::new()));
    }

    #[test]
    fn parse_fixture() {
        let parser = BlueprintParser;
        let content = load_fixture();
        let mut fm = HashMap::new();
        fm.insert("title".to_string(), "Test Blueprint".to_string());
        fm.insert("type".to_string(), "blueprint".to_string());

        let result = parser.parse(&content, &fm).unwrap();
        if let ServiceDesignArtifact::Blueprint(bp) = result {
            assert_eq!(bp.meta.title, "Test Blueprint");
            assert_eq!(bp.meta.primary_persona, "Developer");
            assert_eq!(bp.meta.trigger, "Discovers rafters");
            assert_eq!(bp.meta.scope, "First run to production");
            assert_eq!(bp.steps.len(), 2);

            let step1 = &bp.steps[0];
            assert_eq!(step1.name, "Discovery");
            assert!(!step1.evidence.is_empty());
            assert!(!step1.customer_actions.is_empty());
            assert_eq!(step1.emotional_state.value(), 1);
            assert_eq!(step1.emotional_label, "Curiosity");
            assert!(!step1.pain_points.is_empty());

            let step2 = &bp.steps[1];
            assert_eq!(step2.name, "Installation");
            assert_eq!(step2.emotional_state.value(), -1);

            assert!(!bp.design_decisions.is_empty());
            assert!(!bp.open_questions.is_empty());
        } else {
            panic!("expected Blueprint variant");
        }
    }
}
