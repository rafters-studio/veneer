//! The optional `veneer.json` at a consumer project root: the project's
//! input declarations to veneer (FR-VEN-021).
//!
//! Absence is the common case and yields working defaults -- a project
//! declares this file only when it has something to declare: reporter
//! artifact paths (consumed by FR-VEN-028/029 when they land) or an output
//! directory override for serialized pages. Versioned so the shape can
//! evolve without guessing; unknown fields are typed errors naming the
//! field, never silently ignored -- a typo'd key that silently no-ops is a
//! config that lies.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Schema version this build reads.
pub const VENEER_CONFIG_VERSION: u32 = 1;

/// Parsed `veneer.json`. Every field beyond `version` is optional; the
/// defaults are what an absent file means.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct VeneerConfig {
    /// Declared schema version; must equal [`VENEER_CONFIG_VERSION`].
    pub version: u32,
    /// Output directory override for serialized pages. `None` = the
    /// built-in default (`docs`). The substrate location is NOT
    /// configurable: `.rafters/veneer/` is a contract, not a preference.
    #[serde(default)]
    pub output_dir: Option<PathBuf>,
    /// Declared reporter artifact paths (FR-VEN-028/029 read these; until
    /// then they are carried as input facts).
    #[serde(default)]
    pub reporters: Option<Reporters>,
}

/// Reporter artifact paths the project declares, relative to its root.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Reporters {
    /// Machine-readable test reporter outputs (FR-VEN-028).
    #[serde(default)]
    pub tests: Vec<PathBuf>,
    /// Accessibility reporter outputs (FR-VEN-029).
    #[serde(default)]
    pub accessibility: Vec<PathBuf>,
}

impl Default for VeneerConfig {
    fn default() -> Self {
        VeneerConfig {
            version: VENEER_CONFIG_VERSION,
            output_dir: None,
            reporters: None,
        }
    }
}

impl VeneerConfig {
    /// The effective pages output directory: the declared override or the
    /// built-in default.
    pub fn output_dir(&self) -> PathBuf {
        self.output_dir.clone().unwrap_or_else(|| "docs".into())
    }
}

/// Read `<project>/veneer.json`. Absence yields the defaults; presence is
/// parsed strictly, with errors naming the file and the offending field.
pub fn read_veneer_config(project_root: &Path) -> Result<VeneerConfig, String> {
    let path = project_root.join("veneer.json");
    if !path.is_file() {
        return Ok(VeneerConfig::default());
    }
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let config: VeneerConfig = serde_json::from_str(&text).map_err(|error| {
        // serde names the unknown/invalid field and position in its message.
        format!("invalid {}: {error}", path.display())
    })?;
    if config.version != VENEER_CONFIG_VERSION {
        return Err(format!(
            "invalid {}: version {} is not supported (this veneer reads version {})",
            path.display(),
            config.version,
            VENEER_CONFIG_VERSION
        ));
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project_with(json: Option<&str>) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        if let Some(text) = json {
            fs::write(dir.path().join("veneer.json"), text).expect("write");
        }
        dir
    }

    #[test]
    fn absence_yields_working_defaults() {
        let project = project_with(None);
        let config = read_veneer_config(project.path()).expect("defaults");
        assert_eq!(config, VeneerConfig::default());
        assert_eq!(config.output_dir(), PathBuf::from("docs"));
    }

    #[test]
    fn declared_fields_parse() {
        let project = project_with(Some(
            r#"{"version":1,"outputDir":"site/docs","reporters":{"tests":["reports/vitest.json"]}}"#,
        ));
        let config = read_veneer_config(project.path()).expect("parses");
        assert_eq!(config.output_dir(), PathBuf::from("site/docs"));
        let reporters = config.reporters.expect("reporters");
        assert_eq!(reporters.tests, vec![PathBuf::from("reports/vitest.json")]);
        assert!(reporters.accessibility.is_empty());
    }

    #[test]
    fn an_unknown_field_is_a_typed_error_naming_it() {
        let project = project_with(Some(r#"{"version":1,"outputDirr":"docs"}"#));
        let error = read_veneer_config(project.path()).expect_err("unknown field refused");
        assert!(
            error.contains("outputDirr"),
            "error names the field: {error}"
        );
        assert!(
            error.contains("veneer.json"),
            "error names the file: {error}"
        );
    }

    #[test]
    fn an_unsupported_version_is_a_named_error() {
        let project = project_with(Some(r#"{"version":9}"#));
        let error = read_veneer_config(project.path()).expect_err("version refused");
        assert!(error.contains("version 9"), "{error}");
    }

    #[test]
    fn malformed_json_error_names_the_file() {
        let project = project_with(Some("{not json"));
        let error = read_veneer_config(project.path()).expect_err("malformed refused");
        assert!(error.contains("veneer.json"), "{error}");
    }
}
