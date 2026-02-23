//! Development server command.

use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::Deserialize;
use veneer_server::{DevServer, DevServerConfig};

/// Minimal config file structure for dev server (reads docs.toml).
#[derive(Debug, Deserialize, Default)]
struct ConfigFile {
    #[serde(default)]
    docs: DocsSection,
}

#[derive(Debug, Deserialize, Default)]
struct DocsSection {
    /// Path to a theme CSS file with --veneer-* variable overrides
    theme: Option<String>,
}

/// Load the theme config from the given config file path if it exists.
fn load_theme_config(config_path: &Path) -> Option<String> {
    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(config_path) {
            if let Ok(config) = toml::from_str::<ConfigFile>(&content) {
                return config.docs.theme;
            }
        }
    }
    None
}

/// Run the dev server.
pub async fn run(config_path: &Path, port: u16, open: bool) -> Result<()> {
    tracing::info!("Starting development server on port {}", port);

    let theme = load_theme_config(config_path);

    let config = DevServerConfig {
        port,
        open,
        theme,
        ..Default::default()
    };

    DevServer::new(config).start().await?;

    Ok(())
}
