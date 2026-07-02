//! Shadow-boundary isolation integration (FR-VEN-018).
//!
//! Drives the full pipeline over real-shaped fixtures: a `.classes.ts`
//! modeled verbatim on rafters `packages/ui/src/old/ui/badge.classes.ts`
//! and a stylesheet in the rafters design-tokens exporter output shape
//! (`@theme` plus `@utility` blocks). Asserts the isolation contract
//! structurally on the generated JavaScript: an open shadow root, component
//! CSS delivered only via `shadowRoot.adoptedStyleSheets` built from the
//! embedded scoped CSS, zero page-global style injection, and no framework
//! runtime. The pixel-level hostile-CSS comparison needs a browser harness,
//! which this repository does not have; these tests assert the structural
//! contract that guarantees it.

use std::fs;
use std::path::PathBuf;

use veneer_adapters::{
    extract_classes_from_ts, scoped_web_component_block, shadow_css_for_component,
    ComponentConventions, ReactAdapter,
};

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/shadow")
        .join(name);
    match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => panic!("cannot read fixture {}: {error}", path.display()),
    }
}

/// The structural isolation contract on a generated preview module.
fn assert_isolation_contract(js: &str) {
    // Open shadow root, CSS via adoptedStyleSheets from the embedded string.
    assert!(js.contains("this.attachShadow({ mode: 'open' })"));
    assert!(js.contains("this.shadowRoot.adoptedStyleSheets = [componentStyles()]"));
    assert!(js.contains("componentSheet.replaceSync(componentCss)"));

    // Zero page-global style interaction: the module neither reads the host
    // page's stylesheets nor injects a <style>/<link> into it.
    assert!(!js.contains("document.styleSheets"));
    assert!(!js.contains("document.head"));
    assert!(!js.contains("<style"));
    assert!(!js.contains("<link"));
    assert!(!js.contains("createElement('style')"));
    assert!(!js.contains("createElement('link')"));

    // No framework runtime on the host page: the module is self-contained.
    assert!(!js.contains("import "));
    assert!(!js.contains("require("));
    assert!(!js.to_lowercase().contains("react"));
}

#[test]
fn badge_preview_is_style_isolated_and_carries_its_own_scoped_css() {
    let ts = fixture("badge.classes.ts");
    let css = fixture("rafters.css");

    let adapter = ReactAdapter::with_conventions(ComponentConventions::for_classes_file("badge"));
    let structure = match adapter.extract_structure(&ts) {
        Ok(structure) => structure,
        Err(error) => panic!("badge.classes.ts must extract: {error}"),
    };

    let block = match scoped_web_component_block("badge-preview", &structure, &css) {
        Ok(block) => block,
        Err(error) => panic!("badge preview must render with scoped CSS: {error}"),
    };

    assert_isolation_contract(&block.web_component);

    // The scoped CSS the shadow root adopts contains the typography
    // composite utilities the badge sizes resolve to.
    assert!(block.web_component.contains(".text-label-small {"));
    assert!(block.web_component.contains(".text-label-medium {"));
    // Theme variables those utilities reference ride along as :host vars.
    assert!(block.web_component.contains(":host {"));
    assert!(block
        .web_component
        .contains("--font-size-label-small: 0.75rem;"));
    // Tailwind source at-rules never reach the browser sheet.
    assert!(!block.web_component.contains("@utility"));
    assert!(!block.web_component.contains("@theme"));
}

#[test]
fn badge_classes_outside_the_utility_layer_are_reported_not_silent() {
    // Reality: rafters semantic color classes (bg-primary, text-foreground)
    // are Tailwind theme-generated, not @utility blocks, so they cannot be
    // extracted from the source stylesheet. They must surface as unmatched
    // -- never vanish silently.
    let ts = fixture("badge.classes.ts");
    let css = fixture("rafters.css");

    let classes = extract_classes_from_ts(&ts);
    let shadow = match shadow_css_for_component("Badge", &classes, &css) {
        Ok(shadow) => shadow,
        Err(error) => panic!("badge classes partially match: {error}"),
    };

    assert!(shadow.unmatched.contains(&"bg-primary".to_string()));
    assert!(shadow.css.contains(".text-label-small {"));
}

#[test]
fn dynamically_composed_quality_classes_reach_the_scoped_css() {
    // Tree-shake caveat (bullpen 019f1f4d): `text-quality-${tint}` never
    // appears as a source literal, yet the scoped CSS must contain every
    // class the component resolves to at render.
    let ts = fixture("quality-indicator.classes.ts");
    let css = fixture("rafters.css");

    let classes = extract_classes_from_ts(&ts);
    assert!(
        classes.contains(&"text-quality-*".to_string()),
        "extraction must surface the dynamic composition as a pattern: {classes:?}"
    );

    let shadow = match shadow_css_for_component("QualityIndicator", &classes, &css) {
        Ok(shadow) => shadow,
        Err(error) => panic!("quality pattern matches the tint utilities: {error}"),
    };

    assert!(shadow.css.contains(".text-quality-500 {"));
    assert!(shadow.css.contains(".text-quality-600 {"));
    assert!(shadow
        .css
        .contains("--color-quality-600: oklch(0.55 0.14 140);"));
}

#[test]
fn extraction_failure_names_the_component_instead_of_emitting_unstyled_preview() {
    let ts = fixture("badge.classes.ts");

    let adapter = ReactAdapter::with_conventions(ComponentConventions::for_classes_file("badge"));
    let structure = match adapter.extract_structure(&ts) {
        Ok(structure) => structure,
        Err(error) => panic!("badge.classes.ts must extract: {error}"),
    };

    // A stylesheet with no matching rules must refuse to render, naming the
    // component, rather than emit a preview silently missing its styles.
    let error = match scoped_web_component_block(
        "badge-preview",
        &structure,
        "@utility unrelated {\n  color: red;\n}\n",
    ) {
        Ok(_) => panic!("must not emit a preview with no styles"),
        Err(error) => error,
    };

    let message = error.to_string();
    assert!(
        message.contains(&structure.name),
        "error must name the component: {message}"
    );
}
