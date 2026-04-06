//! Extract documentation from a target project.

use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
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
        let pages = generate_reference_pages(&root_cmd, &ref_dir)?;
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
    let skeletons = generate_default_skeletons(&project_name, &command_groups, &args.output)?;
    tracing::info!("Generated {} skeleton pages", skeletons.len());

    let total = reference_pages.len() + skeletons.len();
    tracing::info!(
        "Extraction complete: {} total pages in {}",
        total,
        args.output.display()
    );

    Ok(())
}
