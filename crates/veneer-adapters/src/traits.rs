//! Trait definitions for framework adapters.

use std::collections::HashMap;

/// Context for transforming a component.
#[derive(Debug, Clone, Default)]
pub struct TransformContext {
    /// Import mappings: "@components/*" -> "./src/components/*"
    pub import_map: HashMap<String, String>,

    /// Full text of the project stylesheet (for rafters projects,
    /// `.rafters/output/rafters.css` -- see `read_rafters_stylesheet`).
    /// The transform scopes the component's CSS out of it for the shadow
    /// root (FR-VEN-018). Empty means the project declares no compiled
    /// stylesheet: a component that declares classes then fails to
    /// transform with an error naming it, never rendering silently
    /// unstyled.
    pub stylesheet: String,
}

/// Result of transforming a component to a Web Component.
#[derive(Debug, Clone)]
pub struct TransformedBlock {
    /// Web Component class definition (JavaScript)
    pub web_component: String,

    /// Custom element tag name (e.g., "button-preview")
    pub tag_name: String,

    /// Tailwind classes used (for CSS scanning)
    pub classes_used: Vec<String>,

    /// Observed attributes derived from props
    pub attributes: Vec<String>,
}

/// Errors that can occur during transformation.
#[derive(Debug, thiserror::Error)]
pub enum TransformError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Transform error: {0}")]
    TransformError(String),

    #[error(
        "No class data found: component has no variant records, size records, or base classes"
    )]
    MissingVariants,

    #[error("Invalid component structure: {0}")]
    InvalidStructure(String),

    /// A component or composite failed to render. Names the item so it
    /// feeds coverage/staleness surfaces instead of silently vanishing
    /// from the output (FR-VEN-003).
    #[error("failed to render {component}: {reason}")]
    RenderFailed { component: String, reason: String },
}

/// Trait for framework-specific adapters.
pub trait FrameworkAdapter: Send + Sync {
    /// Framework identifier (e.g., "react", "solid")
    fn name(&self) -> &'static str;

    /// File extensions this adapter handles
    fn extensions(&self) -> &[&'static str];

    /// Transform source component into a Web Component.
    ///
    /// # Arguments
    /// * `source` - The source code of the component
    /// * `tag_name` - The custom element tag name to use
    /// * `ctx` - Transform context with import mappings
    fn transform(
        &self,
        source: &str,
        tag_name: &str,
        ctx: &TransformContext,
    ) -> Result<TransformedBlock, TransformError>;
}
