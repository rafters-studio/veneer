//! Shadow-boundary isolation integration (FR-VEN-018).
//!
//! Drives the full pipeline over real-shaped fixtures: a `.classes.ts`
//! modeled verbatim on rafters `packages/ui/src/old/ui/badge.classes.ts`
//! and a stylesheet in the COMPILED shape previews actually adopt. Asserts
//! the isolation contract structurally on the generated JavaScript: an open
//! shadow root, CSS delivered only via `shadowRoot.adoptedStyleSheets` from
//! the shared runtime sheet, zero page-global style injection, and no
//! framework runtime.
//!
//! The contract changed with the move to a shared sheet: a preview module no
//! longer embeds its own CSS, so "self-contained" now means it imports
//! nothing but the veneer runtime. The property that matters -- one sheet
//! instance shared by every preview, live-updatable in place -- is a runtime
//! property and is verified in the browser harness, not here. These tests
//! hold the structural line that makes it possible.

use std::fs;
use std::path::PathBuf;

use veneer_adapters::{
    extract_classes_from_ts, uncovered_classes, checked_web_component_block, ComponentConventions,
    ComponentRegistry, ReactAdapter,
};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/shadow")
}

fn fixture(name: &str) -> String {
    let path = fixture_dir().join(name);
    match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => panic!("cannot read fixture {}: {error}", path.display()),
    }
}

fn badge_structure(ts: &str) -> veneer_adapters::ComponentStructure {
    let adapter = ReactAdapter::with_conventions(ComponentConventions::for_classes_file("badge"));
    match adapter.extract_structure(ts) {
        Ok(structure) => structure,
        Err(error) => panic!("badge.classes.ts must extract: {error}"),
    }
}

/// The structural isolation contract on a generated preview module.
fn assert_isolation_contract(js: &str) {
    // Open shadow root; CSS arrives only through the shared runtime sheet.
    assert!(js.contains("this.attachShadow({ mode: 'open' })"));
    assert!(js.contains("this.shadowRoot.adoptedStyleSheets = [sheet()"));

    // The module owns NO CSS: not utility rules, not a host shim. veneer
    // renders what rafters declares and authors no styling of its own.
    assert!(!js.contains("new CSSStyleSheet()"), "must own no stylesheet");
    assert!(!js.contains(".bg-primary"), "must not embed utility CSS");
    assert!(!js.contains(":host{"), "must not author host CSS");

    // Zero page-global style interaction: the module neither reads the host
    // page's stylesheets nor injects a <style>/<link> into it.
    assert!(!js.contains("document.styleSheets"));
    assert!(!js.contains("document.head"));
    assert!(!js.contains("<style"));
    assert!(!js.contains("<link"));
    assert!(!js.contains("createElement('style')"));
    assert!(!js.contains("createElement('link')"));

    // The ONLY import is the veneer runtime. No framework, nothing else.
    let imports: Vec<&str> = js.lines().filter(|line| line.starts_with("import ")).collect();
    assert_eq!(
        imports,
        vec!["import { sheet } from './veneer-runtime.js';"],
        "a preview module imports the runtime and nothing else"
    );
    assert!(!js.contains("require("));
    assert!(!js.to_lowercase().contains("react"));
}

#[test]
fn badge_preview_is_style_isolated_and_carries_no_embedded_css() {
    let ts = fixture("badge.classes.ts");
    let css = fixture("rafters.standalone.css");
    let structure = badge_structure(&ts);

    let block = match checked_web_component_block("badge-preview", &structure, &css) {
        Ok(block) => block,
        Err(error) => panic!("badge preview must render: {error}"),
    };

    assert_isolation_contract(&block.web_component);

    // The rules themselves live in the shared sheet, never in the module.
    assert!(!block.web_component.contains(".text-label-small"));
    assert!(!block.web_component.contains("--font-size-label-small"));
}

#[test]
fn the_runtime_module_shares_one_sheet_and_updates_it_in_place() {
    let runtime = veneer_adapters::generator::runtime_module();

    // Exactly one CSSStyleSheet is ever constructed: identity across every
    // adopting root is the property the live-preview contract rests on.
    assert_eq!(runtime.matches("new CSSStyleSheet()").count(), 1);

    // The update path mutates that instance rather than replacing it. A
    // fresh sheet would leave every already-connected shadow root styled by
    // the old one -- the failure this module exists to prevent.
    assert!(runtime.contains("target.replaceSync(css)"));
    assert!(!runtime.contains("sharedSheet = new CSSStyleSheet();\n  target"));
}

#[test]
fn classes_the_sheet_does_not_cover_are_reported_not_silent() {
    // Previews adopt the sheet whole, so an uncovered class cannot fail the
    // render -- it renders unstyled. The coverage check is the only thing
    // standing between that and a plausible-looking wrong preview.
    let css = fixture("rafters.standalone.css");

    let declared = vec![
        "bg-primary".to_string(),      // in the sheet
        "text-label-small".to_string(), // in the sheet
        "bg-invented-token".to_string(), // NOT in the sheet
    ];

    let uncovered = match uncovered_classes(&declared, &css) {
        Ok(uncovered) => uncovered,
        Err(error) => panic!("coverage check must run: {error}"),
    };

    assert_eq!(uncovered, vec!["bg-invented-token".to_string()]);
}

#[test]
fn a_compiled_variant_does_not_cover_the_bare_utility() {
    // The exact drift shape: the sheet carries `hover:bg-primary-hover`
    // compiled, but never a bare `bg-primary-hover`. Tailwind only emits
    // what it saw, so applying the bare class would render nothing.
    let css = fixture("rafters.standalone.css");

    let uncovered = uncovered_classes(&["bg-primary-hover".to_string()], &css).unwrap();
    assert_eq!(uncovered, vec!["bg-primary-hover".to_string()]);

    let covered = uncovered_classes(&["hover:bg-primary-hover".to_string()], &css).unwrap();
    assert!(covered.is_empty(), "the compiled variant is covered: {covered:?}");
}

#[test]
fn dynamically_composed_quality_classes_surface_as_a_pattern() {
    // Tree-shake caveat (bullpen 019f1f4d): `text-quality-${tint}` never
    // appears as a source literal, so extraction must surface it as a
    // pattern rather than losing it.
    let ts = fixture("quality-indicator.classes.ts");
    let classes = extract_classes_from_ts(&ts);

    assert!(
        classes.contains(&"text-quality-*".to_string()),
        "extraction must surface the dynamic composition as a pattern: {classes:?}"
    );
}

#[test]
fn dynamic_quality_classes_reach_the_generated_module_through_the_registry_pipeline() {
    // The wired pipeline end to end: registry scan (export discovery over
    // the .classes.ts source) -> component structure -> generated Web
    // Component adopting the shared sheet.
    let css = fixture("rafters.standalone.css");

    let mut registry = ComponentRegistry::new();
    let count = match registry.scan(&fixture_dir()) {
        Ok(count) => count,
        Err(error) => panic!("shadow fixtures must scan: {error}"),
    };
    assert!(count >= 2, "badge and quality-indicator must register");

    let block =
        match registry.generate_web_component("QualityIndicator", "quality-indicator-preview", &css)
        {
            Ok(block) => block,
            Err(error) => panic!("quality indicator preview must render: {error}"),
        };

    assert_isolation_contract(&block.web_component);
    assert!(
        block.classes_used.contains(&"text-quality-*".to_string()),
        "the dynamic composition must surface as a pattern: {:?}",
        block.classes_used
    );
}

#[test]
fn an_empty_stylesheet_names_the_component_instead_of_emitting_a_preview() {
    let ts = fixture("badge.classes.ts");
    let structure = badge_structure(&ts);

    // An empty sheet means the pipeline handed us nothing -- a pipeline
    // fault, not a component fault, and never a silent unstyled preview.
    let error = match checked_web_component_block("badge-preview", &structure, "   ") {
        Ok(_) => panic!("must not emit a preview against an empty stylesheet"),
        Err(error) => error,
    };

    let message = error.to_string();
    assert!(
        message.contains(&structure.name),
        "error must name the component: {message}"
    );
}
