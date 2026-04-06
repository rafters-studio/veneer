//! CLI help parser -- extracts structured command data from clap v4 --help output.

use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};

/// A parsed CLI command with its flags and subcommands.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParsedCommand {
    /// Command name (e.g., "reflect", "kanban create")
    pub name: String,
    /// Description from help text
    pub description: String,
    /// Flags/options
    pub flags: Vec<ParsedFlag>,
    /// Subcommands (recursive)
    pub subcommands: Vec<ParsedCommand>,
    /// Usage string
    pub usage: String,
    /// Aliases if any
    pub aliases: Vec<String>,
}

/// A parsed flag/option from --help output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParsedFlag {
    /// Long name (e.g., "--repo")
    pub long: Option<String>,
    /// Short name (e.g., "-v")
    pub short: Option<String>,
    /// Value placeholder (e.g., "REPO")
    pub value_name: Option<String>,
    /// Description
    pub description: String,
    /// Whether the flag is required
    pub required: bool,
    /// Default value if any
    pub default: Option<String>,
}

/// Errors from CLI help parsing.
#[derive(Debug, thiserror::Error)]
pub enum CliParseError {
    #[error("Binary not found or not executable: {0}")]
    BinaryNotFound(String),

    #[error("Failed to run --help: {0}")]
    ExecutionError(String),

    #[error("Non-zero exit from --help: {stderr}")]
    NonZeroExit { stderr: String },
}

/// Parse a binary by running --help recursively on all subcommands.
pub fn parse_cli_help(binary_path: &Path) -> Result<ParsedCommand, CliParseError> {
    let binary = binary_path
        .to_str()
        .ok_or_else(|| CliParseError::BinaryNotFound(binary_path.display().to_string()))?;

    let help_text = run_help(binary, &[])?;
    let name = binary_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(binary);
    let mut root = parse_help_text(name, &help_text);

    // Recursively parse subcommands
    let subcommand_names: Vec<String> = root.subcommands.iter().map(|s| s.name.clone()).collect();
    let mut parsed_subs = Vec::new();

    for sub_name in &subcommand_names {
        match run_help(binary, &[sub_name]) {
            Ok(sub_help) => {
                let mut sub_cmd = parse_help_text(sub_name, &sub_help);

                // Recurse one more level for nested subcommands (e.g., kanban create)
                let nested_names: Vec<String> =
                    sub_cmd.subcommands.iter().map(|s| s.name.clone()).collect();
                let mut parsed_nested = Vec::new();

                for nested_name in &nested_names {
                    match run_help(binary, &[sub_name, nested_name]) {
                        Ok(nested_help) => {
                            parsed_nested.push(parse_help_text(nested_name, &nested_help));
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to parse help for {} {}: {}",
                                sub_name,
                                nested_name,
                                e
                            );
                        }
                    }
                }

                sub_cmd.subcommands = parsed_nested;
                parsed_subs.push(sub_cmd);
            }
            Err(e) => {
                tracing::warn!("Failed to parse help for {}: {}", sub_name, e);
            }
        }
    }

    root.subcommands = parsed_subs;
    Ok(root)
}

/// Run `<binary> [args...] --help` and return stdout.
fn run_help(binary: &str, args: &[&str]) -> Result<String, CliParseError> {
    let mut cmd = Command::new(binary);
    for arg in args {
        cmd.arg(arg);
    }
    cmd.arg("--help");

    let output = cmd
        .output()
        .map_err(|e| CliParseError::BinaryNotFound(format!("{}: {}", binary, e)))?;

    // clap --help exits with 0
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(CliParseError::NonZeroExit { stderr });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Parse a single --help text into a ParsedCommand.
///
/// This is public for unit testing with synthetic help text.
pub fn parse_help_text(name: &str, help_text: &str) -> ParsedCommand {
    let mut cmd = ParsedCommand {
        name: name.to_string(),
        ..Default::default()
    };

    let mut section = Section::Preamble;
    let mut description_lines: Vec<String> = Vec::new();
    let mut current_flag_lines: Vec<String> = Vec::new();

    for line in help_text.lines() {
        // Detect section transitions
        if line == "Commands:" {
            flush_flag(&mut cmd.flags, &mut current_flag_lines);
            section = Section::Commands;
            continue;
        }
        if line == "Options:" {
            flush_flag(&mut cmd.flags, &mut current_flag_lines);
            section = Section::Options;
            continue;
        }
        if line.starts_with("Usage:") {
            flush_flag(&mut cmd.flags, &mut current_flag_lines);
            cmd.usage = line.trim_start_matches("Usage:").trim().to_string();
            section = Section::Usage;
            continue;
        }
        if line == "Arguments:" {
            flush_flag(&mut cmd.flags, &mut current_flag_lines);
            section = Section::Arguments;
            continue;
        }

        match section {
            Section::Preamble => {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    description_lines.push(trimmed.to_string());
                }
            }
            Section::Usage => {
                // Multi-line usage -- append if indented
                let trimmed = line.trim();
                if !trimmed.is_empty() && line.starts_with(' ') {
                    cmd.usage.push(' ');
                    cmd.usage.push_str(trimmed);
                }
            }
            Section::Commands => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Some((name, desc)) = parse_command_line(trimmed) {
                    // Skip "help" subcommand
                    if name != "help" {
                        cmd.subcommands.push(ParsedCommand {
                            name,
                            description: desc,
                            ..Default::default()
                        });
                    }
                }
            }
            Section::Options => {
                if line.trim().is_empty() {
                    flush_flag(&mut cmd.flags, &mut current_flag_lines);
                    continue;
                }
                // Continuation line (indented text following a flag)
                if !line.starts_with("  -")
                    && !line.starts_with("      --")
                    && !current_flag_lines.is_empty()
                {
                    current_flag_lines.push(line.to_string());
                } else {
                    flush_flag(&mut cmd.flags, &mut current_flag_lines);
                    current_flag_lines.push(line.to_string());
                }
            }
            Section::Arguments => {
                // Arguments section -- skip for now
            }
        }
    }

    // Flush remaining
    flush_flag(&mut cmd.flags, &mut current_flag_lines);

    cmd.description = description_lines.join(" ");
    cmd
}

#[derive(Debug, Clone, Copy)]
enum Section {
    Preamble,
    Usage,
    Commands,
    Options,
    Arguments,
}

/// Parse a command line like "  reflect   Store a reflection from a completed session"
fn parse_command_line(line: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = line.splitn(2, "  ").collect();
    if parts.len() == 2 {
        let name = parts[0].trim().to_string();
        let desc = parts[1].trim().to_string();
        if !name.is_empty() {
            return Some((name, desc));
        }
    }
    // Single word command with no description
    let name = line.trim();
    if !name.is_empty() && !name.contains(' ') {
        return Some((name.to_string(), String::new()));
    }
    None
}

/// Flush accumulated flag lines into a ParsedFlag and push to the vec.
fn flush_flag(flags: &mut Vec<ParsedFlag>, lines: &mut Vec<String>) {
    if lines.is_empty() {
        return;
    }

    let joined = lines.join(" ");
    lines.clear();

    if let Some(flag) = parse_flag_line(&joined) {
        flags.push(flag);
    }
}

/// Parse a flag line like "  -v, --verbose      Show informational messages"
/// or "      --repo <REPO>  Repository name(s), comma-separated"
fn parse_flag_line(line: &str) -> Option<ParsedFlag> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut flag = ParsedFlag::default();

    // Use regex to parse the flag pattern
    let re = regex::Regex::new(
        r"^(?:(-\w),\s+)?(--[\w-]+)(?:\s+<([^>]+)>)?\s{2,}(.+?)(?:\s+\[default:\s+([^\]]+)\])?$",
    )
    .ok()?;

    if let Some(caps) = re.captures(trimmed) {
        flag.short = caps.get(1).map(|m| m.as_str().to_string());
        flag.long = caps.get(2).map(|m| m.as_str().to_string());
        flag.value_name = caps.get(3).map(|m| m.as_str().to_string());
        flag.description = caps
            .get(4)
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();
        flag.default = caps.get(5).map(|m| m.as_str().to_string());

        return Some(flag);
    }

    // Try simpler pattern: just short flag
    let re_short =
        regex::Regex::new(r"^(-\w),\s+(--[\w-]+)\s{2,}(.+?)(?:\s+\[default:\s+([^\]]+)\])?$")
            .ok()?;

    if let Some(caps) = re_short.captures(trimmed) {
        flag.short = caps.get(1).map(|m| m.as_str().to_string());
        flag.long = caps.get(2).map(|m| m.as_str().to_string());
        flag.description = caps
            .get(3)
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();
        flag.default = caps.get(4).map(|m| m.as_str().to_string());
        return Some(flag);
    }

    // Try pattern without short: "      --flag <VALUE>  description"
    let re_long = regex::Regex::new(
        r"^(--[\w-]+)(?:\s+<([^>]+)>)?\s{2,}(.+?)(?:\s+\[default:\s+([^\]]+)\])?$",
    )
    .ok()?;

    if let Some(caps) = re_long.captures(trimmed) {
        flag.long = caps.get(1).map(|m| m.as_str().to_string());
        flag.value_name = caps.get(2).map(|m| m.as_str().to_string());
        flag.description = caps
            .get(3)
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();
        flag.default = caps.get(4).map(|m| m.as_str().to_string());
        return Some(flag);
    }

    None
}

/// Determine if a flag is required by checking the usage string.
pub fn mark_required_flags(cmd: &mut ParsedCommand) {
    let usage = cmd.usage.clone();
    for flag in &mut cmd.flags {
        if let Some(ref long) = flag.long {
            // Required flags appear without brackets in the usage string
            // e.g., "--repo <REPO>" is required, "[--verbose]" is optional
            let flag_name = long.trim_start_matches('-');
            let required_pattern = format!("--{}", flag_name);
            let optional_pattern = format!("[--{}", flag_name);

            if usage.contains(&required_pattern) && !usage.contains(&optional_pattern) {
                flag.required = true;
            }
        }
    }

    // Recurse into subcommands
    for sub in &mut cmd.subcommands {
        mark_required_flags(sub);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_help_output() {
        let help_text = "\
Store a reflection from a completed session

Usage: legion reflect [OPTIONS] --repo <REPO>

Options:
      --repo <REPO>              Repository name
  -v, --verbose                  Show informational messages
  -h, --help                     Print help";

        let mut cmd = parse_help_text("reflect", help_text);
        mark_required_flags(&mut cmd);

        assert_eq!(cmd.name, "reflect");
        assert_eq!(
            cmd.description,
            "Store a reflection from a completed session"
        );
        assert_eq!(cmd.usage, "legion reflect [OPTIONS] --repo <REPO>");
        assert_eq!(cmd.flags.len(), 3);

        // --repo should be required (appears without brackets in usage)
        let repo = cmd
            .flags
            .iter()
            .find(|f| f.long.as_deref() == Some("--repo"))
            .unwrap();
        assert!(repo.required);
        assert_eq!(repo.value_name.as_deref(), Some("REPO"));

        // --verbose should be optional
        let verbose = cmd
            .flags
            .iter()
            .find(|f| f.long.as_deref() == Some("--verbose"))
            .unwrap();
        assert!(!verbose.required);
        assert_eq!(verbose.short.as_deref(), Some("-v"));
    }

    #[test]
    fn parses_subcommands() {
        let help_text = "\
Manage the kanban board

Usage: legion kanban [OPTIONS] <COMMAND>

Commands:
  create      Create a new card on the kanban board
  list        List cards for a repo
  help        Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose  Show informational messages on stderr (quiet by default)
  -h, --help     Print help";

        let cmd = parse_help_text("kanban", help_text);
        assert_eq!(cmd.subcommands.len(), 2); // create, list (skip help)
        assert_eq!(cmd.subcommands[0].name, "create");
        assert_eq!(
            cmd.subcommands[0].description,
            "Create a new card on the kanban board"
        );
        assert_eq!(cmd.subcommands[1].name, "list");
    }

    #[test]
    fn parses_flag_with_default() {
        let help_text = "\
Start development server

Usage: veneer dev [OPTIONS]

Options:
  -p, --port <PORT>  Port to listen on [default: 7777]
  -h, --help         Print help";

        let cmd = parse_help_text("dev", help_text);
        let port = cmd
            .flags
            .iter()
            .find(|f| f.long.as_deref() == Some("--port"))
            .unwrap();
        assert_eq!(port.default.as_deref(), Some("7777"));
        assert_eq!(port.short.as_deref(), Some("-p"));
        assert_eq!(port.value_name.as_deref(), Some("PORT"));
    }

    #[test]
    fn parses_flag_without_short() {
        let help_text = "\
Build

Usage: veneer build [OPTIONS]

Options:
      --no-minify  Skip minification
  -h, --help       Print help";

        let cmd = parse_help_text("build", help_text);
        let no_minify = cmd
            .flags
            .iter()
            .find(|f| f.long.as_deref() == Some("--no-minify"))
            .unwrap();
        assert!(no_minify.short.is_none());
        assert_eq!(no_minify.description, "Skip minification");
    }

    #[test]
    fn parses_root_command_with_many_subcommands() {
        let help_text = "\
Agent specialization through deliberate practice

Usage: legion [OPTIONS] <COMMAND>

Commands:
  reflect   Store a reflection from a completed session
  recall    Recall relevant reflections for the current context
  consult   Search reflections across all repos for cross-agent consultation
  post      Post a message to the shared bullpen for other agents
  boost     Mark a reflection as useful after recalling and applying it
  signal    Send a structured signal to another agent
  bullpen   Read the bullpen or check for unread posts
  kanban    Manage the kanban board
  watch     Watch for signals and auto-wake sleeping agents
  serve     Start the web dashboard
  health    Show current system health and recent trend
  help      Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose  Show informational messages on stderr (quiet by default)
  -h, --help     Print help";

        let cmd = parse_help_text("legion", help_text);
        assert_eq!(
            cmd.description,
            "Agent specialization through deliberate practice"
        );
        // 11 subcommands (help excluded)
        assert_eq!(cmd.subcommands.len(), 11);
        assert!(cmd.subcommands.iter().all(|s| s.name != "help"));
    }

    #[test]
    fn required_detection_from_usage() {
        let help_text = "\
Send signal

Usage: legion signal [OPTIONS] --to <TO> --verb <VERB>

Options:
      --to <TO>        Recipient agent name
      --verb <VERB>    Signal verb
      --status <STATUS>  Signal status
  -h, --help           Print help";

        let mut cmd = parse_help_text("signal", help_text);
        mark_required_flags(&mut cmd);

        let to = cmd
            .flags
            .iter()
            .find(|f| f.long.as_deref() == Some("--to"))
            .unwrap();
        assert!(to.required);

        let verb = cmd
            .flags
            .iter()
            .find(|f| f.long.as_deref() == Some("--verb"))
            .unwrap();
        assert!(verb.required);

        let status = cmd
            .flags
            .iter()
            .find(|f| f.long.as_deref() == Some("--status"))
            .unwrap();
        assert!(!status.required);
    }

    #[test]
    fn empty_description_handled() {
        let help_text = "\
Usage: tool cmd

Options:
  -h, --help  Print help";

        let cmd = parse_help_text("cmd", help_text);
        assert_eq!(cmd.description, "");
    }
}
