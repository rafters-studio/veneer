//! Sidebar generation -- produces a sidebar.jsonl from parsed CLI commands and editorial pages.

use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::cli_parser::ParsedCommand;

/// A node in the sidebar tree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SidebarNode {
    /// Display title
    pub title: String,
    /// URL path (e.g., "/docs/getting-started/install")
    pub path: Option<String>,
    /// Icon identifier (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Child nodes for groups
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub children: Vec<SidebarNode>,
    /// Sort order within the parent group
    pub order: u32,
    /// Logical group name (e.g., "getting-started", "commands")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

/// An editorial (hand-written) page to include in the sidebar.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditorialPage {
    /// Display title
    pub title: String,
    /// URL path
    pub path: String,
    /// Section slug (e.g., "getting-started", "concepts")
    pub section: String,
    /// Sort order within the section
    pub order: u32,
}

/// Errors from sidebar generation and writing.
#[derive(Debug, thiserror::Error)]
pub enum SidebarError {
    #[error("Failed to write sidebar file: {0}")]
    WriteError(#[from] std::io::Error),
}

/// Generate sidebar nodes from parsed CLI commands and editorial pages.
///
/// Editorial pages come first, grouped by section. Commands follow, grouped
/// by category: commands with subcommands become their own group, flat commands
/// collect under a "Commands" group.
pub fn generate_sidebar(
    commands: &ParsedCommand,
    editorial_pages: &[EditorialPage],
) -> Vec<SidebarNode> {
    let mut nodes: Vec<SidebarNode> = Vec::new();
    let mut group_order: u32 = 0;

    // -- Editorial pages first, grouped by section --
    let mut sections: Vec<String> = Vec::new();
    for page in editorial_pages {
        if !sections.contains(&page.section) {
            sections.push(page.section.clone());
        }
    }

    for section in &sections {
        let mut children: Vec<EditorialPage> = editorial_pages
            .iter()
            .filter(|p| &p.section == section)
            .cloned()
            .collect();
        children.sort_by_key(|p| p.order);

        let child_nodes: Vec<SidebarNode> = children
            .iter()
            .map(|p| SidebarNode {
                title: p.title.clone(),
                path: Some(p.path.clone()),
                icon: None,
                children: Vec::new(),
                order: p.order,
                group: Some(section.clone()),
            })
            .collect();

        nodes.push(SidebarNode {
            title: section_to_title(section),
            path: None,
            icon: None,
            children: child_nodes,
            order: group_order,
            group: Some(section.clone()),
        });
        group_order += 1;
    }

    // -- Commands from parsed CLI output --
    // Split subcommands into two buckets:
    //   1. Commands that have their own subcommands -> become a group
    //   2. Flat commands (no subcommands) -> collect under "Commands"

    let mut flat_commands: Vec<SidebarNode> = Vec::new();

    for sub in &commands.subcommands {
        // Skip -h/--help flags -- we do this during node construction, not here.
        if sub.subcommands.is_empty() {
            flat_commands.push(command_to_node(sub, flat_commands.len() as u32));
        } else {
            // Command with subcommands becomes its own group
            let children: Vec<SidebarNode> = sub
                .subcommands
                .iter()
                .enumerate()
                .map(|(i, nested)| command_to_node(nested, i as u32))
                .collect();

            nodes.push(SidebarNode {
                title: capitalize(&sub.name),
                path: Some(format!("/docs/cli/{}", sub.name)),
                icon: None,
                children,
                order: group_order,
                group: Some("commands".to_string()),
            });
            group_order += 1;
        }
    }

    if !flat_commands.is_empty() {
        nodes.push(SidebarNode {
            title: "Commands".to_string(),
            path: None,
            icon: None,
            children: flat_commands,
            order: group_order,
            group: Some("commands".to_string()),
        });
    }

    nodes
}

/// Write sidebar nodes as JSONL (one JSON object per line).
pub fn write_sidebar_jsonl(nodes: &[SidebarNode], output: &Path) -> Result<(), SidebarError> {
    let mut file = std::fs::File::create(output)?;
    for node in nodes {
        let json = serde_json::to_string(node)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        writeln!(file, "{}", json)?;
    }
    Ok(())
}

/// Convert a ParsedCommand to a leaf SidebarNode.
fn command_to_node(cmd: &ParsedCommand, order: u32) -> SidebarNode {
    SidebarNode {
        title: capitalize(&cmd.name),
        path: Some(format!("/docs/cli/{}", cmd.name)),
        icon: None,
        children: Vec::new(),
        order,
        group: Some("commands".to_string()),
    }
}

/// Convert a section slug like "getting-started" to "Getting Started".
fn section_to_title(slug: &str) -> String {
    slug.split('-')
        .map(capitalize)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Capitalize the first character of a string.
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli_parser::ParsedCommand;

    fn make_root_with_subcommands() -> ParsedCommand {
        ParsedCommand {
            name: "mytool".to_string(),
            description: "My tool".to_string(),
            subcommands: vec![
                ParsedCommand {
                    name: "build".to_string(),
                    description: "Build the project".to_string(),
                    ..Default::default()
                },
                ParsedCommand {
                    name: "serve".to_string(),
                    description: "Start server".to_string(),
                    ..Default::default()
                },
                ParsedCommand {
                    name: "kanban".to_string(),
                    description: "Manage kanban board".to_string(),
                    subcommands: vec![
                        ParsedCommand {
                            name: "create".to_string(),
                            description: "Create a card".to_string(),
                            ..Default::default()
                        },
                        ParsedCommand {
                            name: "list".to_string(),
                            description: "List cards".to_string(),
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                },
            ],
            ..Default::default()
        }
    }

    fn make_editorial_pages() -> Vec<EditorialPage> {
        vec![
            EditorialPage {
                title: "Installation".to_string(),
                path: "/docs/getting-started/install".to_string(),
                section: "getting-started".to_string(),
                order: 0,
            },
            EditorialPage {
                title: "Quick Start".to_string(),
                path: "/docs/getting-started/quickstart".to_string(),
                section: "getting-started".to_string(),
                order: 1,
            },
            EditorialPage {
                title: "Architecture".to_string(),
                path: "/docs/concepts/architecture".to_string(),
                section: "concepts".to_string(),
                order: 0,
            },
        ]
    }

    #[test]
    fn generates_grouped_sidebar_from_commands() {
        let root = make_root_with_subcommands();
        let sidebar = generate_sidebar(&root, &[]);

        // kanban has subcommands -> its own group
        // build, serve are flat -> under "Commands" group
        assert_eq!(sidebar.len(), 2); // "Kanban" group + "Commands" group

        let kanban_group = sidebar.iter().find(|n| n.title == "Kanban").unwrap();
        assert_eq!(kanban_group.children.len(), 2);
        assert_eq!(kanban_group.children[0].title, "Create");
        assert_eq!(kanban_group.children[1].title, "List");

        let commands_group = sidebar.iter().find(|n| n.title == "Commands").unwrap();
        assert_eq!(commands_group.children.len(), 2);
        assert_eq!(commands_group.children[0].title, "Build");
        assert_eq!(commands_group.children[1].title, "Serve");
    }

    #[test]
    fn editorial_pages_come_first() {
        let root = make_root_with_subcommands();
        let pages = make_editorial_pages();
        let sidebar = generate_sidebar(&root, &pages);

        // Editorial sections come first
        assert_eq!(sidebar[0].title, "Getting Started");
        assert_eq!(sidebar[0].children.len(), 2);
        assert_eq!(sidebar[0].children[0].title, "Installation");
        assert_eq!(sidebar[0].children[1].title, "Quick Start");

        assert_eq!(sidebar[1].title, "Concepts");
        assert_eq!(sidebar[1].children.len(), 1);
        assert_eq!(sidebar[1].children[0].title, "Architecture");

        // Commands come after editorial
        assert!(sidebar[0].order < sidebar[2].order);
        assert!(sidebar[1].order < sidebar[2].order);
    }

    #[test]
    fn writes_valid_jsonl() {
        let root = make_root_with_subcommands();
        let pages = make_editorial_pages();
        let sidebar = generate_sidebar(&root, &pages);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sidebar.jsonl");
        write_sidebar_jsonl(&sidebar, &path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        // Each line should be valid JSON
        assert!(!lines.is_empty());
        for line in &lines {
            let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
            assert!(parsed.is_object());
        }
    }

    #[test]
    fn subcommands_become_nested_groups() {
        let root = ParsedCommand {
            name: "tool".to_string(),
            subcommands: vec![ParsedCommand {
                name: "db".to_string(),
                description: "Database operations".to_string(),
                subcommands: vec![
                    ParsedCommand {
                        name: "migrate".to_string(),
                        description: "Run migrations".to_string(),
                        ..Default::default()
                    },
                    ParsedCommand {
                        name: "seed".to_string(),
                        description: "Seed data".to_string(),
                        ..Default::default()
                    },
                    ParsedCommand {
                        name: "reset".to_string(),
                        description: "Reset database".to_string(),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }],
            ..Default::default()
        };

        let sidebar = generate_sidebar(&root, &[]);
        assert_eq!(sidebar.len(), 1);
        assert_eq!(sidebar[0].title, "Db");
        assert_eq!(sidebar[0].children.len(), 3);
        assert_eq!(sidebar[0].children[0].title, "Migrate");
        assert_eq!(sidebar[0].children[1].title, "Seed");
        assert_eq!(sidebar[0].children[2].title, "Reset");
    }

    #[test]
    fn empty_input_produces_empty_output() {
        let root = ParsedCommand::default();
        let sidebar = generate_sidebar(&root, &[]);
        assert!(sidebar.is_empty());
    }
}
