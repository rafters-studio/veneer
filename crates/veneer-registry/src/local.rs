use std::path::{Path, PathBuf};

use crate::error::RegistryError;
use crate::model::{NamespaceTokenFile, RaftersConfig, RegistryIndex, RegistryItem};
use crate::reader::RegistryReader;

/// Reads rafters registry data from the local filesystem.
pub struct LocalRegistryReader {
    /// Path to the `.rafters/` directory.
    rafters_dir: PathBuf,
}

impl LocalRegistryReader {
    /// Create a new reader pointing at the given `.rafters/` directory.
    pub fn new(rafters_dir: impl Into<PathBuf>) -> Self {
        Self {
            rafters_dir: rafters_dir.into(),
        }
    }

    /// Detect and create a reader from a project root directory.
    /// Returns `Ok(reader)` if `.rafters/` exists, `Err` otherwise.
    pub fn detect(project_root: impl AsRef<Path>) -> Result<Self, RegistryError> {
        let rafters_dir = project_root.as_ref().join(".rafters");
        if rafters_dir.is_dir() {
            Ok(Self::new(rafters_dir))
        } else {
            Err(RegistryError::NotFound(rafters_dir))
        }
    }

    /// Path to the `.rafters/` directory.
    pub fn rafters_dir(&self) -> &Path {
        &self.rafters_dir
    }

    fn read_json<T: serde::de::DeserializeOwned>(&self, path: &Path) -> Result<T, RegistryError> {
        let file = std::fs::File::open(path).map_err(|e| RegistryError::io(path, e))?;
        serde_json::from_reader(std::io::BufReader::new(file))
            .map_err(|e| RegistryError::parse(path, e))
    }

    fn load_registry_item(
        &self,
        kind: &str,
        name: &str,
    ) -> Result<RegistryItem, RegistryError> {
        let path = self
            .rafters_dir
            .join("registry")
            .join(kind)
            .join(format!("{name}.json"));
        self.read_json(&path).map_err(|e| match &e {
            RegistryError::Io { source, .. }
                if source.kind() == std::io::ErrorKind::NotFound =>
            {
                RegistryError::ComponentNotFound(format!("{kind}/{name}"))
            }
            _ => e,
        })
    }
}

impl RegistryReader for LocalRegistryReader {
    fn load_namespaces(&self) -> Result<Vec<NamespaceTokenFile>, RegistryError> {
        let tokens_dir = self.rafters_dir.join("tokens");
        if !tokens_dir.is_dir() {
            return Ok(Vec::new());
        }

        let mut namespaces = Vec::new();
        let entries = std::fs::read_dir(&tokens_dir)
            .map_err(|e| RegistryError::io(&tokens_dir, e))?;

        for entry in entries {
            let entry = entry.map_err(|e| RegistryError::io(&tokens_dir, e))?;
            let path = entry.path();
            if path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(".rafters.json"))
            {
                let ns: NamespaceTokenFile = self.read_json(&path)?;
                namespaces.push(ns);
            }
        }

        Ok(namespaces)
    }

    fn load_namespace(&self, name: &str) -> Result<NamespaceTokenFile, RegistryError> {
        let path = self
            .rafters_dir
            .join("tokens")
            .join(format!("{name}.rafters.json"));
        self.read_json(&path).map_err(|e| match &e {
            RegistryError::Io { source, .. }
                if source.kind() == std::io::ErrorKind::NotFound =>
            {
                RegistryError::NamespaceNotFound(name.to_string())
            }
            _ => e,
        })
    }

    fn load_index(&self) -> Result<RegistryIndex, RegistryError> {
        let path = self.rafters_dir.join("registry").join("index.json");
        self.read_json(&path)
    }

    fn load_component(&self, name: &str) -> Result<RegistryItem, RegistryError> {
        self.load_registry_item("components", name)
    }

    fn load_primitive(&self, name: &str) -> Result<RegistryItem, RegistryError> {
        self.load_registry_item("primitives", name)
    }

    fn load_composite(&self, name: &str) -> Result<RegistryItem, RegistryError> {
        self.load_registry_item("composites", name)
    }

    fn load_config(&self) -> Result<RaftersConfig, RegistryError> {
        let path = self.rafters_dir.join("config.rafters.json");
        self.read_json(&path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_rafters_dir() -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let rafters = tmp.path().join(".rafters");
        std::fs::create_dir_all(rafters.join("tokens")).unwrap();
        std::fs::create_dir_all(rafters.join("registry/components")).unwrap();
        std::fs::create_dir_all(rafters.join("registry/primitives")).unwrap();
        std::fs::create_dir_all(rafters.join("registry/composites")).unwrap();
        (tmp, rafters)
    }

    #[test]
    fn detect_with_rafters_dir() {
        let (tmp, _) = create_rafters_dir();
        let reader = LocalRegistryReader::detect(tmp.path());
        assert!(reader.is_ok());
    }

    #[test]
    fn detect_without_rafters_dir() {
        let tmp = TempDir::new().unwrap();
        let result = LocalRegistryReader::detect(tmp.path());
        assert!(result.is_err());
        assert!(matches!(result, Err(RegistryError::NotFound(_))));
    }

    #[test]
    fn load_namespace_file() {
        let (tmp, rafters) = create_rafters_dir();
        let token_json = r#"{
            "$schema": "https://rafters.studio/schemas/namespace-tokens.json",
            "namespace": "color",
            "version": "1.0.0",
            "generatedAt": "2026-03-18T00:00:00Z",
            "tokens": [
                {
                    "name": "color-neutral-50",
                    "value": "oklch(0.97 0.003 264.5)",
                    "category": "color",
                    "namespace": "color"
                }
            ]
        }"#;
        std::fs::write(rafters.join("tokens/color.rafters.json"), token_json).unwrap();

        let reader = LocalRegistryReader::detect(tmp.path()).unwrap();
        let ns = reader.load_namespace("color").unwrap();
        assert_eq!(ns.namespace, "color");
        assert_eq!(ns.tokens.len(), 1);
        assert_eq!(ns.tokens[0].name, "color-neutral-50");
    }

    #[test]
    fn load_namespace_not_found() {
        let (tmp, _) = create_rafters_dir();
        let reader = LocalRegistryReader::detect(tmp.path()).unwrap();
        let result = reader.load_namespace("nonexistent");
        assert!(matches!(result, Err(RegistryError::NamespaceNotFound(_))));
    }

    #[test]
    fn load_all_namespaces() {
        let (tmp, rafters) = create_rafters_dir();

        for ns_name in ["color", "spacing"] {
            let json = format!(
                r#"{{
                    "namespace": "{ns_name}",
                    "version": "1.0.0",
                    "tokens": []
                }}"#
            );
            std::fs::write(
                rafters.join(format!("tokens/{ns_name}.rafters.json")),
                json,
            )
            .unwrap();
        }

        let reader = LocalRegistryReader::detect(tmp.path()).unwrap();
        let namespaces = reader.load_namespaces().unwrap();
        assert_eq!(namespaces.len(), 2);
    }

    #[test]
    fn load_registry_index() {
        let (tmp, rafters) = create_rafters_dir();
        let json = r#"{
            "name": "rafters",
            "homepage": "https://rafters.studio",
            "components": ["button"],
            "primitives": ["slot"],
            "composites": [],
            "rules": []
        }"#;
        std::fs::write(rafters.join("registry/index.json"), json).unwrap();

        let reader = LocalRegistryReader::detect(tmp.path()).unwrap();
        let index = reader.load_index().unwrap();
        assert_eq!(index.name, "rafters");
        assert_eq!(index.components, vec!["button"]);
    }

    #[test]
    fn load_component() {
        let (tmp, rafters) = create_rafters_dir();
        let json = r#"{
            "name": "button",
            "type": "ui",
            "description": "A button",
            "primitives": ["slot"],
            "files": [{
                "path": "components/ui/button.tsx",
                "content": "export function Button() {}",
                "dependencies": ["react@19.2.0"],
                "devDependencies": []
            }],
            "rules": [],
            "composites": []
        }"#;
        std::fs::write(rafters.join("registry/components/button.json"), json).unwrap();

        let reader = LocalRegistryReader::detect(tmp.path()).unwrap();
        let item = reader.load_component("button").unwrap();
        assert_eq!(item.name, "button");
        assert_eq!(item.files.len(), 1);
    }

    #[test]
    fn load_component_not_found() {
        let (tmp, _) = create_rafters_dir();
        let reader = LocalRegistryReader::detect(tmp.path()).unwrap();
        let result = reader.load_component("nonexistent");
        assert!(matches!(result, Err(RegistryError::ComponentNotFound(_))));
    }

    #[test]
    fn load_config() {
        let (tmp, rafters) = create_rafters_dir();
        let json = r#"{
            "framework": "astro",
            "componentTarget": "react",
            "componentsPath": "src/components/ui",
            "shadcn": true,
            "exports": { "tailwind": true },
            "installed": { "components": ["button"], "primitives": [], "composites": [] }
        }"#;
        std::fs::write(rafters.join("config.rafters.json"), json).unwrap();

        let reader = LocalRegistryReader::detect(tmp.path()).unwrap();
        let config = reader.load_config().unwrap();
        assert_eq!(config.framework, Some("astro".to_string()));
        assert_eq!(config.shadcn, Some(true));
    }
}
