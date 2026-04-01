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

impl ComponentConventions {
    /// Create conventions for a `.classes.ts` file with a component prefix.
    ///
    /// For example, given prefix "badge", this creates conventions that match:
    /// - `badgeVariantClasses` for variants
    /// - `badgeSizeClasses` for sizes
    /// - `badgeBaseClasses` for base classes
    /// - `badgeDisabledClasses` for disabled classes
    ///
    /// Also includes the default unprefixed names as fallbacks.
    pub fn for_classes_file(prefix: &str) -> Self {
        let prefixed_variant = format!("{}VariantClasses", prefix);
        let prefixed_size = format!("{}SizeClasses", prefix);
        let prefixed_base = format!("{}BaseClasses", prefix);
        let prefixed_disabled = format!("{}DisabledClasses", prefix);

        Self {
            variant_records: vec![prefixed_variant, "variantClasses".to_string()],
            size_records: vec![prefixed_size, "sizeClasses".to_string()],
            base_class_vars: vec![prefixed_base, "baseClasses".to_string()],
            disabled_class_vars: vec![prefixed_disabled, "disabledClasses".to_string()],
            ..Default::default()
        }
    }
}
