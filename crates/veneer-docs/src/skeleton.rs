//! Skeleton page generator -- creates editorial MDX templates with slot markers.

use std::fs;
use std::path::Path;

use crate::mdx_reference::GeneratedPage;

/// Page template type.
#[derive(Debug, Clone)]
pub enum PageTemplate {
    GettingStarted,
    Concept { topic: String },
    Architecture,
}

/// Errors from skeleton generation.
#[derive(Debug, thiserror::Error)]
pub enum SkeletonError {
    #[error("Failed to write skeleton page: {0}")]
    WriteError(#[from] std::io::Error),
}

/// Generate a skeleton MDX page.
pub fn generate_skeleton(template: &PageTemplate, project_name: &str) -> String {
    match template {
        PageTemplate::GettingStarted => format!(
            "\
---
title: Getting Started
description: Install and start using {project_name}.
order: 0
---

{{/* veneer:prerequisites */}}

## Install

{{/* veneer:install */}}

## First Use

{{/* veneer:first-use */}}

## Next Steps

{{/* veneer:next-steps */}}
"
        ),
        PageTemplate::Architecture => format!(
            "\
---
title: Architecture
description: How {project_name} is structured and how the pieces fit together.
order: 1
---

{{/* veneer:overview */}}

## Project Structure

{{/* veneer:project-structure */}}

## Data Flow

{{/* veneer:data-flow */}}

## Key Design Decisions

{{/* veneer:design-decisions */}}
"
        ),
        PageTemplate::Concept { topic } => {
            let title = capitalize(topic);
            format!(
                "\
---
title: {title}
description: How {topic} works in {project_name}.
order: 10
---

{{/* veneer:overview */}}

## How It Works

{{/* veneer:how-it-works */}}

## Key Concepts

{{/* veneer:key-concepts */}}

## Related

{{/* veneer:related */}}
"
            )
        }
    }
}

/// Generate default skeleton pages for a project.
///
/// Creates getting-started, architecture, and concept pages based on
/// detected command groups. Never overwrites existing files.
pub fn generate_default_skeletons(
    project_name: &str,
    command_groups: &[String],
    output_dir: &Path,
) -> Result<Vec<GeneratedPage>, SkeletonError> {
    let mut pages = Vec::new();

    // Getting started
    let gs_path = output_dir.join("getting-started.mdx");
    if write_if_new(
        &gs_path,
        &generate_skeleton(&PageTemplate::GettingStarted, project_name),
    )? {
        pages.push(GeneratedPage {
            path: gs_path,
            title: "Getting Started".to_string(),
            command_name: String::new(),
        });
    }

    // Architecture
    let arch_path = output_dir.join("architecture.mdx");
    if write_if_new(
        &arch_path,
        &generate_skeleton(&PageTemplate::Architecture, project_name),
    )? {
        pages.push(GeneratedPage {
            path: arch_path,
            title: "Architecture".to_string(),
            command_name: String::new(),
        });
    }

    // Concept pages from command groups
    let concepts_dir = output_dir.join("concepts");
    for group in command_groups {
        let concept_path = concepts_dir.join(format!("{}.mdx", group));
        let template = PageTemplate::Concept {
            topic: group.clone(),
        };
        if write_if_new(&concept_path, &generate_skeleton(&template, project_name))? {
            pages.push(GeneratedPage {
                path: concept_path,
                title: capitalize(group),
                command_name: String::new(),
            });
        }
    }

    Ok(pages)
}

/// Write content to path only if the file does not already exist.
/// Returns true if the file was written, false if skipped.
fn write_if_new(path: &Path, content: &str) -> Result<bool, SkeletonError> {
    if path.exists() {
        tracing::warn!("Skipping existing file: {}", path.display());
        return Ok(false);
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, content)?;
    Ok(true)
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn generates_getting_started_with_project_name() {
        let mdx = generate_skeleton(&PageTemplate::GettingStarted, "legion");
        assert!(mdx.contains("Install and start using legion"));
        assert!(mdx.contains("{/* veneer:install */}"));
        assert!(mdx.contains("{/* veneer:prerequisites */}"));
        assert!(mdx.contains("{/* veneer:first-use */}"));
        assert!(mdx.contains("{/* veneer:next-steps */}"));
    }

    #[test]
    fn generates_architecture_with_project_name() {
        let mdx = generate_skeleton(&PageTemplate::Architecture, "legion");
        assert!(mdx.contains("title: Architecture"));
        assert!(mdx.contains("How legion is structured"));
        assert!(mdx.contains("{/* veneer:project-structure */}"));
    }

    #[test]
    fn generates_concept_page_with_topic() {
        let mdx = generate_skeleton(
            &PageTemplate::Concept {
                topic: "reflections".into(),
            },
            "legion",
        );
        assert!(mdx.contains("title: Reflections"));
        assert!(mdx.contains("How reflections works in legion"));
        assert!(mdx.contains("{/* veneer:how-it-works */}"));
    }

    #[test]
    fn does_not_overwrite_existing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("getting-started.mdx");

        // Write initial content
        fs::write(&path, "existing content").unwrap();

        // Try to generate -- should skip
        let pages = generate_default_skeletons("test", &[], dir.path()).unwrap();

        // getting-started was skipped, architecture was created
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].title, "Architecture");

        // Original content preserved
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "existing content");
    }

    #[test]
    fn generates_concept_pages_from_groups() {
        let dir = tempdir().unwrap();
        let groups = vec!["memory".to_string(), "communication".to_string()];
        let pages = generate_default_skeletons("legion", &groups, dir.path()).unwrap();

        // getting-started + architecture + 2 concepts = 4
        assert_eq!(pages.len(), 4);
        assert!(dir.path().join("concepts/memory.mdx").exists());
        assert!(dir.path().join("concepts/communication.mdx").exists());
    }

    #[test]
    fn frontmatter_is_valid_yaml() {
        let mdx = generate_skeleton(&PageTemplate::GettingStarted, "legion");
        assert!(mdx.starts_with("---\n"));
        let end = mdx.find("\n---\n").unwrap();
        let yaml = &mdx[4..end];
        assert!(yaml.contains("title:"));
        assert!(yaml.contains("description:"));
        assert!(yaml.contains("order:"));
    }
}
