//! Documentation extraction and generation for veneer.

pub mod cli_parser;
pub mod mdx_reference;
pub mod sidebar;
pub mod skeleton;

pub use cli_parser::{
    mark_required_flags, parse_cli_help, CliParseError, ParsedCommand, ParsedFlag,
};
pub use mdx_reference::{
    generate_command_mdx, generate_reference_pages, GeneratedPage, MdxGenError,
};
pub use sidebar::{
    generate_sidebar, write_sidebar_jsonl, EditorialPage, SidebarError, SidebarNode,
};
pub use skeleton::{generate_default_skeletons, generate_skeleton, PageTemplate, SkeletonError};
