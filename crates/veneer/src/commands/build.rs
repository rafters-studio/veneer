//! Static site build command.

use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use serde::Deserialize;
use veneer_static::{BuildConfig, StaticBuilder};

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
    /// Paths to CSS stylesheets to include
    styles: Option<Vec<String>>,
    /// Path to a theme CSS file with --veneer-* variable overrides
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

/// Load configuration from docs.toml if it exists.
/// Returns an error if the config file exists but is malformed.
fn load_config() -> Result<ConfigFile> {
    let config_path = PathBuf::from("docs.toml");
    if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .map_err(|e| anyhow::anyhow!("Failed to read docs.toml: {}", e))?;
        let config: ConfigFile = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse docs.toml: {}", e))?;
        tracing::info!("Loaded config from docs.toml");
        return Ok(config);
    }
    Ok(ConfigFile::default())
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

    let has_tsx = resolved.is_dir()
        && std::fs::read_dir(&resolved)
            .map(|entries| {
                entries.filter_map(|e| e.ok()).any(|e| {
                    let p = e.path();
                    p.extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext == "tsx" || ext == "jsx")
                        .unwrap_or(false)
                        || p.is_dir()
                })
            })
            .unwrap_or(false);

    if !has_tsx {
        tracing::warn!(
            "Components directory contains no .tsx/.jsx files: {}",
            resolved.display()
        );
    }

    Ok(resolved)
}

/// Run the build command.
pub async fn run(
    output: Option<PathBuf>,
    minify: Option<bool>,
    cli_components_dir: Option<PathBuf>,
) -> Result<()> {
    tracing::info!("Building static site...");

    let file_config = load_config()?;

    // CLI flag takes precedence over docs.toml
    let components_dir = match cli_components_dir {
        Some(dir) => {
            tracing::info!("Using CLI components directory: {}", dir.display());
            Some(resolve_components_dir(dir)?)
        }
        None => file_config
            .components
            .dir
            .map(PathBuf::from)
            .map(resolve_components_dir)
            .transpose()?,
    };

    let config = BuildConfig {
        docs_dir: PathBuf::from(&file_config.docs.dir),
        output_dir: output.unwrap_or_else(|| PathBuf::from(&file_config.docs.output)),
        components_dir,
        minify: minify.unwrap_or(file_config.build.minify),
        base_url: file_config.docs.base_url,
        title: file_config.docs.title,
        styles: file_config.docs.styles.unwrap_or_default(),
        theme: file_config.docs.theme,
    };

    let result = StaticBuilder::new(config).build().await?;

    tracing::info!(
        "Built {} pages with {} components in {}ms",
        result.pages,
        result.components,
        result.duration_ms
    );

    tracing::info!("Output: {}", result.output_dir.display());

    Ok(())
}
