//! Component registry for looking up component definitions.
//!
//! Scans a components directory, parses source files, and provides
//! lookup by component name for generating Web Components.
//!
//! Uses export discovery instead of hardcoded naming conventions:
//! reads every `export const` in a `.classes.ts` file, categorizes
//! each export by its shape and name suffix, and builds the component
//! record from what actually exists.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    BindingPatternKind, Declaration, Expression, ObjectPropertyKind, PropertyKey, Statement,
};
use oxc_parser::Parser;
use oxc_span::SourceType;
use walkdir::WalkDir;

use crate::generator::generate_web_component;
use crate::react::{ComponentStructure, ReactAdapter};
use crate::traits::TransformedBlock;
use crate::ts_helpers::{
    extract_nested_object_classes, extract_string_value, normalize_whitespace,
    unwrap_type_expressions,
};

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

/// A single discovered export from a .classes.ts file.
#[derive(Debug)]
struct DiscoveredExport {
    name: String,
    shape: ExportShape,
}

/// The value shape of a discovered export.
#[derive(Debug)]
enum ExportShape {
    /// Key-value pairs (from flat or nested object expressions)
    Record { entries: Vec<(String, String)> },
    /// A single resolved class string (from string literal, template, array, or join)
    Scalar { value: String },
}

/// Categorize an export name based on what role it plays in the component.
///
/// Priority order: Variant > Size > Base > Disabled > Other.
/// First match wins, so `variantSizeClasses` would classify as Variant.
#[derive(Debug, PartialEq)]
enum ExportRole {
    Variant,
    Size,
    Base,
    Disabled,
    Other,
}

impl ExportRole {
    fn classify(name: &str) -> Self {
        let lower = name.to_lowercase();
        if lower.contains("variant") {
            Self::Variant
        } else if lower.contains("size") {
            Self::Size
        } else if lower.contains("base") {
            Self::Base
        } else if lower.contains("disabled") {
            Self::Disabled
        } else {
            Self::Other
        }
    }
}

/// Discover all exported constants from a TypeScript source file.
///
/// This is the core of the discovery approach: instead of looking for
/// specific variable names, we read every `export const` and determine
/// its shape (flat record, nested record, string, array, object).
fn discover_exports(source: &str, file_hint: &str) -> Vec<DiscoveredExport> {
    let allocator = Allocator::default();
    let source_type = SourceType::ts();
    let ret = Parser::new(&allocator, source, source_type).parse();

    if ret.panicked {
        eprintln!("warning: parser panicked on {file_hint}");
        return Vec::new();
    }
    if !ret.errors.is_empty() {
        eprintln!(
            "warning: parse errors in {file_hint}: {}",
            ret.errors
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        );
        return Vec::new();
    }

    let mut exports = Vec::new();

    for stmt in &ret.program.body {
        let Statement::ExportNamedDeclaration(export) = stmt else {
            continue;
        };
        let Some(ref decl) = export.declaration else {
            continue;
        };
        let Declaration::VariableDeclaration(var_decl) = decl else {
            continue;
        };

        for declarator in &var_decl.declarations {
            let name = match &declarator.id.kind {
                BindingPatternKind::BindingIdentifier(id) => id.name.as_str().to_string(),
                _ => continue,
            };

            let Some(ref init) = declarator.init else {
                continue;
            };

            let init = unwrap_type_expressions(init);

            if let Some(export) = try_extract_export(&name, init) {
                exports.push(export);
            }
        }
    }

    exports
}

/// Try to extract a DiscoveredExport from an expression.
fn try_extract_export(name: &str, expr: &Expression<'_>) -> Option<DiscoveredExport> {
    // Try as object expression first (Records and plain objects)
    if let Expression::ObjectExpression(obj) = expr {
        let mut flat_entries: Vec<(String, String)> = Vec::new();
        let mut nested_entries: Vec<(String, String)> = Vec::new();
        let mut has_nested = false;

        for prop in &obj.properties {
            let ObjectPropertyKind::ObjectProperty(prop) = prop else {
                continue;
            };

            let key = match &prop.key {
                PropertyKey::StaticIdentifier(id) => id.name.as_str().to_string(),
                PropertyKey::StringLiteral(s) => s.value.as_str().to_string(),
                PropertyKey::NumericLiteral(n) => {
                    // Use integer formatting if it's a whole number, otherwise float
                    let v = n.value;
                    if v == (v as i64) as f64 {
                        format!("{}", v as i64)
                    } else {
                        format!("{}", v)
                    }
                }
                _ => continue,
            };

            let value_expr = unwrap_type_expressions(&prop.value);

            // Try string value first
            if let Some(value) = extract_string_value(value_expr) {
                flat_entries.push((key.clone(), value.clone()));
                nested_entries.push((key, value));
            }
            // Try nested object (flatten all string values)
            else if let Some(value) = extract_nested_object_classes(value_expr) {
                nested_entries.push((key, value));
                has_nested = true;
            }
        }

        let entries = if has_nested {
            nested_entries
        } else {
            flat_entries
        };
        if !entries.is_empty() {
            return Some(DiscoveredExport {
                name: name.to_string(),
                shape: ExportShape::Record { entries },
            });
        }

        // Object with no extractable values -- still record it if it has properties
        // (might be entirely nested objects we could not parse)
        return None;
    }

    // Try as string literal / template literal / concatenation
    if let Some(value) = extract_string_value(expr) {
        if !value.is_empty() {
            return Some(DiscoveredExport {
                name: name.to_string(),
                shape: ExportShape::Scalar { value },
            });
        }
        return None;
    }

    // Try as array expression (join elements)
    if let Expression::ArrayExpression(arr) = expr {
        let parts = collect_array_string_values(arr);
        if !parts.is_empty() {
            return Some(DiscoveredExport {
                name: name.to_string(),
                shape: ExportShape::Scalar {
                    value: parts.join(" "),
                },
            });
        }
    }

    // Try as method call -- e.g., [...].join(' ')
    if let Expression::CallExpression(call) = expr {
        if let Expression::StaticMemberExpression(member) = &call.callee {
            if member.property.name.as_str() == "join" {
                if let Expression::ArrayExpression(arr) = &member.object {
                    let parts = collect_array_string_values(arr);
                    if !parts.is_empty() {
                        let sep = call
                            .arguments
                            .first()
                            .and_then(|arg| {
                                if let oxc_ast::ast::Argument::StringLiteral(s) = arg {
                                    Some(s.value.as_str().to_string())
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_else(|| " ".to_string());
                        return Some(DiscoveredExport {
                            name: name.to_string(),
                            shape: ExportShape::Scalar {
                                value: parts.join(&sep),
                            },
                        });
                    }
                }
            }
        }
    }

    None
}

/// Extract non-empty string values from array elements.
fn collect_array_string_values(arr: &oxc_ast::ast::ArrayExpression<'_>) -> Vec<String> {
    let mut parts = Vec::new();
    for element in &arr.elements {
        let expr_ref = match element {
            oxc_ast::ast::ArrayExpressionElement::SpreadElement(_) => continue,
            oxc_ast::ast::ArrayExpressionElement::Elision(_) => continue,
            _ => element.to_expression(),
        };
        if let Some(value) = extract_string_value(expr_ref) {
            if !value.is_empty() {
                parts.push(value);
            }
        }
    }
    parts
}

/// Build a ComponentStructure from discovered exports.
///
/// Uses the export name to classify each export's role (variant, size,
/// base, disabled, other) and then assembles the structure from whatever
/// was found. Never rejects a file for missing a specific export name.
fn build_structure_from_exports(
    component_name: &str,
    exports: Vec<DiscoveredExport>,
) -> Option<ComponentStructure> {
    if exports.is_empty() {
        return None;
    }

    let mut variant_lookup: Vec<(String, String)> = Vec::new();
    let mut size_lookup: Vec<(String, String)> = Vec::new();
    let mut base_classes_parts: Vec<String> = Vec::new();
    let mut disabled_classes: Option<String> = None;
    let mut extra_classes: Vec<String> = Vec::new();

    for export in exports {
        let role = ExportRole::classify(&export.name);

        match (role, export.shape) {
            (ExportRole::Variant, ExportShape::Record { entries }) => {
                if variant_lookup.is_empty() {
                    variant_lookup = entries;
                } else {
                    for (_, v) in entries {
                        extra_classes.push(v);
                    }
                }
            }

            (ExportRole::Size, ExportShape::Record { entries }) => {
                if size_lookup.is_empty() {
                    size_lookup = entries;
                } else {
                    for (_, v) in entries {
                        extra_classes.push(v);
                    }
                }
            }

            (ExportRole::Base, ExportShape::Scalar { value }) => {
                base_classes_parts.push(value);
            }

            (ExportRole::Disabled, ExportShape::Scalar { value }) => {
                disabled_classes = Some(value);
            }

            (ExportRole::Other, ExportShape::Record { entries }) => {
                for (_, v) in entries {
                    if !v.is_empty() {
                        extra_classes.push(v);
                    }
                }
            }

            (ExportRole::Other, ExportShape::Scalar { value }) => {
                if !value.is_empty() {
                    base_classes_parts.push(value);
                }
            }

            (ExportRole::Base, ExportShape::Record { entries }) => {
                for (_, v) in entries {
                    if !v.is_empty() {
                        base_classes_parts.push(v);
                    }
                }
            }

            (ExportRole::Disabled, ExportShape::Record { entries }) => {
                let combined: String = entries
                    .into_iter()
                    .filter(|(_, v)| !v.is_empty())
                    .map(|(_, v)| v)
                    .collect::<Vec<_>>()
                    .join(" ");
                if !combined.is_empty() {
                    disabled_classes = Some(combined);
                }
            }

            // Variant/Size role but scalar value -- treat as base classes
            (ExportRole::Variant | ExportRole::Size, ExportShape::Scalar { value }) => {
                if !value.is_empty() {
                    base_classes_parts.push(value);
                }
            }
        }
    }

    // Combine base classes -- always include extra_classes (from "Other" role records)
    // alongside any explicit base classes found.
    let mut all_base_parts: Vec<String> = base_classes_parts;
    all_base_parts.extend(extra_classes);
    let base_classes = all_base_parts.join(" ");

    // We found something -- build the structure
    let has_content =
        !variant_lookup.is_empty() || !size_lookup.is_empty() || !base_classes.is_empty();

    if !has_content {
        return None;
    }

    let default_variant = variant_lookup
        .first()
        .map(|(k, _)| k.clone())
        .unwrap_or_else(|| "default".to_string());

    let default_size = size_lookup
        .first()
        .map(|(k, _)| k.clone())
        .unwrap_or_else(|| "default".to_string());

    // Infer observed attributes from what we found
    let mut observed_attributes = Vec::new();
    if !variant_lookup.is_empty() {
        observed_attributes.push("variant".to_string());
    }
    if !size_lookup.is_empty() {
        observed_attributes.push("size".to_string());
    }

    Some(ComponentStructure {
        name: component_name.to_string(),
        variant_lookup,
        size_lookup,
        base_classes: normalize_whitespace(&base_classes),
        disabled_classes: disabled_classes
            .unwrap_or_else(|| "opacity-50 pointer-events-none cursor-not-allowed".to_string()),
        default_variant,
        default_size,
        observed_attributes,
    })
}

impl ComponentRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Scan a directory for component files and populate the registry.
    ///
    /// For `.classes.ts` files, uses export discovery: reads every `export const`,
    /// categorizes by name pattern and value shape, builds component records from
    /// whatever is actually exported. Never rejects a file for not matching a
    /// specific naming convention.
    ///
    /// For `.tsx`/`.jsx` files, uses the ReactAdapter with conventions-based
    /// extraction (unchanged -- these are component source files, not class defs).
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
                // Discovery-based extraction for .classes.ts files
                let stem = filename.strip_suffix(".classes.ts").unwrap_or("unknown");
                let component_name = kebab_to_pascal(stem);

                let exports = discover_exports(&source, filename);
                match build_structure_from_exports(&component_name, exports) {
                    Some(s) => (s, component_name),
                    None => continue,
                }
            } else {
                // Standard component file (.tsx/.jsx) -- use conventions-based extraction
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
    fn discovers_prefixed_variant_and_size_classes() {
        let temp = tempdir().unwrap();
        let comp_dir = temp.path().join("components");
        fs::create_dir_all(&comp_dir).unwrap();

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
    fn discovers_structural_component_classes() {
        let temp = tempdir().unwrap();
        let comp_dir = temp.path().join("components");
        fs::create_dir_all(&comp_dir).unwrap();

        // Structural components export individual class constants without variant records
        fs::write(
            comp_dir.join("accordion.classes.ts"),
            r#"
export const accordionItemClasses = 'border-b';
export const accordionTriggerClasses = 'flex items-center px-2 py-1.5';
export const accordionContentClasses = 'overflow-hidden transition-all';
            "#,
        )
        .unwrap();

        let mut registry = ComponentRegistry::new();
        let count = registry.scan(&comp_dir).unwrap();

        assert_eq!(count, 1);
        assert!(registry.contains("Accordion"));

        let cached = registry.get("Accordion").unwrap();
        assert!(cached.structure.base_classes.contains("border-b"));
        assert!(cached.structure.base_classes.contains("flex"));
    }

    #[test]
    fn discovers_nested_record_variant_classes() {
        let temp = tempdir().unwrap();
        let comp_dir = temp.path().join("components");
        fs::create_dir_all(&comp_dir).unwrap();

        // Checkbox has nested variant records: Record<string, { border: string; checked: string; ring: string }>
        fs::write(
            comp_dir.join("checkbox.classes.ts"),
            r#"
export const checkboxBaseClasses = 'inline-flex items-center';
export const checkboxVariantClasses: Record<string, { border: string; checked: string; ring: string }> = {
  default: {
    border: 'border-primary',
    checked: 'data-[state=checked]:bg-primary',
    ring: 'focus-visible:ring-primary-ring',
  },
  secondary: {
    border: 'border-secondary',
    checked: 'data-[state=checked]:bg-secondary',
    ring: 'focus-visible:ring-secondary-ring',
  },
};
export const checkboxSizeClasses: Record<string, { box: string; icon: string }> = {
  sm: { box: 'h-3.5 w-3.5', icon: 'h-2.5 w-2.5' },
  default: { box: 'h-4 w-4', icon: 'h-3 w-3' },
};
            "#,
        )
        .unwrap();

        let mut registry = ComponentRegistry::new();
        let count = registry.scan(&comp_dir).unwrap();

        assert_eq!(count, 1);
        assert!(registry.contains("Checkbox"));

        let cached = registry.get("Checkbox").unwrap();
        assert_eq!(cached.structure.variant_lookup.len(), 2);
        assert_eq!(cached.structure.size_lookup.len(), 2);
        // Nested values should be flattened
        assert!(cached.structure.variant_lookup[0]
            .1
            .contains("border-primary"));
    }

    #[test]
    fn discovers_typography_plain_object() {
        let temp = tempdir().unwrap();
        let comp_dir = temp.path().join("components");
        fs::create_dir_all(&comp_dir).unwrap();

        fs::write(
            comp_dir.join("typography.classes.ts"),
            r#"
export const typographyClasses = {
  h1: 'scroll-m-20 text-4xl font-bold',
  h2: 'scroll-m-20 text-3xl font-semibold',
  p: 'leading-7 text-foreground',
} as const;
            "#,
        )
        .unwrap();

        let mut registry = ComponentRegistry::new();
        let count = registry.scan(&comp_dir).unwrap();

        assert_eq!(count, 1);
        assert!(registry.contains("Typography"));

        let cached = registry.get("Typography").unwrap();
        // typographyClasses is "Other" role, no variant/size in name,
        // so it goes into extra_classes -> base_classes
        assert!(cached.structure.base_classes.contains("text-4xl"));
    }

    #[test]
    fn discovers_grid_multiple_records() {
        let temp = tempdir().unwrap();
        let comp_dir = temp.path().join("components");
        fs::create_dir_all(&comp_dir).unwrap();

        fs::write(
            comp_dir.join("grid.classes.ts"),
            r#"
export const gridGapClasses: Record<string, string> = {
  '0': 'gap-0',
  '4': 'gap-4',
  '8': 'gap-8',
};

export const gridColumnClasses: Record<string, string> = {
  1: 'grid-cols-1',
  2: 'grid-cols-2',
  3: 'grid-cols-3',
};

export const gridGoldenClasses = 'grid-cols-3 [&>*:first-child]:col-span-2';
            "#,
        )
        .unwrap();

        let mut registry = ComponentRegistry::new();
        let count = registry.scan(&comp_dir).unwrap();

        assert_eq!(count, 1);
        assert!(registry.contains("Grid"));

        let cached = registry.get("Grid").unwrap();
        // All these are "Other" role, so they contribute to base_classes
        assert!(cached.structure.base_classes.contains("gap-0"));
        assert!(cached.structure.base_classes.contains("grid-cols-1"));
    }

    #[test]
    fn discovers_container_with_size_records() {
        let temp = tempdir().unwrap();
        let comp_dir = temp.path().join("components");
        fs::create_dir_all(&comp_dir).unwrap();

        fs::write(
            comp_dir.join("container.classes.ts"),
            r#"
export const containerSizeClasses: Record<string, string> = {
  sm: 'max-w-sm',
  md: 'max-w-md',
  lg: 'max-w-lg',
};

export const containerPaddingClasses: Record<string, string> = {
  '4': 'p-4',
  '8': 'p-8',
};
            "#,
        )
        .unwrap();

        let mut registry = ComponentRegistry::new();
        let count = registry.scan(&comp_dir).unwrap();

        assert_eq!(count, 1);
        assert!(registry.contains("Container"));

        let cached = registry.get("Container").unwrap();
        // containerSizeClasses -> Size role
        assert_eq!(cached.structure.size_lookup.len(), 3);
        assert_eq!(cached.structure.size_lookup[0].0, "sm");
    }

    #[test]
    fn discovers_array_join_pattern() {
        let temp = tempdir().unwrap();
        let comp_dir = temp.path().join("components");
        fs::create_dir_all(&comp_dir).unwrap();

        fs::write(
            comp_dir.join("tabs.classes.ts"),
            r#"
export const tabsListClasses = 'inline-flex items-center';
export const tabsTriggerBaseClasses = [
  'inline-flex items-center',
  'justify-center whitespace-nowrap',
].join(' ');
            "#,
        )
        .unwrap();

        let mut registry = ComponentRegistry::new();
        let count = registry.scan(&comp_dir).unwrap();

        assert_eq!(count, 1);
        assert!(registry.contains("Tabs"));

        let cached = registry.get("Tabs").unwrap();
        assert!(cached.structure.base_classes.contains("inline-flex"));
    }

    #[test]
    fn scans_kebab_case_classes_file() {
        let temp = tempdir().unwrap();
        let comp_dir = temp.path().join("components");
        fs::create_dir_all(&comp_dir).unwrap();

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

        assert_eq!(count, 1);
        assert!(registry.contains("ContextMenu"));

        let cached = registry.get("ContextMenu").unwrap();
        assert!(cached.structure.base_classes.contains("flex"));
        assert!(cached.structure.base_classes.contains("bg-popover"));
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

    #[test]
    fn classify_export_names() {
        assert_eq!(
            ExportRole::classify("badgeVariantClasses"),
            ExportRole::Variant
        );
        assert_eq!(ExportRole::classify("buttonSizeClasses"), ExportRole::Size);
        assert_eq!(ExportRole::classify("inputBaseClasses"), ExportRole::Base);
        assert_eq!(
            ExportRole::classify("switchTrackDisabledClasses"),
            ExportRole::Disabled
        );
        assert_eq!(
            ExportRole::classify("accordionItemClasses"),
            ExportRole::Other
        );
        assert_eq!(ExportRole::classify("typographyClasses"), ExportRole::Other);
        assert_eq!(ExportRole::classify("gridGoldenClasses"), ExportRole::Other);
    }

    #[test]
    fn discover_exports_finds_all_shapes() {
        let source = r#"
export const fooVariantClasses: Record<string, string> = {
  primary: 'bg-blue',
  secondary: 'bg-gray',
};

export const fooBaseClasses = 'inline-flex items-center';

export const fooDisabledClasses = 'opacity-50';
        "#;

        let exports = discover_exports(source, "test.classes.ts");
        assert_eq!(exports.len(), 3);
    }
}
