//! Watch mode: re-run the substrate phase whenever the project's inputs
//! change. The counterpart to one-shot `extract` for local development -- edit
//! a component and its `.rafters/veneer/` docs regenerate.
//!
//! Two invariants keep the loop honest:
//! - It never watches veneer's own output (`.rafters/veneer/`); a change there
//!   is filtered out, so writing the substrate can never re-trigger a rebuild
//!   and spin the watcher forever.
//! - A burst of change events (an editor save can fire several) is coalesced
//!   into a single rebuild after a quiet debounce window, rather than
//!   rebuilding once per raw event.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Args;
use notify::{RecursiveMode, Watcher};
use serde::Deserialize;

use crate::commands::extract::run_substrate_phase;

/// Default quiet period after the last change before a rebuild fires.
const DEFAULT_DEBOUNCE_MS: u64 = 300;

#[derive(Args)]
pub struct WatchArgs {
    /// Path to the target project to watch
    #[arg(short, long)]
    project: PathBuf,

    /// Quiet period in milliseconds to coalesce a burst of changes into one
    /// rebuild
    #[arg(long, default_value_t = DEFAULT_DEBOUNCE_MS)]
    debounce_ms: u64,
}

/// The config path fields that point at the project's watchable source.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct WatchPaths {
    components_path: Option<String>,
    composites_path: Option<String>,
    primitives_path: Option<String>,
    rules_path: Option<String>,
}

pub async fn run(args: WatchArgs) -> Result<()> {
    let project = args.project;
    let debounce = Duration::from_millis(args.debounce_ms);

    if !project.exists() {
        anyhow::bail!("Project path does not exist: {}", project.display());
    }

    // The output veneer writes; a change under here must never trigger a
    // rebuild, or the watcher loops on its own output.
    let output_dir = project.join(".rafters").join("veneer");

    // Build once up front so the substrate exists before the first edit.
    build_once(&project);

    let targets = watch_targets(&project);
    if targets.is_empty() {
        anyhow::bail!(
            "Nothing to watch under {} (no declared source paths or .rafters inputs found)",
            project.display()
        );
    }

    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |result| {
        // A send failure only means the receiver is gone (shutting down).
        let _ = tx.send(result);
    })
    .context("failed to create the file watcher")?;

    for target in &targets {
        watcher
            .watch(target, RecursiveMode::Recursive)
            .with_context(|| format!("failed to watch {}", target.display()))?;
        tracing::info!("Watching {}", target.display());
    }
    tracing::info!(
        "Watching {} target(s); edit a source file to rebuild. Ctrl-C to stop.",
        targets.len()
    );

    let loop_project = project.clone();
    tokio::task::spawn_blocking(move || {
        watch_loop(rx, &loop_project, &output_dir, debounce);
    })
    .await
    .context("watch loop terminated unexpectedly")?;

    // Hold the watcher alive until the loop ends; dropping it stops watching.
    drop(watcher);
    Ok(())
}

/// The paths to watch: the project's declared source directories (from the
/// rafters config) plus the rafters inputs the substrate reads (tokens +
/// compiled output + the config file). veneer's own output is never included.
/// Only paths that exist are returned.
fn watch_targets(project: &Path) -> Vec<PathBuf> {
    let mut targets = Vec::new();

    let config_path = project.join(".rafters").join("config.rafters.json");
    if let Ok(text) = std::fs::read_to_string(&config_path) {
        let paths: WatchPaths = serde_json::from_str(&text).unwrap_or_default();
        for relative in [
            paths.components_path,
            paths.composites_path,
            paths.primitives_path,
            paths.rules_path,
        ]
        .into_iter()
        .flatten()
        {
            let dir = project.join(relative);
            if dir.is_dir() {
                targets.push(dir);
            }
        }
        targets.push(config_path);
    }

    // The namespace and the compiled stylesheet the previews scope from.
    for rafters_input in ["tokens", "output"] {
        let dir = project.join(".rafters").join(rafters_input);
        if dir.is_dir() {
            targets.push(dir);
        }
    }

    targets
}

/// Block for the first event of a burst, drain the rest until a quiet window,
/// then rebuild once -- skipping bursts that only touched veneer's own output.
fn watch_loop(
    rx: mpsc::Receiver<notify::Result<notify::Event>>,
    project: &Path,
    output_dir: &Path,
    debounce: Duration,
) {
    while let Ok(first) = rx.recv() {
        let mut changed = HashSet::new();
        collect_relevant(first, output_dir, &mut changed);

        loop {
            match rx.recv_timeout(debounce) {
                Ok(event) => collect_relevant(event, output_dir, &mut changed),
                Err(mpsc::RecvTimeoutError::Timeout) => break,
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        }

        if changed.is_empty() {
            continue; // every path was veneer's own output.
        }

        tracing::info!("Change detected ({} path(s)); rebuilding", changed.len());
        build_once(project);
    }
}

/// Record the event's paths that are not under veneer's own output directory.
fn collect_relevant(
    event: notify::Result<notify::Event>,
    output_dir: &Path,
    changed: &mut HashSet<PathBuf>,
) {
    let Ok(event) = event else {
        return;
    };
    for path in event.paths {
        if !path.starts_with(output_dir) {
            changed.insert(path);
        }
    }
}

/// Run one substrate build, reporting the outcome. A failed rebuild is logged
/// and swallowed -- a transient bad edit must not kill a running watcher.
fn build_once(project: &Path) {
    match run_substrate_phase(project) {
        Ok(outcome) => tracing::info!(
            "Wrote {} docs / {} index lines to {}",
            outcome.docs_lines,
            outcome.index_lines,
            outcome.dir.display()
        ),
        Err(error) => tracing::error!("Rebuild failed: {error:#}"),
    }
}
