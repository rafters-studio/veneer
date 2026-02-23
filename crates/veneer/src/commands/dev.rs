//! Development server command.

use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use serde::Deserialize;
use veneer_server::{DevServer, DevServerConfig};

/// Config file structure for dev server (reads docs.toml).
#[derive(Debug, Deserialize, Default)]
struct ConfigFile {
    #[serde(default)]
    docs: DocsSection,
    #[serde(default)]
    components: ComponentsSection,
}

#[derive(Debug, Deserialize, Default)]
struct DocsSection {
    /// Path to a theme CSS file with --veneer-* variable overrides
    theme: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ComponentsSection {
    dir: Option<String>,
}

/// Load config from docs.toml if it exists.
fn load_config() -> ConfigFile {
    let config_path = PathBuf::from("docs.toml");
    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(config) = toml::from_str::<ConfigFile>(&content) {
                return config;
            }
        }
    }
    ConfigFile::default()
}

/// Resolve a components directory path, making relative paths absolute from CWD.
fn resolve_components_dir(path: PathBuf) -> Result<PathBuf> {
    let resolved = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .map_err(|e| anyhow::anyhow!("Failed to get current directory: {}", e))?
            .join(path)
    };

    if !resolved.exists() {
        anyhow::bail!(
            "Components directory does not exist: {}",
            resolved.display()
        );
    }

    Ok(resolved)
}

/// Run the dev server.
pub async fn run(port: u16, open: bool, cli_components_dir: Option<PathBuf>) -> Result<()> {
    tracing::info!("Starting development server on port {}", port);

    let file_config = load_config();

    // CLI flag takes precedence over docs.toml, which takes precedence over the default
    let components_dir = match cli_components_dir {
        Some(dir) => {
            tracing::info!("Using CLI components directory: {}", dir.display());
            resolve_components_dir(dir)?
        }
        None => match file_config.components.dir {
            Some(dir) => resolve_components_dir(PathBuf::from(dir))?,
            None => PathBuf::from("src/components"),
        },
    };

    let config = DevServerConfig {
        port,
        open,
        components_dir,
        theme: file_config.docs.theme,
        ..Default::default()
    };

    DevServer::new(config).start().await?;

    Ok(())
}
