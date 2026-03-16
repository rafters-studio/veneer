//! Configurable conventions for component extraction.
//!
//! Allows users to specify which variable names map to variant records,
//! size records, base classes, and other component structure elements,
//! making veneer work with any design system naming convention.

use serde::Deserialize;

/// Configuration for how component structure is identified in source code.
///
/// Each field contains a list of variable names or attribute names that
/// veneer should recognize. The defaults match the conventions used by
/// the original hardcoded extraction logic.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ComponentConventions {
    /// Variable names that hold variant-to-class mappings (e.g., "variantClasses").
    pub variant_records: Vec<String>,

    /// Variable names that hold size-to-class mappings (e.g., "sizeClasses").
    pub size_records: Vec<String>,

    /// Variable names that hold base CSS classes (e.g., "baseClasses").
    pub base_class_vars: Vec<String>,

    /// Variable names that hold disabled-state CSS classes (e.g., "disabledClasses").
    pub disabled_class_vars: Vec<String>,

    /// Prop names to exclude from observed attributes (e.g., "children", "className").
    pub excluded_props: Vec<String>,

    /// Attribute names to check as fallback when no interface or destructuring is found.
    pub fallback_attributes: Vec<String>,
}

impl Default for ComponentConventions {
    fn default() -> Self {
        Self {
            variant_records: vec!["variantClasses".to_string()],
            size_records: vec!["sizeClasses".to_string()],
            base_class_vars: vec!["baseClasses".to_string()],
            disabled_class_vars: vec!["disabledClasses".to_string(), "disabledCls".to_string()],
            excluded_props: vec![
                "children".to_string(),
                "className".to_string(),
                "style".to_string(),
            ],
            fallback_attributes: vec![
                "variant".to_string(),
                "size".to_string(),
                "disabled".to_string(),
                "loading".to_string(),
            ],
        }
    }
}
