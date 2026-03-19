use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("rafters directory not found at {0}")]
    NotFound(PathBuf),

    #[error("failed to read file {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to parse {path}: {source}")]
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("namespace file not found: {0}")]
    NamespaceNotFound(String),

    #[error("component not found: {0}")]
    ComponentNotFound(String),
}

impl RegistryError {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    pub fn parse(path: impl Into<PathBuf>, source: serde_json::Error) -> Self {
        Self::Parse {
            path: path.into(),
            source,
        }
    }
}
