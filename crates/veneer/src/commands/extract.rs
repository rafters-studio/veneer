//! Extract documentation from a target project.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;
use veneer_adapters::{
    assess_coverage, not_yet_documented_placeholder, read_rafters_namespace, ComponentRegistry,
    CoverageReport, CoverageState,
};
use veneer_docs::{
    generate_default_skeletons, generate_reference_pages, generate_sidebar, mark_required_flags,
    parse_cli_help, write_sidebar_jsonl, EditorialPage,
};

#[derive(Args)]
pub struct ExtractArgs {
    /// Path to the target project to extract docs from
    #[arg(short, long)]
    project: PathBuf,

    /// Output directory for generated MDX files
    #[arg(short, long, default_value = "docs")]
    output: PathBuf,

    /// Path to the project binary for --help extraction
    #[arg(short, long)]
    binary: Option<PathBuf>,

    /// Layout path to inject into MDX frontmatter (e.g., "../../layouts/Docs.astro")
    #[arg(short, long)]
    layout: Option<String>,
}

pub async fn run(args: ExtractArgs) -> Result<()> {
    let project_name = args
        .project
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
        .to_string();

    if !args.project.exists() {
        anyhow::bail!("Project path does not exist: {}", args.project.display());
    }

    tracing::info!("Extracting docs from {}", args.project.display());

    // Phase 1: Parse CLI help if binary is provided
    let mut reference_pages = Vec::new();
    let mut command_groups: Vec<String> = Vec::new();

    if let Some(ref binary_path) = args.binary {
        if !binary_path.exists() {
            anyhow::bail!("Binary not found: {}", binary_path.display());
        }

        tracing::info!("Parsing CLI help from {}", binary_path.display());

        let mut root_cmd = parse_cli_help(binary_path)?;
        mark_required_flags(&mut root_cmd);

        // Extract command groups (subcommands that have their own subcommands)
        for sub in &root_cmd.subcommands {
            if !sub.subcommands.is_empty() {
                command_groups.push(sub.name.clone());
            }
        }

        // Phase 2: Generate reference MDX pages
        let ref_dir = args.output.clone();
        let pages = generate_reference_pages(&root_cmd, &ref_dir, args.layout.as_deref())?;
        tracing::info!("Generated {} reference pages", pages.len());
        reference_pages = pages;

        // Phase 3: Generate sidebar JSONL
        let editorial_pages: Vec<EditorialPage> = vec![
            EditorialPage {
                title: "Getting Started".to_string(),
                path: "/getting-started".to_string(),
                section: "getting-started".to_string(),
                order: 0,
            },
            EditorialPage {
                title: "Architecture".to_string(),
                path: "/architecture".to_string(),
                section: "concepts".to_string(),
                order: 1,
            },
        ];

        let sidebar = generate_sidebar(&root_cmd, &editorial_pages);
        let sidebar_path = args.output.join("sidebar.jsonl");
        write_sidebar_jsonl(&sidebar, &sidebar_path)?;
        tracing::info!("Generated sidebar at {}", sidebar_path.display());
    }

    // Phase 4: Generate skeleton editorial pages
    let skeletons = generate_default_skeletons(
        &project_name,
        &command_groups,
        &args.output,
        args.layout.as_deref(),
    )?;
    tracing::info!("Generated {} skeleton pages", skeletons.len());

    let total = reference_pages.len() + skeletons.len();
    tracing::info!(
        "Extraction complete: {} total pages in {}",
        total,
        args.output.display()
    );

    // Phase 5: component coverage (FR-VEN-009). Measured against the
    // discovered set; every uncovered item gets an explicit placeholder.
    let coverage = run_coverage_phase(&args.project, &args.output, args.layout.as_deref())?;
    tracing::info!(
        "Emitted {} not-yet-documented placeholder pages",
        coverage.placeholder_paths.len()
    );
    print_coverage_summary(&coverage.report);

    Ok(())
}

/// Outcome of the coverage phase: the report plus the placeholder pages
/// that were emitted for uncovered items.
struct CoverageOutcome {
    report: CoverageReport,
    placeholder_paths: Vec<PathBuf>,
}

/// Assess coverage against the discovered set and emit an explicit "not
/// yet documented" placeholder page under `<output>/components/` for
/// every uncovered item -- a real artifact, never a blank or a 404.
fn run_coverage_phase(
    project: &Path,
    output: &Path,
    layout: Option<&str>,
) -> Result<CoverageOutcome> {
    let source = read_rafters_namespace(project)
        .with_context(|| format!("failed to read the rafters source in {}", project.display()))?;
    let items = ComponentRegistry::discover(project, &source)
        .with_context(|| format!("failed to discover components in {}", project.display()))?;
    let assessed = assess_coverage(&items, &source);

    let components_dir = output.join("components");
    let mut placeholder_paths: Vec<PathBuf> = Vec::new();
    for entry in &assessed {
        let CoverageState::NotYetDocumented { reason } = &entry.state else {
            continue;
        };
        let artifact = not_yet_documented_placeholder(&entry.item, reason, layout);
        let path = components_dir.join(&artifact.file_name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(&path, &artifact.content)
            .with_context(|| format!("failed to write placeholder {}", path.display()))?;
        placeholder_paths.push(path);
    }

    Ok(CoverageOutcome {
        report: CoverageReport::from_assessed(&assessed),
        placeholder_paths,
    })
}

/// The queryable CLI coverage summary: exact counts against the
/// discovered set, then each not-yet-documented item by name.
fn print_coverage_summary(report: &CoverageReport) {
    println!(
        "Coverage: {} of {} discovered components documented, {} not yet documented",
        report.documented.len(),
        report.total,
        report.not_yet_documented.len()
    );
    for name in &report.not_yet_documented {
        println!("  not yet documented: {name}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A project with known partial coverage: Button renders, Broken does
    /// not parse, and ghost-widget is installed with no source file.
    fn partial_coverage_project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let components = dir.path().join("components");
        fs::create_dir_all(&components).expect("mkdir components");
        fs::write(
            components.join("button.tsx"),
            "\
const variantClasses = {
  default: 'bg-primary text-primary-foreground',
  secondary: 'bg-secondary text-secondary-foreground',
};

export function Button() {
  return <button />;
}
",
        )
        .expect("write button");
        fs::write(
            components.join("broken.tsx"),
            "const variantClasses = {\n  default: 'bg-primary',\n;\n\nexport function Broken( {\n",
        )
        .expect("write broken");

        let rafters = dir.path().join(".rafters");
        fs::create_dir_all(rafters.join("tokens")).expect("mkdir .rafters/tokens");
        fs::write(
            rafters.join("config.rafters.json"),
            "{\"version\":\"1.0.0\",\"componentsPath\":\"components\",\
             \"installed\":{\"components\":[\"ghost-widget\"],\"composites\":[]}}",
        )
        .expect("write config");
        fs::write(
            rafters.join("tokens/semantic.rafters.json"),
            "{\"namespace\":\"semantic\",\"tokens\":[{\"name\":\"primary\",\
             \"value\":\"oklch(0.6 0.2 25)\"}]}",
        )
        .expect("write tokens");
        dir
    }

    // AC: the CLI-facing summary reports exact numbers against the
    // discovered set of a fixture with known partial coverage.
    #[test]
    fn coverage_phase_reports_exact_numbers() {
        let project = partial_coverage_project();
        let output = tempfile::tempdir().expect("output dir");

        let outcome = run_coverage_phase(project.path(), output.path(), None)
            .expect("coverage phase must run");
        let report = &outcome.report;
        assert_eq!(report.total, 3);
        assert_eq!(report.documented, ["Button"]);
        assert_eq!(report.not_yet_documented, ["Broken", "ghost-widget"]);
    }

    // AC: every uncovered item gets an explicit "not yet documented"
    // artifact on disk; documented items get no placeholder.
    #[test]
    fn coverage_phase_emits_a_placeholder_per_gap_and_only_per_gap() {
        let project = partial_coverage_project();
        let output = tempfile::tempdir().expect("output dir");

        let outcome = run_coverage_phase(project.path(), output.path(), Some("../Docs.astro"))
            .expect("coverage phase must run");

        let components_dir = output.path().join("components");
        assert_eq!(
            outcome.placeholder_paths,
            [
                components_dir.join("broken.mdx"),
                components_dir.join("ghost-widget.mdx"),
            ]
        );
        for path in &outcome.placeholder_paths {
            let content = fs::read_to_string(path).expect("placeholder must exist");
            assert!(!content.trim().is_empty(), "never a blank page");
            assert!(content.contains("status: not-yet-documented"));
            assert!(content.contains("layout: ../Docs.astro"));
        }
        assert!(
            !components_dir.join("button.mdx").exists(),
            "a documented component gets no placeholder"
        );
    }
}
