//! Extract documentation from a target project.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;
use veneer_adapters::{
    assess_coverage, not_yet_documented_placeholder, read_rafters_namespace,
    read_rafters_stylesheet, ComponentRegistry, CoverageReport, CoverageState, PlaceholderArtifact,
    NOT_YET_DOCUMENTED_STATUS,
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
/// Placeholders from a previous run whose item is no longer a gap are
/// removed first, so the pages on disk cannot drift from the reported
/// numbers.
fn run_coverage_phase(
    project: &Path,
    output: &Path,
    layout: Option<&str>,
) -> Result<CoverageOutcome> {
    let source = read_rafters_namespace(project)
        .with_context(|| format!("failed to read the rafters source in {}", project.display()))?;
    let items = ComponentRegistry::discover(project, &source)
        .with_context(|| format!("failed to discover components in {}", project.display()))?;
    // A project without a compiled stylesheet assesses against empty CSS:
    // every preview that needs styles is refused (FR-VEN-018) and lands in
    // not-yet-documented with that refusal as its reason.
    let full_css = read_rafters_stylesheet(project)
        .with_context(|| {
            format!(
                "failed to read the compiled stylesheet in {}",
                project.display()
            )
        })?
        .unwrap_or_default();
    let assessed = assess_coverage(items, &source, &full_css);

    let artifacts: Vec<PlaceholderArtifact> = assessed
        .iter()
        .filter_map(|entry| match &entry.state {
            CoverageState::NotYetDocumented { reason } => {
                Some(not_yet_documented_placeholder(&entry.item, reason, layout))
            }
            CoverageState::Documented => None,
        })
        .collect();

    let components_dir = output.join("components");
    remove_stale_placeholders(&components_dir, &artifacts)?;

    let mut placeholder_paths: Vec<PathBuf> = Vec::new();
    for artifact in &artifacts {
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

/// Remove placeholder pages a previous run left behind for items that are
/// no longer in the not-yet-documented set (fixed, or gone from the
/// discovered set), so a stale "not yet documented" claim never survives
/// a re-run. Only pages this phase owns are candidates: `.mdx` files
/// whose frontmatter carries the placeholder status marker. Real
/// documentation pages are never touched.
fn remove_stale_placeholders(components_dir: &Path, current: &[PlaceholderArtifact]) -> Result<()> {
    if !components_dir.is_dir() {
        return Ok(());
    }
    let entries = fs::read_dir(components_dir)
        .with_context(|| format!("failed to read {}", components_dir.display()))?;
    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read {}", components_dir.display()))?;
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file_name.ends_with(".mdx")
            || current
                .iter()
                .any(|artifact| artifact.file_name == file_name)
        {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        if !content.contains(NOT_YET_DOCUMENTED_STATUS) {
            continue;
        }
        fs::remove_file(&path)
            .with_context(|| format!("failed to remove stale placeholder {}", path.display()))?;
    }
    Ok(())
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

    /// The committed fixture with known partial coverage (shared with the
    /// veneer-adapters coverage tests, so the two layers cannot drift):
    /// Button renders, the hero-banner composite manifest renders, Broken
    /// does not parse, and ghost-widget is installed with no source file.
    fn partial_coverage_project() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../veneer-adapters/tests/fixtures/coverage/partial")
    }

    // AC: the CLI-facing summary reports exact numbers against the
    // discovered set of a fixture with known partial coverage.
    #[test]
    fn coverage_phase_reports_exact_numbers() {
        let project = partial_coverage_project();
        let output = tempfile::tempdir().expect("output dir");

        let outcome =
            run_coverage_phase(&project, output.path(), None).expect("coverage phase must run");
        let report = &outcome.report;
        assert_eq!(report.total, 4);
        assert_eq!(report.documented, ["Button", "hero-banner"]);
        assert_eq!(report.not_yet_documented, ["Broken", "ghost-widget"]);
    }

    // AC: every uncovered item gets an explicit "not yet documented"
    // artifact on disk; documented items get no placeholder.
    #[test]
    fn coverage_phase_emits_a_placeholder_per_gap_and_only_per_gap() {
        let project = partial_coverage_project();
        let output = tempfile::tempdir().expect("output dir");

        let outcome = run_coverage_phase(&project, output.path(), Some("../Docs.astro"))
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
        assert!(
            !components_dir.join("hero-banner.mdx").exists(),
            "a documented composite gets no placeholder"
        );
    }

    // A placeholder left by a previous run for an item that is no longer
    // a gap must be removed on re-run, while real documentation pages in
    // the same directory are never touched.
    #[test]
    fn coverage_phase_removes_stale_placeholders_but_not_real_pages() {
        let project = partial_coverage_project();
        let output = tempfile::tempdir().expect("output dir");
        let components_dir = output.path().join("components");
        fs::create_dir_all(&components_dir).expect("components dir");

        // A stale placeholder from a prior run: its item ("Vanished") is
        // not in the current discovered set, so its claim is now false.
        let stale = not_yet_documented_placeholder(
            &veneer_adapters::DiscoveredItem {
                name: "Vanished".to_string(),
                kind: veneer_adapters::DiscoveredKind::Component,
                source_path: PathBuf::from("components/vanished.tsx"),
                generated: false,
            },
            "it used to be broken",
            None,
        );
        let stale_path = components_dir.join(&stale.file_name);
        fs::write(&stale_path, &stale.content).expect("stale placeholder");

        // A real documentation page (no placeholder marker) must survive.
        let real_page = components_dir.join("button.mdx");
        fs::write(&real_page, "---\ntitle: Button\n---\n\n# Button\n").expect("real page");

        let outcome =
            run_coverage_phase(&project, output.path(), None).expect("coverage phase must run");

        assert!(
            !stale_path.exists(),
            "a stale placeholder must not survive a re-run"
        );
        assert!(
            real_page.exists(),
            "a real documentation page is never removed"
        );
        assert_eq!(
            outcome.placeholder_paths,
            [
                components_dir.join("broken.mdx"),
                components_dir.join("ghost-widget.mdx"),
            ]
        );
    }
}
