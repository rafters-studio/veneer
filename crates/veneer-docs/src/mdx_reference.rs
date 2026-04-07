//! MDX reference page generation from parsed CLI command data.

use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::cli_parser::{ParsedCommand, ParsedFlag};

/// A generated MDX reference page.
#[derive(Debug, Clone)]
pub struct GeneratedPage {
    /// Path to the generated MDX file.
    pub path: PathBuf,
    /// Page title (e.g., "kanban create").
    pub title: String,
    /// Raw command name (e.g., "create").
    pub command_name: String,
}

/// Errors from MDX generation.
#[derive(Debug, thiserror::Error)]
pub enum MdxGenError {
    #[error("Failed to write MDX file: {0}")]
    WriteError(String),

    #[error("IO error: {0}")]
    IoError(String),
}

impl From<std::io::Error> for MdxGenError {
    fn from(e: std::io::Error) -> Self {
        MdxGenError::IoError(e.to_string())
    }
}

/// Returns true if the flag is -h/--help and should be skipped from the table.
fn is_help_flag(flag: &ParsedFlag) -> bool {
    flag.long.as_deref() == Some("--help") || flag.short.as_deref() == Some("-h")
}

/// Filter flags to exclude -h/--help.
fn visible_flags(flags: &[ParsedFlag]) -> Vec<&ParsedFlag> {
    flags.iter().filter(|f| !is_help_flag(f)).collect()
}

/// Generate an MDX reference page for a single command.
///
/// If `parent_name` is provided, the title becomes "parent_name command_name".
pub fn generate_command_mdx(
    command: &ParsedCommand,
    parent_name: Option<&str>,
    layout: Option<&str>,
) -> String {
    let mut out = String::new();

    let title = match parent_name {
        Some(parent) => format!("{} {}", parent, command.name),
        None => command.name.clone(),
    };

    // Frontmatter
    writeln!(out, "---").unwrap();
    if let Some(layout_path) = layout {
        writeln!(out, "layout: {}", layout_path).unwrap();
    }
    writeln!(out, "title: \"{}\"", title).unwrap();
    writeln!(
        out,
        "description: \"{}\"",
        command.description.replace('"', "\\\"")
    )
    .unwrap();
    writeln!(out, "---").unwrap();
    writeln!(out).unwrap();

    // Overview editorial slot
    writeln!(out, "{{/* veneer:overview */}}").unwrap();
    writeln!(out).unwrap();

    // Usage section
    writeln!(out, "## Usage").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "```bash").unwrap();
    writeln!(out, "{}", command.usage).unwrap();
    writeln!(out, "```").unwrap();
    writeln!(out).unwrap();

    // Flags table (only if there are visible flags)
    let flags = visible_flags(&command.flags);
    if !flags.is_empty() {
        writeln!(out, "## Flags").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "| Flag | Short | Required | Default | Description |").unwrap();
        writeln!(out, "| --- | --- | --- | --- | --- |").unwrap();

        for flag in &flags {
            let long = flag
                .long
                .as_deref()
                .map(|l| format!("`{}`", l))
                .unwrap_or_else(|| "-".to_string());
            let short = flag
                .short
                .as_deref()
                .map(|s| format!("`{}`", s))
                .unwrap_or_else(|| "-".to_string());
            let required = if flag.required { "Yes" } else { "No" };
            let default = flag.default.as_deref().unwrap_or("-");
            let desc = &flag.description;

            writeln!(
                out,
                "| {} | {} | {} | {} | {} |",
                long, short, required, default, desc
            )
            .unwrap();
        }
        writeln!(out).unwrap();
    }

    // Remaining editorial slots
    writeln!(out, "{{/* veneer:when-to-use */}}").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "{{/* veneer:examples */}}").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "{{/* veneer:gotchas */}}").unwrap();

    out
}

/// Generate MDX reference pages for a command tree.
///
/// Creates one MDX file per command in `output_dir/reference/`.
/// Subcommands go in subdirectories: `reference/kanban/create.mdx`.
pub fn generate_reference_pages(
    root: &ParsedCommand,
    output_dir: &Path,
    layout: Option<&str>,
) -> Result<Vec<GeneratedPage>, MdxGenError> {
    let reference_dir = output_dir.join("reference");
    let mut pages = Vec::new();

    // Generate page for root command
    let root_path = reference_dir.join(format!("{}.mdx", root.name));
    let root_mdx = generate_command_mdx(root, None, layout);
    write_mdx_file(&root_path, &root_mdx)?;
    pages.push(GeneratedPage {
        path: root_path,
        title: root.name.clone(),
        command_name: root.name.clone(),
    });

    // Generate pages for subcommands
    for sub in &root.subcommands {
        let sub_dir = reference_dir.join(&root.name);
        let sub_path = sub_dir.join(format!("{}.mdx", sub.name));
        let sub_mdx = generate_command_mdx(sub, Some(&root.name), layout);
        write_mdx_file(&sub_path, &sub_mdx)?;
        pages.push(GeneratedPage {
            path: sub_path,
            title: format!("{} {}", root.name, sub.name),
            command_name: sub.name.clone(),
        });

        // Nested subcommands (e.g., kanban create)
        for nested in &sub.subcommands {
            let nested_dir = sub_dir.join(&sub.name);
            let nested_path = nested_dir.join(format!("{}.mdx", nested.name));
            let parent_title = format!("{} {}", root.name, sub.name);
            let nested_mdx = generate_command_mdx(nested, Some(&parent_title), layout);
            write_mdx_file(&nested_path, &nested_mdx)?;
            pages.push(GeneratedPage {
                path: nested_path,
                title: format!("{} {} {}", root.name, sub.name, nested.name),
                command_name: nested.name.clone(),
            });
        }
    }

    Ok(pages)
}

/// Write MDX content to a file, creating parent directories as needed.
fn write_mdx_file(path: &Path, content: &str) -> Result<(), MdxGenError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            MdxGenError::WriteError(format!(
                "Failed to create directory {}: {}",
                parent.display(),
                e
            ))
        })?;
    }
    std::fs::write(path, content)
        .map_err(|e| MdxGenError::WriteError(format!("Failed to write {}: {}", path.display(), e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_command() -> ParsedCommand {
        ParsedCommand {
            name: "reflect".to_string(),
            description: "Store a reflection from a completed session".to_string(),
            usage: "legion reflect [OPTIONS] --repo <REPO>".to_string(),
            flags: vec![
                ParsedFlag {
                    long: Some("--repo".to_string()),
                    short: None,
                    value_name: Some("REPO".to_string()),
                    description: "Repository name".to_string(),
                    required: true,
                    default: None,
                },
                ParsedFlag {
                    long: Some("--verbose".to_string()),
                    short: Some("-v".to_string()),
                    value_name: None,
                    description: "Show informational messages".to_string(),
                    required: false,
                    default: None,
                },
                ParsedFlag {
                    long: Some("--help".to_string()),
                    short: Some("-h".to_string()),
                    value_name: None,
                    description: "Print help".to_string(),
                    required: false,
                    default: None,
                },
            ],
            subcommands: Vec::new(),
            aliases: Vec::new(),
        }
    }

    #[test]
    fn generates_valid_frontmatter() {
        let cmd = make_command();
        let mdx = generate_command_mdx(&cmd, None, None);

        assert!(mdx.starts_with("---\n"));
        assert!(mdx.contains("title: \"reflect\""));
        assert!(mdx.contains("description: \"Store a reflection from a completed session\""));
        // Frontmatter closes
        let second_dash = mdx[4..].find("---\n").unwrap();
        assert!(second_dash > 0);
    }

    #[test]
    fn generates_flag_table_with_correct_columns() {
        let cmd = make_command();
        let mdx = generate_command_mdx(&cmd, None, None);

        assert!(mdx.contains("## Flags"));
        assert!(mdx.contains("| Flag | Short | Required | Default | Description |"));
        assert!(mdx.contains("| `--repo` | - | Yes | - | Repository name |"));
        assert!(mdx.contains("| `--verbose` | `-v` | No | - | Show informational messages |"));
    }

    #[test]
    fn includes_editorial_slots() {
        let cmd = make_command();
        let mdx = generate_command_mdx(&cmd, None, None);

        assert!(mdx.contains("{/* veneer:overview */}"));
        assert!(mdx.contains("{/* veneer:when-to-use */}"));
        assert!(mdx.contains("{/* veneer:examples */}"));
        assert!(mdx.contains("{/* veneer:gotchas */}"));
    }

    #[test]
    fn nested_subcommand_uses_parent_name_in_title() {
        let cmd = ParsedCommand {
            name: "create".to_string(),
            description: "Create a new card".to_string(),
            usage: "legion kanban create [OPTIONS]".to_string(),
            flags: vec![ParsedFlag {
                long: Some("--help".to_string()),
                short: Some("-h".to_string()),
                value_name: None,
                description: "Print help".to_string(),
                required: false,
                default: None,
            }],
            subcommands: Vec::new(),
            aliases: Vec::new(),
        };

        let mdx = generate_command_mdx(&cmd, Some("kanban"), None);
        assert!(mdx.contains("title: \"kanban create\""));
    }

    #[test]
    fn skips_help_flag_from_table() {
        let cmd = make_command();
        let mdx = generate_command_mdx(&cmd, None, None);

        // Should not contain --help in the table rows
        assert!(!mdx.contains("| `--help`"));
        // But should still have the flags section (other flags exist)
        assert!(mdx.contains("## Flags"));
    }

    #[test]
    fn no_flags_section_when_only_help() {
        let cmd = ParsedCommand {
            name: "health".to_string(),
            description: "Show system health".to_string(),
            usage: "legion health".to_string(),
            flags: vec![ParsedFlag {
                long: Some("--help".to_string()),
                short: Some("-h".to_string()),
                value_name: None,
                description: "Print help".to_string(),
                required: false,
                default: None,
            }],
            subcommands: Vec::new(),
            aliases: Vec::new(),
        };

        let mdx = generate_command_mdx(&cmd, None, None);
        assert!(!mdx.contains("## Flags"));
    }

    #[test]
    fn generates_correct_file_paths_for_nested_commands() {
        let root = ParsedCommand {
            name: "legion".to_string(),
            description: "Agent tool".to_string(),
            usage: "legion <COMMAND>".to_string(),
            flags: vec![],
            subcommands: vec![ParsedCommand {
                name: "kanban".to_string(),
                description: "Manage kanban".to_string(),
                usage: "legion kanban <COMMAND>".to_string(),
                flags: vec![],
                subcommands: vec![ParsedCommand {
                    name: "create".to_string(),
                    description: "Create a card".to_string(),
                    usage: "legion kanban create [OPTIONS]".to_string(),
                    flags: vec![],
                    subcommands: Vec::new(),
                    aliases: Vec::new(),
                }],
                aliases: Vec::new(),
            }],
            aliases: Vec::new(),
        };

        let tmp = tempfile::tempdir().unwrap();
        let pages = generate_reference_pages(&root, tmp.path(), None).unwrap();

        assert_eq!(pages.len(), 3);

        // Root
        assert_eq!(pages[0].path, tmp.path().join("reference/legion.mdx"));
        assert_eq!(pages[0].title, "legion");

        // Subcommand
        assert_eq!(
            pages[1].path,
            tmp.path().join("reference/legion/kanban.mdx")
        );
        assert_eq!(pages[1].title, "legion kanban");

        // Nested subcommand
        assert_eq!(
            pages[2].path,
            tmp.path().join("reference/legion/kanban/create.mdx")
        );
        assert_eq!(pages[2].title, "legion kanban create");
    }
}
