use std::collections::HashMap;

use crate::error::ServiceDesignError;
use crate::model::{
    PainPointMatrix, PainTheme, Probe, ScoringDimension, ScoringRubric, ServiceDesignArtifact,
};
use crate::parser::table::{extract_sections, extract_tables};
use crate::parser::{detect_by_type_or_heading, find_section_content, ArtifactParser, Frontmatter};

pub struct PainPointMatrixParser;

impl ArtifactParser for PainPointMatrixParser {
    fn can_parse(&self, content: &str, frontmatter: &Frontmatter) -> bool {
        detect_by_type_or_heading(content, frontmatter, "pain-matrix", "## Scoring Rubric")
            || detect_by_type_or_heading(
                content,
                frontmatter,
                "pain-matrix",
                "## 1. Scoring Rubric",
            )
    }

    fn parse(
        &self,
        content: &str,
        _frontmatter: &Frontmatter,
    ) -> Result<ServiceDesignArtifact, ServiceDesignError> {
        let sections = extract_sections(content);

        let rubric = parse_rubric(&sections);
        let themes = parse_themes(&sections);
        let ranked_priorities = parse_ranked_priorities(&sections);
        let disconfirmation_log = parse_disconfirmation_log(&sections);
        let probe_backlog = parse_probe_backlog(&sections);

        Ok(ServiceDesignArtifact::PainPointMatrix(PainPointMatrix {
            rubric,
            themes,
            ranked_priorities,
            disconfirmation_log,
            probe_backlog,
        }))
    }
}

fn parse_rubric(sections: &[crate::parser::table::Section]) -> ScoringRubric {
    let content = find_section_content(sections, "Scoring Rubric")
        .or_else(|| find_section_content(sections, "1. Scoring Rubric"))
        .unwrap_or("");

    let tables = extract_tables(content);
    let mut dimensions = Vec::new();

    for table in &tables {
        let name_idx =
            find_col(&table.headers, "dimension").or_else(|| find_col(&table.headers, "name"));
        let weight_idx = find_col(&table.headers, "weight");
        let scale_idx =
            find_col(&table.headers, "scale").or_else(|| find_col(&table.headers, "max"));

        for row in &table.rows {
            let name = name_idx
                .and_then(|i| row.get(i))
                .cloned()
                .unwrap_or_default();
            let weight: f32 = weight_idx
                .and_then(|i| row.get(i))
                .and_then(|s| s.parse().ok())
                .unwrap_or(1.0);
            let scale_max: u8 = scale_idx
                .and_then(|i| row.get(i))
                .and_then(|s| s.parse().ok())
                .unwrap_or(5);

            if !name.is_empty() {
                dimensions.push(ScoringDimension {
                    name,
                    weight,
                    scale_max,
                });
            }
        }
    }

    ScoringRubric { dimensions }
}

fn parse_themes(sections: &[crate::parser::table::Section]) -> Vec<PainTheme> {
    let content = find_section_content(sections, "Theme")
        .or_else(|| find_section_content(sections, "2. Theme"))
        .unwrap_or("");

    let tables = extract_tables(content);
    let mut themes = Vec::new();

    for table in &tables {
        let name_idx =
            find_col(&table.headers, "theme").or_else(|| find_col(&table.headers, "name"));
        let composite_idx =
            find_col(&table.headers, "composite").or_else(|| find_col(&table.headers, "score"));
        let evidence_idx = find_col(&table.headers, "evidence");
        let cost_idx = find_col(&table.headers, "cost");

        // Find score dimension columns (anything not name/composite/evidence/cost)
        let score_cols: Vec<(usize, String)> = table
            .headers
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                Some(*i) != name_idx
                    && Some(*i) != composite_idx
                    && Some(*i) != evidence_idx
                    && Some(*i) != cost_idx
            })
            .map(|(i, h)| (i, h.clone()))
            .collect();

        for row in &table.rows {
            let name = name_idx
                .and_then(|i| row.get(i))
                .cloned()
                .unwrap_or_default();
            let composite_score: f32 = composite_idx
                .and_then(|i| row.get(i))
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let evidence = evidence_idx
                .and_then(|i| row.get(i))
                .cloned()
                .unwrap_or_default();
            let monthly_cost = cost_idx.and_then(|i| row.get(i)).cloned();

            let mut scores = HashMap::new();
            for (col_idx, col_name) in &score_cols {
                if let Some(val) = row.get(*col_idx) {
                    if let Ok(score) = val.parse::<f32>() {
                        scores.insert(col_name.clone(), score);
                    }
                }
            }

            if !name.is_empty() {
                themes.push(PainTheme {
                    name,
                    scores,
                    composite_score,
                    evidence,
                    monthly_cost,
                });
            }
        }
    }

    themes
}

fn parse_ranked_priorities(sections: &[crate::parser::table::Section]) -> Vec<String> {
    let content = find_section_content(sections, "Ranked Priorities")
        .or_else(|| find_section_content(sections, "3. Ranked Priorities"))
        .unwrap_or("");

    let tables = extract_tables(content);
    let mut priorities = Vec::new();

    for table in &tables {
        let name_idx = find_col(&table.headers, "priority")
            .or_else(|| find_col(&table.headers, "theme"))
            .or_else(|| find_col(&table.headers, "name"));

        for row in &table.rows {
            if let Some(name) = name_idx.and_then(|i| row.get(i)) {
                if !name.is_empty() {
                    priorities.push(name.clone());
                }
            }
        }
    }

    // If no table, try numbered list
    if priorities.is_empty() {
        priorities = crate::parser::extract_numbered_list(content);
    }

    priorities
}

fn parse_disconfirmation_log(sections: &[crate::parser::table::Section]) -> Vec<String> {
    let content = find_section_content(sections, "Disconfirmation")
        .or_else(|| find_section_content(sections, "5. Disconfirmation"))
        .unwrap_or("");

    let tables = extract_tables(content);
    let mut entries = Vec::new();

    for table in &tables {
        // Take the first meaningful column content from each row
        for row in &table.rows {
            let entry = row
                .iter()
                .filter(|c| !c.is_empty())
                .map(|c| c.as_str())
                .collect::<Vec<_>>()
                .join(" - ");
            if !entry.is_empty() {
                entries.push(entry);
            }
        }
    }

    entries
}

fn parse_probe_backlog(sections: &[crate::parser::table::Section]) -> Vec<Probe> {
    let content = find_section_content(sections, "Probe")
        .or_else(|| find_section_content(sections, "6. Probe"))
        .or_else(|| find_section_content(sections, "Experiment Backlog"))
        .unwrap_or("");

    let tables = extract_tables(content);
    let mut probes = Vec::new();

    for table in &tables {
        let question_idx =
            find_col(&table.headers, "question").or_else(|| find_col(&table.headers, "hypothesis"));
        let method_idx = find_col(&table.headers, "method");
        let metric_idx =
            find_col(&table.headers, "metric").or_else(|| find_col(&table.headers, "success"));

        for row in &table.rows {
            let question = question_idx
                .and_then(|i| row.get(i))
                .cloned()
                .unwrap_or_default();
            let method = method_idx
                .and_then(|i| row.get(i))
                .cloned()
                .unwrap_or_default();
            let success_metric = metric_idx
                .and_then(|i| row.get(i))
                .cloned()
                .unwrap_or_default();

            if !question.is_empty() {
                probes.push(Probe {
                    question,
                    method,
                    success_metric,
                });
            }
        }
    }

    probes
}

fn find_col(headers: &[String], pattern: &str) -> Option<usize> {
    headers
        .iter()
        .position(|h| h.to_lowercase().contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_fixture() -> String {
        include_str!("test_fixtures/sample_pain_matrix.md").to_string()
    }

    #[test]
    fn can_parse_by_frontmatter() {
        let parser = PainPointMatrixParser;
        let mut fm = HashMap::new();
        fm.insert("type".to_string(), "pain-matrix".to_string());
        assert!(parser.can_parse("", &fm));
    }

    #[test]
    fn can_parse_by_heading() {
        let parser = PainPointMatrixParser;
        assert!(parser.can_parse("## Scoring Rubric\n", &HashMap::new()));
    }

    #[test]
    fn parse_fixture() {
        let parser = PainPointMatrixParser;
        let content = load_fixture();
        let mut fm = HashMap::new();
        fm.insert("type".to_string(), "pain-matrix".to_string());

        let result = parser.parse(&content, &fm).unwrap();
        if let ServiceDesignArtifact::PainPointMatrix(matrix) = result {
            assert!(!matrix.rubric.dimensions.is_empty());
            assert!(!matrix.themes.is_empty());

            let theme = &matrix.themes[0];
            assert!(!theme.name.is_empty());
            assert!(theme.composite_score > 0.0);

            assert!(!matrix.ranked_priorities.is_empty());
            assert!(!matrix.probe_backlog.is_empty());
        } else {
            panic!("expected PainPointMatrix variant");
        }
    }
}
