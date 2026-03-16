//! Watch command - rebuild on filesystem changes without starting an HTTP server.

use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use notify::{RecursiveMode, Watcher};
use serde::Deserialize;
use veneer_static::{BuildConfig, StaticBuilder};

const PID_DIR: &str = ".veneer";
const PID_FILE: &str = ".veneer/watch.pid";

/// Arguments for the watch command.
#[derive(clap::Args)]
pub struct WatchArgs {
    /// Stop a running watcher
    #[arg(long)]
    pub off: bool,

    /// Output directory (overrides config)
    #[arg(long)]
    pub output: Option<PathBuf>,

    /// Components directory (overrides config)
    #[arg(long)]
    pub components_dir: Option<PathBuf>,

    /// Path to config file
    #[arg(long, default_value = "docs.toml")]
    pub config: PathBuf,
}

/// Configuration file structure (docs.toml).
#[derive(Debug, Deserialize, Default)]
struct ConfigFile {
    #[serde(default)]
    docs: DocsConfig,
    #[serde(default)]
    components: ComponentsConfig,
    #[serde(default)]
    build: BuildSettings,
}

#[derive(Debug, Deserialize, Default)]
struct DocsConfig {
    #[serde(default = "default_docs_dir")]
    dir: String,
    #[serde(default = "default_output")]
    output: String,
    #[serde(default = "default_title")]
    title: String,
    #[serde(default = "default_base_url")]
    base_url: String,
    styles: Option<Vec<String>>,
    theme: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ComponentsConfig {
    dir: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct BuildSettings {
    #[serde(default = "default_minify")]
    minify: bool,
}

fn default_docs_dir() -> String {
    "docs".to_string()
}
fn default_output() -> String {
    "dist".to_string()
}
fn default_title() -> String {
    "Documentation".to_string()
}
fn default_base_url() -> String {
    "/".to_string()
}
fn default_minify() -> bool {
    true
}

/// Load configuration from the given config file path.
fn load_config(config_path: &PathBuf) -> Result<ConfigFile> {
    if config_path.exists() {
        let content = fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read {}", config_path.display()))?;
        let config: ConfigFile = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", config_path.display()))?;
        tracing::info!("Loaded config from {}", config_path.display());
        return Ok(config);
    }
    Ok(ConfigFile::default())
}

/// Build a BuildConfig from the file config and CLI overrides.
fn make_build_config(
    file_config: &ConfigFile,
    output: &Option<PathBuf>,
    components_dir: &Option<PathBuf>,
) -> BuildConfig {
    BuildConfig {
        docs_dir: PathBuf::from(&file_config.docs.dir),
        output_dir: output
            .clone()
            .unwrap_or_else(|| PathBuf::from(&file_config.docs.output)),
        components_dir: components_dir
            .clone()
            .or_else(|| file_config.components.dir.as_ref().map(PathBuf::from)),
        minify: file_config.build.minify,
        base_url: file_config.docs.base_url.clone(),
        title: file_config.docs.title.clone(),
        styles: file_config.docs.styles.clone().unwrap_or_default(),
        theme: file_config.docs.theme.clone(),
    }
}

/// Run the watch command.
pub async fn run(args: WatchArgs) -> Result<()> {
    if args.off {
        return stop_watcher();
    }

    start_watcher(args).await
}

/// Stop a running watcher by reading its PID file and sending SIGTERM.
fn stop_watcher() -> Result<()> {
    let pid_path = PathBuf::from(PID_FILE);
    if !pid_path.exists() {
        tracing::info!("No watcher is running (no PID file found)");
        return Ok(());
    }

    let pid_str = fs::read_to_string(&pid_path).context("Failed to read PID file")?;
    let pid: i32 = pid_str.trim().parse().context("Invalid PID in watch.pid")?;

    tracing::info!("Stopping watcher (PID {})", pid);

    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        match kill(Pid::from_raw(pid), Signal::SIGTERM) {
            Ok(()) => {
                tracing::info!("Sent SIGTERM to watcher (PID {})", pid);
            }
            Err(nix::errno::Errno::ESRCH) => {
                tracing::info!("Watcher process {} not found (already stopped)", pid);
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Failed to send signal to PID {}: {}",
                    pid,
                    e
                ));
            }
        }
    }

    #[cfg(not(unix))]
    {
        tracing::warn!("Signal sending is only supported on Unix platforms");
    }

    // Clean up PID file
    if pid_path.exists() {
        fs::remove_file(&pid_path).context("Failed to remove PID file")?;
    }

    Ok(())
}

/// Write the current process PID to the PID file.
fn write_pid_file() -> Result<()> {
    fs::create_dir_all(PID_DIR).context("Failed to create .veneer directory")?;
    let pid = std::process::id();
    fs::write(PID_FILE, pid.to_string()).context("Failed to write PID file")?;
    tracing::debug!("Wrote PID {} to {}", pid, PID_FILE);
    Ok(())
}

/// Remove the PID file if it exists.
fn remove_pid_file() {
    let pid_path = PathBuf::from(PID_FILE);
    if pid_path.exists() {
        if let Err(e) = fs::remove_file(&pid_path) {
            tracing::warn!("Failed to remove PID file: {}", e);
        }
    }
}

/// Perform a build and return the result, logging any errors without panicking.
async fn do_build(build_config: &BuildConfig) -> bool {
    match StaticBuilder::new(build_config.clone()).build().await {
        Ok(result) => {
            tracing::info!(
                "Built {} pages with {} components in {}ms",
                result.pages,
                result.components,
                result.duration_ms
            );
            true
        }
        Err(e) => {
            tracing::error!("Build failed: {}", e);
            false
        }
    }
}

/// Classify a file path for logging purposes.
fn describe_change(path: &std::path::Path) -> &'static str {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        "mdx" | "md" => "doc",
        "tsx" | "jsx" | "ts" | "js" => "component",
        "css" => "stylesheet",
        "toml" => "config",
        _ => "file",
    }
}

/// Start watching for file changes and rebuild on each change.
async fn start_watcher(args: WatchArgs) -> Result<()> {
    let file_config = load_config(&args.config)?;
    let build_config = make_build_config(&file_config, &args.output, &args.components_dir);

    // Write PID file
    write_pid_file()?;

    // Initial build
    tracing::info!("Running initial build...");
    do_build(&build_config).await;

    // Determine paths to watch
    let mut watch_paths: Vec<PathBuf> = vec![build_config.docs_dir.clone()];
    if let Some(ref comp_dir) = build_config.components_dir {
        watch_paths.push(comp_dir.clone());
    }
    let rafters_output = PathBuf::from(".rafters/output");
    if rafters_output.exists() {
        watch_paths.push(rafters_output);
    }
    // Also watch the config file itself
    if args.config.exists() {
        watch_paths.push(args.config.clone());
    }

    tracing::info!(
        "Watching for changes in: {}",
        watch_paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Set up the file watcher with a sync channel
    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, _>| {
        if let Ok(event) = res {
            let _ = tx.send(event);
        }
    })
    .map_err(|e| anyhow::anyhow!("Failed to create file watcher: {}", e))?;

    for path in &watch_paths {
        if path.exists() {
            watcher
                .watch(path, RecursiveMode::Recursive)
                .map_err(|e| anyhow::anyhow!("Failed to watch {}: {}", path.display(), e))?;
        }
    }

    // Set up Ctrl+C handler
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let shutdown_tx = std::sync::Mutex::new(Some(shutdown_tx));

    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            if let Ok(mut guard) = shutdown_tx.lock() {
                if let Some(tx) = guard.take() {
                    let _ = tx.send(());
                }
            }
        }
    });

    // Main watch loop
    let debounce_duration = Duration::from_millis(100);
    let mut last_build_time = Instant::now();

    loop {
        // Check for shutdown signal
        if shutdown_rx.try_recv().is_ok() {
            tracing::info!("Received shutdown signal, cleaning up...");
            break;
        }

        // Check for file events (non-blocking with timeout)
        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(event) => {
                let now = Instant::now();
                if now.duration_since(last_build_time) < debounce_duration {
                    continue;
                }

                // Drain any queued events within debounce window
                while rx.recv_timeout(Duration::from_millis(50)).is_ok() {}

                // Log what changed
                for path in &event.paths {
                    let kind = describe_change(path);
                    tracing::info!("Change detected ({}) in {}", kind, path.display());
                }

                // Rebuild
                tracing::info!("Rebuilding...");
                do_build(&build_config).await;
                last_build_time = Instant::now();
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // No events, loop back to check shutdown
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                tracing::error!("File watcher disconnected");
                break;
            }
        }
    }

    // Clean up
    remove_pid_file();
    tracing::info!("Watcher stopped");

    // Keep watcher alive until here
    drop(watcher);

    Ok(())
}
