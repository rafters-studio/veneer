//! Parser for a component behavior's exported `Config` interface -- the
//! canonical prop/API surface of a new-constitution rafters component
//! (interface contract, 2026-07-16).
//!
//! The `.element.ts` files are thin bind-wrappers with no `observedAttributes`
//! (the attributes are read once inside `bindX`), so reading props off the
//! element yields nothing. The real surface is the behavior's `<Name>Config`
//! interface (e.g. `ButtonConfig`), whose keys become the Web Component's
//! kebab-cased observed attributes (`delayDuration` -> `delay-duration`).
//!
//! The full surface is the interface's own properties PLUS the properties of
//! the `Config` interfaces it extends (`ButtonConfig extends PressableConfig`).
//! This parser reads ONE file: it returns the own properties and NAMES the
//! extended bases. Naming an unresolved base is honest-absence -- the doc
//! never silently drops inherited attributes, it declares
//! `extends PressableConfig` so a caller (or a reader) knows the surface
//! continues. Resolving the base across files is a separate step.

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Declaration, Expression, PropertyKey, Statement, TSInterfaceHeritage, TSSignature,
};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};

use crate::intelligence::PropDoc;
use crate::ts_helpers::{kebab_case, normalize_whitespace};

/// One component's `Config` interface as declared in its `.behavior.ts`: the
/// interface name, its own declared properties, and the names of the `Config`
/// interfaces it extends (unresolved -- each lives in another file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigInterface {
    /// The interface name, for example `ButtonConfig`.
    pub name: String,
    /// The properties declared directly on this interface, in source order.
    pub own_props: Vec<PropDoc>,
    /// The base `Config` interfaces this one extends, by name -- the
    /// unresolved remainder of the attribute surface.
    pub extends: Vec<String>,
}

/// The Web Component observed-attribute name for a config property: the
/// property key kebab-cased (`delayDuration` -> `delay-duration`).
pub fn attribute_name(property: &str) -> String {
    kebab_case(property)
}

/// Parse a `.behavior.ts` source for its exported `Config` interface. Returns
/// `None` when the module exports no `*Config` interface (a behavior that
/// declares no config), which is honest-absence, not a failure.
///
/// A behavior file declares exactly one `<Name>Config` interface (its own);
/// the bases it extends are imported, not declared here, so the first
/// exported interface whose name ends with `Config` is the component's.
pub fn parse_config_interface(source: &str) -> Result<Option<ConfigInterface>, String> {
    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, source, SourceType::ts()).parse();
    if ret.panicked || !ret.errors.is_empty() {
        return Err("failed to parse the behavior source while reading its Config interface".into());
    }

    for statement in &ret.program.body {
        let Statement::ExportNamedDeclaration(export) = statement else {
            continue;
        };
        let Some(Declaration::TSInterfaceDeclaration(interface)) = &export.declaration else {
            continue;
        };
        if interface.id.name.as_str().ends_with("Config") {
            return Ok(Some(config_from_interface(interface, source)));
        }
    }
    Ok(None)
}

fn config_from_interface(
    interface: &oxc_ast::ast::TSInterfaceDeclaration<'_>,
    source: &str,
) -> ConfigInterface {
    let name = interface.id.name.as_str().to_string();

    let mut own_props = Vec::new();
    for signature in &interface.body.body {
        let TSSignature::TSPropertySignature(property) = signature else {
            continue;
        };
        let prop_name = match &property.key {
            PropertyKey::StaticIdentifier(identifier) => identifier.name.as_str().to_string(),
            PropertyKey::StringLiteral(literal) => literal.value.as_str().to_string(),
            _ => continue,
        };
        let type_text = property.type_annotation.as_ref().map(|annotation| {
            normalize_whitespace(annotation.type_annotation.span().source_text(source))
        });
        if !own_props.iter().any(|prop: &PropDoc| prop.name == prop_name) {
            own_props.push(PropDoc {
                name: prop_name,
                type_text,
                optional: property.optional,
            });
        }
    }

    let extends = interface
        .extends
        .iter()
        .filter_map(heritage_name)
        .collect();

    ConfigInterface {
        name,
        own_props,
        extends,
    }
}

/// The base interface name from one `extends` clause entry. A simple
/// `extends PressableConfig` is an identifier; anything more exotic (a
/// qualified name, an expression) is skipped rather than guessed at.
fn heritage_name(heritage: &TSInterfaceHeritage<'_>) -> Option<String> {
    match &heritage.expression {
        Expression::Identifier(identifier) => Some(identifier.name.as_str().to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUTTON_BEHAVIOR: &str = r#"
        import { pressable, type PressableConfig } from '../../lib/pressable';
        export type ButtonVariant = 'default' | 'primary';
        export type ButtonSize = 'default' | 'sm';
        export interface ButtonConfig extends PressableConfig {
          variant: ButtonVariant;
          size: ButtonSize;
        }
        export const button = pressable();
    "#;

    const DIALOG_BEHAVIOR: &str = r#"
        import { disclosable, type DisclosableConfig } from '../../lib/disclosable';
        export interface DialogConfig extends DisclosableConfig {
          /** Modal dialogs trap focus. Default: true. */
          modal?: boolean | undefined;
        }
    "#;

    #[test]
    fn reads_own_props_and_names_the_extended_base() {
        let config = parse_config_interface(BUTTON_BEHAVIOR)
            .expect("parses")
            .expect("has a Config interface");
        assert_eq!(config.name, "ButtonConfig");
        assert_eq!(config.extends, ["PressableConfig"]);
        let names: Vec<&str> = config.own_props.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["variant", "size"]);
        assert_eq!(config.own_props[0].type_text.as_deref(), Some("ButtonVariant"));
        assert!(!config.own_props[0].optional);
    }

    #[test]
    fn an_optional_prop_carries_its_optional_flag_and_type() {
        let config = parse_config_interface(DIALOG_BEHAVIOR)
            .expect("parses")
            .expect("has a Config interface");
        assert_eq!(config.name, "DialogConfig");
        assert_eq!(config.extends, ["DisclosableConfig"]);
        assert_eq!(config.own_props.len(), 1);
        let modal = &config.own_props[0];
        assert_eq!(modal.name, "modal");
        assert!(modal.optional, "modal? is optional");
        assert_eq!(modal.type_text.as_deref(), Some("boolean | undefined"));
    }

    #[test]
    fn camel_case_keys_become_kebab_case_attributes() {
        assert_eq!(attribute_name("delayDuration"), "delay-duration");
        assert_eq!(attribute_name("modal"), "modal");
        assert_eq!(attribute_name("variant"), "variant");
    }

    #[test]
    fn a_behavior_without_a_config_interface_is_none_not_an_error() {
        let source = "export const x = 1; export interface ButtonState { open: boolean; }";
        assert_eq!(parse_config_interface(source).expect("parses"), None);
    }

    #[test]
    fn a_config_with_no_extends_has_an_empty_base_list() {
        let source = "export interface RootConfig { id: string; }";
        let config = parse_config_interface(source)
            .expect("parses")
            .expect("has a Config interface");
        assert!(config.extends.is_empty());
        assert_eq!(config.own_props.len(), 1);
        assert_eq!(config.own_props[0].name, "id");
    }
}
