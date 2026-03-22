use regex::Regex;

use crate::error::ServiceDesignError;
use crate::model::{JourneyMap, JourneyMeta, JourneyPhase, ServiceDesignArtifact};
use crate::parser::table::extract_sections;
use crate::parser::table::extract_tables;
use crate::parser::{
    detect_by_type_or_heading, find_section_content, parse_emotional_score, ArtifactParser,
    Frontmatter,
};

pub struct JourneyMapParser;

impl ArtifactParser for JourneyMapParser {
    fn can_parse(&self, content: &str, frontmatter: &Frontmatter) -> bool {
        if detect_by_type_or_heading(content, frontmatter, "journey-map", "## Journey Phases") {
            return true;
        }
        // Also detect by ### Phase headings
        content.contains("### Phase")
    }

    fn parse(
        &self,
        content: &str,
        frontmatter: &Frontmatter,
    ) -> Result<ServiceDesignArtifact, ServiceDesignError> {
        let sections = extract_sections(content);

        let title = frontmatter.get("title").cloned().unwrap_or_default();

        let overview_content = find_section_content(&sections, "Overview").unwrap_or("");

        let meta = JourneyMeta {
            title,
            persona: extract_overview_field(overview_content, "Persona"),
            scenario: extract_overview_field(overview_content, "Scenario"),
            goal: extract_overview_field(overview_content, "Goal"),
        };

        let phases = parse_phases(content)?;

        Ok(ServiceDesignArtifact::JourneyMap(JourneyMap {
            meta,
            phases,
        }))
    }
}

fn extract_overview_field(content: &str, field: &str) -> String {
    crate::parser::extract_bold_field(content, field).unwrap_or_default()
}

fn parse_phases(content: &str) -> Result<Vec<JourneyPhase>, ServiceDesignError> {
    let phase_re =
        Regex::new(r"(?m)^### Phase \d+:\s*(.+)$").map_err(|e| ServiceDesignError::Parse {
            file: String::new(),
            message: format!("regex error: {e}"),
        })?;

    let phase_positions: Vec<(usize, String)> = phase_re
        .captures_iter(content)
        .filter_map(|cap| {
            let m = cap.get(0)?;
            Some((m.start(), cap[1].trim().to_string()))
        })
        .collect();

    let mut phases = Vec::new();

    for (i, (start, name)) in phase_positions.iter().enumerate() {
        let end = phase_positions
            .get(i + 1)
            .map(|(pos, _)| *pos)
            .unwrap_or(content.len());

        let section = &content[*start..end];
        let phase = parse_single_phase(section, name)?;
        phases.push(phase);
    }

    Ok(phases)
}

fn parse_single_phase(section: &str, name: &str) -> Result<JourneyPhase, ServiceDesignError> {
    let tables = extract_tables(section);

    let mut actions = Vec::new();
    let mut touchpoints = Vec::new();
    let mut emotions = String::new();
    let mut pain_points = Vec::new();
    let mut opportunities = Vec::new();
    let mut emotional_score =
        crate::model::EmotionalScore::new(0).map_err(|e| ServiceDesignError::Parse {
            file: String::new(),
            message: format!("default score: {e}"),
        })?;

    // Look for horizontal table with standard journey map columns
    for table in &tables {
        let header_lower: Vec<String> = table.headers.iter().map(|h| h.to_lowercase()).collect();

        for row in &table.rows {
            for (j, header) in header_lower.iter().enumerate() {
                if j >= row.len() {
                    continue;
                }
                let cell = &row[j];
                if cell.is_empty() {
                    continue;
                }

                if header.contains("action") {
                    actions.extend(split_cell_items(cell));
                } else if header.contains("touchpoint") {
                    touchpoints.extend(split_cell_items(cell));
                } else if header.contains("emotional") {
                    emotions = cell.clone();
                    if let Ok((score, _)) = parse_emotional_score(cell) {
                        emotional_score = score;
                    }
                } else if header.contains("pain") {
                    pain_points.extend(split_cell_items(cell));
                } else if header.contains("opportunit") {
                    opportunities.extend(split_cell_items(cell));
                }
            }
        }
    }

    Ok(JourneyPhase {
        name: name.to_string(),
        time_range: None,
        actions,
        thoughts: vec![],
        emotions,
        touchpoints,
        pain_points,
        opportunities,
        emotional_score,
    })
}

/// Split a table cell that may contain multiple items separated by commas,
/// semicolons, or line breaks.
fn split_cell_items(cell: &str) -> Vec<String> {
    cell.split([';', '\n'])
        .flat_map(|part| {
            if part.contains(',') && !part.contains('(') {
                part.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            } else {
                vec![part.trim().to_string()]
            }
        })
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn load_fixture() -> String {
        include_str!("test_fixtures/sample_journey.md").to_string()
    }

    #[test]
    fn can_parse_by_frontmatter() {
        let parser = JourneyMapParser;
        let mut fm = HashMap::new();
        fm.insert("type".to_string(), "journey-map".to_string());
        assert!(parser.can_parse("", &fm));
    }

    #[test]
    fn can_parse_by_heading() {
        let parser = JourneyMapParser;
        assert!(parser.can_parse("## Journey Phases\n", &HashMap::new()));
    }

    #[test]
    fn parse_fixture() {
        let parser = JourneyMapParser;
        let content = load_fixture();
        let mut fm = HashMap::new();
        fm.insert(
            "title".to_string(),
            "Developer Onboarding Journey".to_string(),
        );
        fm.insert("type".to_string(), "journey-map".to_string());

        let result = parser.parse(&content, &fm).unwrap();
        if let ServiceDesignArtifact::JourneyMap(jm) = result {
            assert_eq!(jm.meta.title, "Developer Onboarding Journey");
            assert!(!jm.meta.persona.is_empty());
            assert_eq!(jm.phases.len(), 2);

            let phase1 = &jm.phases[0];
            assert_eq!(phase1.name, "Discovery");
            assert!(!phase1.actions.is_empty());
            assert!(!phase1.touchpoints.is_empty());
            assert!(!phase1.emotions.is_empty());
        } else {
            panic!("expected JourneyMap variant");
        }
    }
}
