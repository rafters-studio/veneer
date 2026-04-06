//! Documentation extraction and generation for veneer.

pub mod cli_parser;

pub use cli_parser::{
    mark_required_flags, parse_cli_help, CliParseError, ParsedCommand, ParsedFlag,
};
