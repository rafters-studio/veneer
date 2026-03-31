//! Component registry for looking up component definitions.
//!
//! Scans a components directory, parses source files, and provides
//! lookup by component name for generating Web Components.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::conventions::ComponentConventions;
use crate::generator::generate_web_component;
use crate::react::{ComponentStructure, ReactAdapter};
use crate::traits::TransformedBlock;

/// A registry of component definitions.
#[derive(Debug, Default)]
pub struct ComponentRegistry {
    /// Cached component structures by name (lowercase)
    components: HashMap<String, CachedComponent>,
}

/// A cached component with its source and structure.
#[derive(Debug, Clone)]
pub struct CachedComponent {
    /// Original component name
    pub name: String,

    /// Source file path
    pub source_path: PathBuf,

    /// Extracted structure
    pub structure: ComponentStructure,

    /// Full source code
    pub source: String,
}

impl ComponentRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Scan a directory for component files and populate the registry.
    ///
    /// Processes both component files (.tsx/.jsx) and class definition files
    /// (.classes.ts) to maximize coverage. Class definition files use
    /// prefix-based conventions (e.g., `badgeVariantClasses` in `badge.classes.ts`).
    pub fn scan(&mut self, components_dir: &Path) -> Result<usize, RegistryError> {
        if !components_dir.exists() {
            return Err(RegistryError::DirectoryNotFound(
                components_dir.display().to_string(),
            ));
        }

        let default_adapter = ReactAdapter::new();
        let mut count = 0;

        for entry in WalkDir::new(components_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Skip test files, stories, and index files
            if filename.contains(".test.")
                || filename.contains(".spec.")
                || filename.contains(".stories.")
                || filename == "index.tsx"
                || filename == "index.jsx"
                || filename == "index.ts"
            {
                continue;
            }

            // Determine file type and extraction strategy
            let is_classes_file = filename.ends_with(".classes.ts");
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

            if !is_classes_file && ext != "tsx" && ext != "jsx" {
                continue;
            }

            // Read source
            let source = match fs::read_to_string(path) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Extract structure based on file type
            let (structure, name) = if is_classes_file {
                // For .classes.ts files, derive prefix from filename
                // e.g., "badge.classes.ts" -> prefix "badge", name "Badge"
                let stem = filename.strip_suffix(".classes.ts").unwrap_or("unknown");
                let prefix = kebab_to_camel(stem);
                let component_name = kebab_to_pascal(stem);

                let conventions = ComponentConventions::for_classes_file(&prefix);
                let adapter = ReactAdapter::with_conventions(conventions);
                match adapter.extract_structure(&source) {
                    Ok(mut s) => {
                        // Override the name since classes files have no function declaration
                        s.name = component_name.clone();
                        (s, component_name)
                    }
                    Err(_) => continue,
                }
            } else {
                // Standard component file
                match default_adapter.extract_structure(&source) {
                    Ok(s) => {
                        let name = if s.name.is_empty() || s.name == "Component" {
                            path.file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown")
                                .to_string()
                        } else {
                            s.name.clone()
                        };
                        (s, name)
                    }
                    Err(_) => continue,
                }
            };

            let cached = CachedComponent {
                name: name.clone(),
                source_path: path.to_path_buf(),
                structure,
                source,
            };

            // Store by lowercase name for case-insensitive lookup.
            // Only insert if not already present (component files take priority over classes files).
            let key = name.to_lowercase();
            if let std::collections::hash_map::Entry::Vacant(e) = self.components.entry(key) {
                e.insert(cached);
                count += 1;
            }
        }

        Ok(count)
    }

    /// Look up a component by name (case-insensitive).
    pub fn get(&self, name: &str) -> Option<&CachedComponent> {
        self.components.get(&name.to_lowercase())
    }

    /// Check if a component exists.
    pub fn contains(&self, name: &str) -> bool {
        self.components.contains_key(&name.to_lowercase())
    }

    /// Get all registered component names.
    pub fn names(&self) -> Vec<&str> {
        self.components.values().map(|c| c.name.as_str()).collect()
    }

    /// Generate a Web Component for a registered component.
    pub fn generate_web_component(
        &self,
        component_name: &str,
        tag_name: &str,
    ) -> Result<TransformedBlock, RegistryError> {
        let cached = self
            .get(component_name)
            .ok_or_else(|| RegistryError::ComponentNotFound(component_name.to_string()))?;

        let classes_used = cached.structure.collect_all_classes();

        let web_component = generate_web_component(tag_name, &cached.structure);

        Ok(TransformedBlock {
            web_component,
            tag_name: tag_name.to_string(),
            classes_used,
            attributes: cached.structure.observed_attributes.clone(),
        })
    }
}

/// Convert kebab-case to camelCase (e.g., "context-menu" -> "contextMenu").
fn kebab_to_camel(s: &str) -> String {
    let mut result = String::new();

    for (i, part) in s.split('-').enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            result.push_str(part);
        } else {
            let mut chars = part.chars();
            if let Some(c) = chars.next() {
                result.push(c.to_ascii_uppercase());
                result.push_str(chars.as_str());
            }
        }
    }

    result
}

/// Convert kebab-case to PascalCase (e.g., "context-menu" -> "ContextMenu").
fn kebab_to_pascal(s: &str) -> String {
    s.split('-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(c) => {
                    let mut s = c.to_uppercase().collect::<String>();
                    s.push_str(chars.as_str());
                    s
                }
                None => String::new(),
            }
        })
        .collect()
}

/// Errors that can occur with the registry.
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Components directory not found: {0}")]
    DirectoryNotFound(String),

    #[error("Component not found: {0}")]
    ComponentNotFound(String),

    #[error("Failed to parse component: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn scans_components_directory() {
        let temp = tempdir().unwrap();
        let comp_dir = temp.path().join("components");
        fs::create_dir_all(&comp_dir).unwrap();

        // Create a valid component file
        fs::write(
            comp_dir.join("button.tsx"),
            r#"
const variantClasses = {
  default: 'bg-primary text-white',
  secondary: 'bg-secondary text-black',
};

export function Button() {
  return <button />;
}
            "#,
        )
        .unwrap();

        let mut registry = ComponentRegistry::new();
        let count = registry.scan(&comp_dir).unwrap();

        assert_eq!(count, 1);
        assert!(registry.contains("Button"));
    }

    #[test]
    fn generates_web_component_from_registry() {
        let temp = tempdir().unwrap();
        let comp_dir = temp.path().join("components");
        fs::create_dir_all(&comp_dir).unwrap();

        fs::write(
            comp_dir.join("button.tsx"),
            r#"
const variantClasses = {
  primary: 'bg-blue-500',
};

export function Button() {}
            "#,
        )
        .unwrap();

        let mut registry = ComponentRegistry::new();
        registry.scan(&comp_dir).unwrap();

        let result = registry
            .generate_web_component("Button", "button-preview")
            .unwrap();

        assert_eq!(result.tag_name, "button-preview");
        assert!(result.web_component.contains("bg-blue-500"));
    }

    #[test]
    fn scans_classes_ts_files() {
        let temp = tempdir().unwrap();
        let comp_dir = temp.path().join("components");
        fs::create_dir_all(&comp_dir).unwrap();

        // Create a .classes.ts file matching rafters convention
        fs::write(
            comp_dir.join("badge.classes.ts"),
            r#"
export const badgeVariantClasses: Record<string, string> = {
  default: 'bg-primary text-primary-foreground',
  secondary: 'bg-secondary text-secondary-foreground',
  destructive: 'bg-destructive text-destructive-foreground',
};

export const badgeSizeClasses: Record<string, string> = {
  sm: 'px-2 py-0.5 text-xs',
  default: 'px-2.5 py-0.5 text-xs',
  lg: 'px-3 py-1 text-sm',
};

export const badgeBaseClasses = 'inline-flex items-center rounded-full font-semibold';
            "#,
        )
        .unwrap();

        let mut registry = ComponentRegistry::new();
        let count = registry.scan(&comp_dir).unwrap();

        assert_eq!(count, 1, "Should register 1 component from classes file");
        assert!(
            registry.contains("Badge"),
            "Should register as PascalCase 'Badge'"
        );

        let cached = registry.get("Badge").unwrap();
        assert_eq!(cached.structure.variant_lookup.len(), 3);
        assert_eq!(cached.structure.size_lookup.len(), 3);
        assert!(cached.structure.base_classes.contains("inline-flex"));
    }

    #[test]
    fn scans_kebab_case_classes_file() {
        let temp = tempdir().unwrap();
        let comp_dir = temp.path().join("components");
        fs::create_dir_all(&comp_dir).unwrap();

        // Multi-word kebab-case class file
        fs::write(
            comp_dir.join("context-menu.classes.ts"),
            r#"
export const contextMenuItemClasses = 'flex items-center px-2 py-1.5';
export const contextMenuContentClasses = 'bg-popover text-popover-foreground';
            "#,
        )
        .unwrap();

        let mut registry = ComponentRegistry::new();
        let count = registry.scan(&comp_dir).unwrap();

        // This file has no variantClasses, so it should NOT register
        assert_eq!(count, 0);
    }

    #[test]
    fn kebab_to_camel_works() {
        assert_eq!(super::kebab_to_camel("badge"), "badge");
        assert_eq!(super::kebab_to_camel("context-menu"), "contextMenu");
        assert_eq!(super::kebab_to_camel("alert-dialog"), "alertDialog");
        assert_eq!(super::kebab_to_camel("input-otp"), "inputOtp");
    }

    #[test]
    fn kebab_to_pascal_works() {
        assert_eq!(super::kebab_to_pascal("badge"), "Badge");
        assert_eq!(super::kebab_to_pascal("context-menu"), "ContextMenu");
        assert_eq!(super::kebab_to_pascal("alert-dialog"), "AlertDialog");
    }

    #[test]
    fn skips_test_and_story_files() {
        let temp = tempdir().unwrap();
        let comp_dir = temp.path().join("components");
        fs::create_dir_all(&comp_dir).unwrap();

        // These should be skipped
        fs::write(
            comp_dir.join("button.test.tsx"),
            "const variantClasses = { test: 'x' };",
        )
        .unwrap();
        fs::write(
            comp_dir.join("button.stories.tsx"),
            "const variantClasses = { story: 'y' };",
        )
        .unwrap();

        let mut registry = ComponentRegistry::new();
        let count = registry.scan(&comp_dir).unwrap();

        assert_eq!(count, 0);
    }
}
