//! Framework adapters for transforming JSX to Web Components.
//!
//! This crate provides the core transformation logic that converts React/Solid JSX
//! components into static Web Components for documentation previews.

pub mod conventions;
pub mod generator;
pub mod inline;
pub mod react;
pub mod registry;
pub mod tokens;
pub mod traits;

pub use conventions::ComponentConventions;
pub use generator::{
    generate_controls_panel, generate_passthrough_web_component, generate_web_component,
};
pub use inline::{parse_inline_jsx, parse_inline_jsx_all, to_custom_element, InlineJsx, PropValue};
pub use react::{ComponentStructure, ReactAdapter};
pub use registry::{CachedComponent, ComponentRegistry, RegistryError};
pub use tokens::{parse_dtcg_tokens, DesignToken, DesignTokens, TokenParseError};
pub use traits::{FrameworkAdapter, TransformContext, TransformError, TransformedBlock};
