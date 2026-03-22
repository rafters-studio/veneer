use crate::error::ServiceDesignError;
use crate::model::{
    Actor, ActorType, Channel, EcosystemMap, FailureMode, ServiceDesignArtifact, ValueExchange,
};
use crate::parser::table::{extract_sections, extract_tables};
use crate::parser::{detect_by_type_or_heading, find_section_content, ArtifactParser, Frontmatter};

pub struct EcosystemMapParser;

impl ArtifactParser for EcosystemMapParser {
    fn can_parse(&self, content: &str, frontmatter: &Frontmatter) -> bool {
        detect_by_type_or_heading(content, frontmatter, "ecosystem", "## Actors")
    }

    fn parse(
        &self,
        content: &str,
        frontmatter: &Frontmatter,
    ) -> Result<ServiceDesignArtifact, ServiceDesignError> {
        let sections = extract_sections(content);

        let title = frontmatter.get("title").cloned().unwrap_or_default();

        let core_service = find_section_content(&sections, "Core Service")
            .map(|c| c.trim().to_string())
            .unwrap_or_default();

        let actors = parse_actors(content);
        let channels = parse_channels(content);
        let value_exchanges = parse_value_exchanges(content);
        let failure_modes = parse_failure_modes(content);

        Ok(ServiceDesignArtifact::EcosystemMap(EcosystemMap {
            title,
            core_service,
            actors,
            channels,
            value_exchanges,
            moments_of_truth: vec![],
            failure_modes,
        }))
    }
}

fn parse_actors(content: &str) -> Vec<Actor> {
    let sections = extract_sections(content);
    let mut actors = Vec::new();

    let type_map = [
        ("Primary", ActorType::Primary),
        ("Secondary", ActorType::Secondary),
        ("Tertiary", ActorType::Tertiary),
        ("Future", ActorType::Future),
    ];

    for (heading_prefix, actor_type) in &type_map {
        if let Some(section_content) = find_section_content(&sections, heading_prefix) {
            let tables = extract_tables(section_content);
            for table in &tables {
                let name_idx = find_column_index(&table.headers, "name")
                    .or_else(|| find_column_index(&table.headers, "actor"));
                let desc_idx = find_column_index(&table.headers, "description")
                    .or_else(|| find_column_index(&table.headers, "role"));

                for row in &table.rows {
                    let name = name_idx
                        .and_then(|i| row.get(i))
                        .cloned()
                        .unwrap_or_default();
                    let description = desc_idx
                        .and_then(|i| row.get(i))
                        .cloned()
                        .unwrap_or_default();

                    if !name.is_empty() {
                        actors.push(Actor {
                            name,
                            actor_type: actor_type.clone(),
                            description,
                        });
                    }
                }
            }
        }
    }

    actors
}

fn parse_channels(content: &str) -> Vec<Channel> {
    let sections = extract_sections(content);
    let mut channels = Vec::new();

    if let Some(section_content) = find_section_content(&sections, "Channels") {
        let tables = extract_tables(section_content);
        for table in &tables {
            let name_idx = find_column_index(&table.headers, "name")
                .or_else(|| find_column_index(&table.headers, "channel"));
            let type_idx = find_column_index(&table.headers, "type");
            let desc_idx = find_column_index(&table.headers, "description");

            for row in &table.rows {
                let name = name_idx
                    .and_then(|i| row.get(i))
                    .cloned()
                    .unwrap_or_default();
                let channel_type = type_idx
                    .and_then(|i| row.get(i))
                    .cloned()
                    .unwrap_or_default();
                let description = desc_idx
                    .and_then(|i| row.get(i))
                    .cloned()
                    .unwrap_or_default();

                if !name.is_empty() {
                    channels.push(Channel {
                        name,
                        channel_type,
                        description,
                    });
                }
            }
        }
    }

    channels
}

fn parse_value_exchanges(content: &str) -> Vec<ValueExchange> {
    let sections = extract_sections(content);
    let mut exchanges = Vec::new();

    if let Some(section_content) = find_section_content(&sections, "Value Exchange") {
        let tables = extract_tables(section_content);
        for table in &tables {
            let actor_idx = find_column_index(&table.headers, "actor");
            let gives_idx = find_column_index(&table.headers, "gives");
            let gets_idx = find_column_index(&table.headers, "gets")
                .or_else(|| find_column_index(&table.headers, "receives"));

            for row in &table.rows {
                let actor = actor_idx
                    .and_then(|i| row.get(i))
                    .cloned()
                    .unwrap_or_default();
                let gives = gives_idx
                    .and_then(|i| row.get(i))
                    .map(|s| split_items(s))
                    .unwrap_or_default();
                let gets = gets_idx
                    .and_then(|i| row.get(i))
                    .map(|s| split_items(s))
                    .unwrap_or_default();

                if !actor.is_empty() {
                    exchanges.push(ValueExchange { actor, gives, gets });
                }
            }
        }
    }

    exchanges
}

fn parse_failure_modes(content: &str) -> Vec<FailureMode> {
    let sections = extract_sections(content);
    let mut modes = Vec::new();

    if let Some(section_content) = find_section_content(&sections, "Failure Mode") {
        let tables = extract_tables(section_content);
        for table in &tables {
            let mode_idx = find_column_index(&table.headers, "mode")
                .or_else(|| find_column_index(&table.headers, "failure"));
            let impact_idx = find_column_index(&table.headers, "impact");
            let recovery_idx = find_column_index(&table.headers, "recovery");

            for row in &table.rows {
                let mode = mode_idx
                    .and_then(|i| row.get(i))
                    .cloned()
                    .unwrap_or_default();
                let impact = impact_idx
                    .and_then(|i| row.get(i))
                    .cloned()
                    .unwrap_or_default();
                let recovery = recovery_idx
                    .and_then(|i| row.get(i))
                    .cloned()
                    .unwrap_or_default();

                if !mode.is_empty() {
                    modes.push(FailureMode {
                        mode,
                        impact,
                        recovery,
                    });
                }
            }
        }
    }

    modes
}

fn find_column_index(headers: &[String], pattern: &str) -> Option<usize> {
    headers
        .iter()
        .position(|h| h.to_lowercase().contains(pattern))
}

fn split_items(text: &str) -> Vec<String> {
    text.split([',', ';'])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn load_fixture() -> String {
        include_str!("test_fixtures/sample_ecosystem.md").to_string()
    }

    #[test]
    fn can_parse_by_frontmatter() {
        let parser = EcosystemMapParser;
        let mut fm = HashMap::new();
        fm.insert("type".to_string(), "ecosystem".to_string());
        assert!(parser.can_parse("", &fm));
    }

    #[test]
    fn can_parse_by_heading() {
        let parser = EcosystemMapParser;
        assert!(parser.can_parse("## Actors\n", &HashMap::new()));
    }

    #[test]
    fn parse_fixture() {
        let parser = EcosystemMapParser;
        let content = load_fixture();
        let mut fm = HashMap::new();
        fm.insert("title".to_string(), "Rafters Ecosystem".to_string());
        fm.insert("type".to_string(), "ecosystem".to_string());

        let result = parser.parse(&content, &fm).unwrap();
        if let ServiceDesignArtifact::EcosystemMap(em) = result {
            assert_eq!(em.title, "Rafters Ecosystem");
            assert!(!em.core_service.is_empty());
            assert!(!em.actors.is_empty());

            // Check actor types
            let primary_count = em
                .actors
                .iter()
                .filter(|a| a.actor_type == ActorType::Primary)
                .count();
            assert!(primary_count > 0);

            assert!(!em.channels.is_empty());
            assert!(!em.failure_modes.is_empty());
        } else {
            panic!("expected EcosystemMap variant");
        }
    }
}
