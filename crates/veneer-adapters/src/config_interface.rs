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

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Declaration, Expression, ImportDeclarationSpecifier, PropertyKey, Statement,
    TSInterfaceHeritage, TSSignature,
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

/// The full prop/API surface of a component's `Config` interface: its own
/// properties merged with every property it inherits through its resolved
/// extends chain, plus any bases that could not be resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConfig {
    /// The component's own Config interface name (for example `ButtonConfig`).
    pub name: String,
    /// The complete property surface: own properties first, then inherited
    /// ones in resolution order. A subtype property shadows an inherited one
    /// of the same name (own wins).
    pub props: Vec<PropDoc>,
    /// Bases that could not be resolved to a local interface -- an external
    /// (node_modules) base, a missing file, or a re-export this reader does
    /// not follow. Named honestly so the surface never claims to be complete
    /// when it is not.
    pub unresolved_extends: Vec<String>,
}

/// A parsed TS module: every interface it declares (by name, own props, and
/// the bases it extends) and the module each named import resolves to.
struct ParsedModule {
    interfaces: Vec<ConfigInterface>,
    /// `(local name, module specifier)` for each named import, so an extends
    /// base can be traced to the file that declares it.
    imports: Vec<(String, String)>,
}

/// Bound on how deep the extends chain is followed -- a backstop against a
/// pathological or cyclic type graph (cycles are also caught by the visited
/// set). rafters config chains are one or two deep.
const MAX_EXTENDS_DEPTH: usize = 8;

/// Resolve a component's full Config prop surface from its `.behavior.ts`,
/// following the extends chain across files. Returns `None` when the module
/// declares no `*Config` interface.
pub fn resolve_config_interface(behavior_path: &Path) -> Result<Option<ResolvedConfig>, String> {
    let source = std::fs::read_to_string(behavior_path)
        .map_err(|error| format!("failed to read {}: {error}", behavior_path.display()))?;
    let module = parse_module(&source)?;

    let Some(primary) = module
        .interfaces
        .iter()
        .find(|interface| interface.name.ends_with("Config"))
    else {
        return Ok(None);
    };

    let mut props = primary.own_props.clone();
    let mut unresolved = Vec::new();
    let mut visited = HashSet::new();
    visited.insert(primary.name.clone());

    resolve_bases(
        &primary.extends,
        behavior_path,
        &module,
        &mut props,
        &mut unresolved,
        &mut visited,
        1,
    )?;

    Ok(Some(ResolvedConfig {
        name: primary.name.clone(),
        props,
        unresolved_extends: unresolved,
    }))
}

/// Append `from`'s properties that are not already present by name -- an own
/// or nearer property shadows an inherited one.
fn merge_props(into: &mut Vec<PropDoc>, from: &[PropDoc]) {
    for prop in from {
        if !into.iter().any(|existing| existing.name == prop.name) {
            into.push(prop.clone());
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_bases(
    bases: &[String],
    module_path: &Path,
    module: &ParsedModule,
    props: &mut Vec<PropDoc>,
    unresolved: &mut Vec<String>,
    visited: &mut HashSet<String>,
    depth: usize,
) -> Result<(), String> {
    if depth > MAX_EXTENDS_DEPTH {
        return Ok(());
    }
    for base in bases {
        if !visited.insert(base.clone()) {
            continue;
        }

        // A base declared in the same module -- merge and recurse in place.
        if let Some(local) = module.interfaces.iter().find(|iface| &iface.name == base) {
            merge_props(props, &local.own_props);
            let extends = local.extends.clone();
            resolve_bases(&extends, module_path, module, props, unresolved, visited, depth + 1)?;
            continue;
        }

        // Otherwise trace the base through this module's imports.
        let Some((_, specifier)) = module.imports.iter().find(|(name, _)| name == base) else {
            unresolved.push(base.clone());
            continue;
        };
        let Some(base_path) = resolve_module_path(module_path, specifier) else {
            unresolved.push(base.clone());
            continue;
        };
        let Ok(base_source) = std::fs::read_to_string(&base_path) else {
            unresolved.push(base.clone());
            continue;
        };
        let base_module = parse_module(&base_source)?;
        let Some(base_iface) = base_module.interfaces.iter().find(|iface| &iface.name == base)
        else {
            unresolved.push(base.clone());
            continue;
        };
        merge_props(props, &base_iface.own_props);
        let extends = base_iface.extends.clone();
        resolve_bases(
            &extends,
            &base_path,
            &base_module,
            props,
            unresolved,
            visited,
            depth + 1,
        )?;
    }
    Ok(())
}

/// Resolve a relative import specifier to a `.ts` file on disk, relative to
/// the importing file. External (bare) specifiers return `None` -- a
/// node_modules type is not veneer's to resolve. Tries `<spec>.ts` then
/// `<spec>/index.ts`.
fn resolve_module_path(from_file: &Path, specifier: &str) -> Option<PathBuf> {
    if !specifier.starts_with('.') && !specifier.starts_with('/') {
        return None;
    }
    let base_dir = from_file.parent()?;
    let joined = base_dir.join(specifier);

    let with_ts = joined.with_extension("ts");
    if with_ts.is_file() {
        return Some(with_ts);
    }
    let index = joined.join("index.ts");
    if index.is_file() {
        return Some(index);
    }
    None
}

/// Parse a module for every interface it declares and its named imports.
fn parse_module(source: &str) -> Result<ParsedModule, String> {
    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, source, SourceType::ts()).parse();
    if ret.panicked || !ret.errors.is_empty() {
        return Err("failed to parse a module while resolving the Config extends chain".into());
    }

    let mut interfaces = Vec::new();
    let mut imports = Vec::new();
    for statement in &ret.program.body {
        match statement {
            Statement::TSInterfaceDeclaration(interface) => {
                interfaces.push(config_from_interface(interface, source));
            }
            Statement::ExportNamedDeclaration(export) => {
                if let Some(Declaration::TSInterfaceDeclaration(interface)) = &export.declaration {
                    interfaces.push(config_from_interface(interface, source));
                }
            }
            Statement::ImportDeclaration(import) => {
                let specifier = import.source.value.as_str().to_string();
                if let Some(specifiers) = &import.specifiers {
                    for entry in specifiers {
                        if let ImportDeclarationSpecifier::ImportSpecifier(named) = entry {
                            imports.push((named.local.name.as_str().to_string(), specifier.clone()));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(ParsedModule {
        interfaces,
        imports,
    })
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

    #[test]
    fn resolves_inherited_props_across_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("base.ts"),
            "export interface BaseConfig { open?: boolean | undefined; defaultOpen?: boolean | undefined; }",
        )
        .expect("write base");
        let behavior = dir.path().join("widget.behavior.ts");
        std::fs::write(
            &behavior,
            "import type { BaseConfig } from './base';\nexport interface WidgetConfig extends BaseConfig { label: string; }",
        )
        .expect("write behavior");

        let resolved = resolve_config_interface(&behavior)
            .expect("resolves")
            .expect("has a Config");
        assert_eq!(resolved.name, "WidgetConfig");
        assert!(
            resolved.unresolved_extends.is_empty(),
            "BaseConfig resolves via the relative import"
        );
        let names: Vec<&str> = resolved.props.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["label", "open", "defaultOpen"], "own first, then inherited");
    }

    #[test]
    fn an_unresolvable_external_base_is_named_not_dropped() {
        let dir = tempfile::tempdir().expect("tempdir");
        let behavior = dir.path().join("widget.behavior.ts");
        std::fs::write(
            &behavior,
            "import type { ExternalConfig } from 'some-pkg';\nexport interface WidgetConfig extends ExternalConfig { label: string; }",
        )
        .expect("write behavior");

        let resolved = resolve_config_interface(&behavior)
            .expect("resolves")
            .expect("has a Config");
        let names: Vec<&str> = resolved.props.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["label"], "own props only; the external base is not read");
        assert_eq!(resolved.unresolved_extends, ["ExternalConfig"]);
    }

    #[test]
    fn a_subtype_prop_shadows_an_inherited_one_of_the_same_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("base.ts"),
            "export interface BaseConfig { disabled?: boolean; extra?: string; }",
        )
        .expect("write base");
        let behavior = dir.path().join("widget.behavior.ts");
        std::fs::write(
            &behavior,
            "import type { BaseConfig } from './base';\nexport interface WidgetConfig extends BaseConfig { disabled: boolean; }",
        )
        .expect("write behavior");

        let resolved = resolve_config_interface(&behavior).unwrap().unwrap();
        let disabled = resolved.props.iter().find(|p| p.name == "disabled").unwrap();
        assert!(!disabled.optional, "the subtype's required disabled wins over the base optional");
        assert_eq!(resolved.props.len(), 2, "disabled (own) + extra (inherited)");
    }

}
