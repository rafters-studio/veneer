//! Framework adapters for transforming JSX to Web Components.
//!
//! This crate provides the core transformation logic that converts React/Solid JSX
//! components into static Web Components for documentation previews.

pub mod conventions;
pub mod generator;
pub mod inline;
pub mod intelligence;
pub mod rafters_source;
pub mod react;
pub mod registry;
pub mod scope;
pub mod tokens;
pub mod traits;
pub(crate) mod ts_helpers;

pub use conventions::ComponentConventions;
pub use generator::{
    generate_controls_panel, generate_passthrough_web_component, generate_web_component,
    web_component_block,
};
pub use inline::{parse_inline_jsx, parse_inline_jsx_all, to_custom_element, InlineJsx, PropValue};
pub use intelligence::{
    render_component, CognitiveLoad, CompiledIntelligence, Constraint, ConstraintKind,
    DependencyOrigin, DependencyRef, PropDoc, RenderedComponent, TokenRef, VariantDoc,
};
pub use rafters_source::{
    read_rafters_namespace, AccessibilityMatrices, ContrastMatrix, ContrastPair,
    IntelligenceSource, NamespaceError, NamespaceFile, NamespaceToken, OklchComponents,
    RaftersNamespace, StructuredValue, TokenValue, UsagePatterns, UserOverride,
};
pub use react::{ComponentStructure, ReactAdapter};
pub use registry::{
    CachedComponent, ComponentRegistry, DiscoveredItem, DiscoveredKind, RegistryError,
};
pub use scope::{extract_classes_from_ts, scope_css};
pub use tokens::{parse_dtcg_tokens, DesignToken, DesignTokens, TokenParseError};
pub use traits::{FrameworkAdapter, TransformContext, TransformError, TransformedBlock};
