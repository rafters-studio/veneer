//! Web Component code generator.
//!
//! Every generated custom element is style-isolated (FR-VEN-018): it
//! attaches an open shadow root and delivers its component CSS through
//! `shadowRoot.adoptedStyleSheets`, built from a scoped CSS string embedded
//! in the module itself. The generated code never injects a page-global
//! `<style>` or `<link>` and never reads the host page's stylesheets, so a
//! preview neither leaks styles into the host page nor absorbs conflicting
//! global CSS from it.

use crate::react::ComponentStructure;
use crate::scope::shadow_css_for_component;
use crate::traits::{TransformError, TransformedBlock};

/// Assemble the full transform result for a component structure: the
/// generated Web Component plus the classes and attributes the structure
/// declares. The single assembly point for structure-based previews.
///
/// `scoped_css` is the browser-ready CSS for the component's shadow root,
/// already extracted by [`shadow_css_for_component`]. Production callers
/// hold a full stylesheet, not extracted CSS, and go through
/// [`scoped_web_component_block`], which performs that extraction and
/// enforces the no-silently-missing-styles contract (FR-VEN-018).
pub fn web_component_block(
    tag_name: &str,
    structure: &ComponentStructure,
    scoped_css: &str,
) -> TransformedBlock {
    TransformedBlock {
        web_component: generate_web_component(tag_name, structure, scoped_css),
        tag_name: tag_name.to_string(),
        classes_used: structure.collect_all_classes(),
        attributes: structure.observed_attributes.clone(),
    }
}

/// Assemble a transform result with CSS scoped from the full project
/// stylesheet. Extraction failure is a [`TransformError::RenderFailed`]
/// naming the component — a preview is never emitted silently missing its
/// styles. Classes with no matching rule are warned about individually.
pub fn scoped_web_component_block(
    tag_name: &str,
    structure: &ComponentStructure,
    full_css: &str,
) -> Result<TransformedBlock, TransformError> {
    let classes = structure.collect_all_classes();
    let shadow =
        shadow_css_for_component(&structure.name, &classes, full_css).map_err(|error| {
            TransformError::RenderFailed {
                component: structure.name.clone(),
                reason: error.to_string(),
            }
        })?;

    for class in &shadow.unmatched {
        eprintln!(
            "veneer/scoped_web_component_block: component '{}': no CSS rule found for '{class}'",
            structure.name
        );
    }

    Ok(web_component_block(tag_name, structure, &shadow.css))
}

/// Generate the shared JS prelude that turns the embedded scoped CSS into a
/// lazily-constructed `CSSStyleSheet` for `shadowRoot.adoptedStyleSheets`.
/// Construction is deferred to first connect so the module also loads in
/// environments without constructable stylesheets (for example SSR).
fn stylesheet_js(scoped_css: &str) -> String {
    format!(
        r#"const componentCss = '{css}';

let componentSheet = null;
function componentStyles() {{
  if (componentSheet === null) {{
    componentSheet = new CSSStyleSheet();
    componentSheet.replaceSync(componentCss);
  }}
  return componentSheet;
}}"#,
        css = escape_string(scoped_css),
    )
}

/// Generate a Web Component class from the extracted component structure.
/// Component CSS is delivered via `shadowRoot.adoptedStyleSheets` from the
/// embedded `scoped_css`; no page-global style is read or injected.
pub fn generate_web_component(
    tag_name: &str,
    structure: &ComponentStructure,
    scoped_css: &str,
) -> String {
    let class_name = to_pascal_case(tag_name);

    let variant_entries: String = structure
        .variant_lookup
        .iter()
        .map(|(k, v)| format!("  {}: '{}',", k, escape_string(v)))
        .collect::<Vec<_>>()
        .join("\n");

    let size_entries: String = structure
        .size_lookup
        .iter()
        .map(|(k, v)| format!("  {}: '{}',", k, escape_string(v)))
        .collect::<Vec<_>>()
        .join("\n");

    let attrs_array: String = structure
        .observed_attributes
        .iter()
        .map(|a| format!("'{}'", a))
        .collect::<Vec<_>>()
        .join(", ");

    let default_variant = &structure.default_variant;
    let default_size = &structure.default_size;
    let base_classes = escape_string(&structure.base_classes);
    let disabled_classes = escape_string(&structure.disabled_classes);

    format!(
        r#"/**
 * {class_name} - Generated Web Component Preview
 * Auto-generated from {name} component
 * Tag: <{tag_name}>
 */

{stylesheet_js}

const variantClasses = {{
{variant_entries}
}};

const sizeClasses = {{
{size_entries}
}};

const baseClasses = '{base_classes}';
const disabledClasses = '{disabled_classes}';

export class {class_name} extends HTMLElement {{
  static observedAttributes = [{attrs_array}];

  #button = null;

  constructor() {{
    super();
    this.attachShadow({{ mode: 'open' }});
  }}

  connectedCallback() {{
    this.shadowRoot.adoptedStyleSheets = [componentStyles()];
    this.#render();
  }}

  attributeChangedCallback() {{
    this.#render();
  }}

  #render() {{
    if (!this.shadowRoot) return;

    const variant = this.getAttribute('variant') || '{default_variant}';
    const size = this.getAttribute('size') || '{default_size}';
    const disabled = this.hasAttribute('disabled');
    const loading = this.hasAttribute('loading');

    const isDisabled = disabled || loading;

    const classes = [
      baseClasses,
      variantClasses[variant] ?? variantClasses['{default_variant}'],
      sizeClasses[size] ?? sizeClasses['{default_size}'],
      isDisabled ? disabledClasses : '',
    ]
      .filter(Boolean)
      .join(' ');

    // Clear existing button if any
    if (this.#button) {{
      this.#button.remove();
    }}

    this.#button = document.createElement('button');
    this.#button.type = 'button';
    this.#button.className = classes;
    this.#button.disabled = isDisabled;

    if (isDisabled) {{
      this.#button.setAttribute('aria-disabled', 'true');
    }}
    if (loading) {{
      this.#button.setAttribute('aria-busy', 'true');
    }}

    if (loading) {{
      const span = document.createElement('span');
      span.setAttribute('aria-hidden', 'true');
      span.textContent = 'Loading...';
      this.#button.appendChild(span);
    }} else {{
      // Use slot for content
      const slot = document.createElement('slot');
      this.#button.appendChild(slot);
    }}

    this.shadowRoot.appendChild(this.#button);
  }}
}}

// Register the custom element
if (typeof customElements !== 'undefined') {{
  customElements.define('{tag_name}', {class_name});
}}

export default {class_name};
"#,
        class_name = class_name,
        name = structure.name,
        tag_name = tag_name,
        stylesheet_js = stylesheet_js(scoped_css),
        variant_entries = variant_entries,
        size_entries = size_entries,
        base_classes = base_classes,
        disabled_classes = disabled_classes,
        attrs_array = attrs_array,
        default_variant = default_variant,
        default_size = default_size,
    )
}

/// Generate a passthrough Web Component that renders a slot with adopted styles.
///
/// Used for compound/structural components (Card, Accordion, Dialog, etc.) that
/// don't have variant/size switching but still need style isolation for previews.
/// The component renders its light DOM children inside a shadow root whose
/// `adoptedStyleSheets` carry the embedded `scoped_css` — never page styles.
pub fn generate_passthrough_web_component(tag_name: &str, scoped_css: &str) -> String {
    let class_name = to_pascal_case(tag_name);

    format!(
        r#"/**
 * {class_name} - Passthrough Web Component Preview
 * Auto-generated for static component preview
 * Tag: <{tag_name}>
 */

{stylesheet_js}

export class {class_name} extends HTMLElement {{
  constructor() {{
    super();
    this.attachShadow({{ mode: 'open' }});
  }}

  connectedCallback() {{
    this.shadowRoot.adoptedStyleSheets = [componentStyles()];
    this.#render();
  }}

  #render() {{
    if (!this.shadowRoot) return;
    const slot = document.createElement('slot');
    this.shadowRoot.appendChild(slot);
  }}
}}

if (typeof customElements !== 'undefined') {{
  customElements.define('{tag_name}', {class_name});
}}

export default {class_name};
"#,
        class_name = class_name,
        tag_name = tag_name,
        stylesheet_js = stylesheet_js(scoped_css),
    )
}

/// Convert kebab-case to PascalCase.
fn to_pascal_case(s: &str) -> String {
    s.split('-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// Escape a string for JavaScript output.
fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
}

/// Generate an interactive controls panel for a live preview component.
///
/// Returns an empty string if the component has no controllable attributes
/// (no variants, no sizes, no boolean observed attributes).
///
/// The controls panel is plain HTML + inline JS (not a Web Component).
/// It manipulates the preview component via DOM attribute manipulation.
pub fn generate_controls_panel(tag_name: &str, structure: &ComponentStructure) -> String {
    // Determine which boolean attributes get checkboxes (skip variant/size since they have dropdowns)
    let boolean_attrs: Vec<&str> = structure
        .observed_attributes
        .iter()
        .filter(|a| *a != "variant" && *a != "size")
        .map(|a| a.as_str())
        .collect();

    let has_variants = !structure.variant_lookup.is_empty();
    let has_sizes = !structure.size_lookup.is_empty();
    let has_booleans = !boolean_attrs.is_empty();

    // Return empty string if nothing to control
    if !has_variants && !has_sizes && !has_booleans {
        return String::new();
    }

    let mut html = String::new();

    html.push_str(&format!(
        r#"<div class="veneer-controls" data-veneer-controls-for="{tag_name}">"#,
    ));

    // Variant select
    if has_variants {
        html.push_str(r#"<label class="veneer-controls-field">"#);
        html.push_str(r#"<span class="veneer-controls-label">Variant</span>"#);
        html.push_str(r#"<select class="veneer-controls-select" data-veneer-attr="variant">"#);
        for (key, _) in &structure.variant_lookup {
            let selected = if *key == structure.default_variant {
                " selected"
            } else {
                ""
            };
            html.push_str(&format!(
                r#"<option value="{key}"{selected}>{key}</option>"#,
            ));
        }
        html.push_str("</select>");
        html.push_str("</label>");
    }

    // Size select
    if has_sizes {
        html.push_str(r#"<label class="veneer-controls-field">"#);
        html.push_str(r#"<span class="veneer-controls-label">Size</span>"#);
        html.push_str(r#"<select class="veneer-controls-select" data-veneer-attr="size">"#);
        for (key, _) in &structure.size_lookup {
            let selected = if *key == structure.default_size {
                " selected"
            } else {
                ""
            };
            html.push_str(&format!(
                r#"<option value="{key}"{selected}>{key}</option>"#,
            ));
        }
        html.push_str("</select>");
        html.push_str("</label>");
    }

    // Boolean checkboxes
    for attr in &boolean_attrs {
        html.push_str(r#"<label class="veneer-controls-field veneer-controls-checkbox">"#);
        html.push_str(&format!(
            r#"<input type="checkbox" data-veneer-attr="{attr}">"#,
        ));
        html.push_str(&format!(
            r#"<span class="veneer-controls-label">{}</span>"#,
            capitalize_first(attr),
        ));
        html.push_str("</label>");
    }

    // Inline JS for wiring up controls
    html.push_str(&format!(
        r#"<script>
(function() {{
  var controls = document.currentScript.parentElement;
  var preview = controls.previousElementSibling;
  while (preview && !preview.querySelector('{tag_name}')) {{
    preview = preview.previousElementSibling;
  }}
  var target = preview ? preview.querySelector('{tag_name}') : null;
  if (!target) return;

  controls.querySelectorAll('select[data-veneer-attr]').forEach(function(sel) {{
    sel.addEventListener('change', function() {{
      target.setAttribute(sel.dataset.veneerAttr, sel.value);
    }});
  }});

  controls.querySelectorAll('input[type="checkbox"][data-veneer-attr]').forEach(function(cb) {{
    cb.addEventListener('change', function() {{
      if (cb.checked) {{
        target.setAttribute(cb.dataset.veneerAttr, '');
      }} else {{
        target.removeAttribute(cb.dataset.veneerAttr);
      }}
    }});
  }});
}})();
</script>"#,
    ));

    html.push_str("</div>");

    html
}

/// Capitalize the first letter of a string (for display labels).
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_pascal_case_works() {
        assert_eq!(to_pascal_case("button-preview"), "ButtonPreview");
        assert_eq!(to_pascal_case("my-component"), "MyComponent");
        assert_eq!(to_pascal_case("simple"), "Simple");
    }

    #[test]
    fn escape_string_works() {
        assert_eq!(escape_string("hello"), "hello");
        assert_eq!(escape_string("it's"), "it\\'s");
        assert_eq!(escape_string("line1\nline2"), "line1\\nline2");
    }

    #[test]
    fn generates_valid_web_component() {
        let structure = ComponentStructure {
            name: "Button".to_string(),
            variant_lookup: vec![
                (
                    "primary".to_string(),
                    "bg-primary text-primary-foreground".to_string(),
                ),
                (
                    "secondary".to_string(),
                    "bg-secondary text-secondary-foreground".to_string(),
                ),
            ],
            size_lookup: vec![
                ("sm".to_string(), "h-8 px-3".to_string()),
                ("md".to_string(), "h-10 px-4".to_string()),
            ],
            base_classes: "inline-flex items-center".to_string(),
            disabled_classes: "opacity-50".to_string(),
            default_variant: "primary".to_string(),
            default_size: "md".to_string(),
            observed_attributes: vec!["variant".to_string(), "size".to_string()],
            dynamic_class_patterns: vec![],
        };

        let output = generate_web_component("my-button", &structure, ".bg-primary {\n}");

        assert!(output.contains("class MyButton extends HTMLElement"));
        assert!(output.contains("static observedAttributes"));
        assert!(output.contains("customElements.define('my-button'"));
        assert!(output.contains("bg-primary"));
        assert!(output.contains("adoptedStyleSheets"));
    }

    const SCOPED_CSS: &str = ":host {\n  --color-primary: oklch(0.645 0.12 180);\n}\n\n.bg-primary {\n  background-color: var(--color-primary);\n}";

    /// The isolation contract, asserted structurally on the generated JS:
    /// component CSS enters only through the shadow root, never the page.
    fn assert_style_isolated(js: &str) {
        // Open shadow root.
        assert!(
            js.contains("this.attachShadow({ mode: 'open' })"),
            "must attach an open shadow root"
        );
        // CSS delivered via adoptedStyleSheets on the shadow root.
        assert!(
            js.contains("this.shadowRoot.adoptedStyleSheets = [componentStyles()]"),
            "must adopt component styles onto the shadow root"
        );
        assert!(
            js.contains("componentSheet.replaceSync(componentCss)"),
            "must build the sheet from the embedded scoped CSS"
        );
        // Zero page-global style interaction: nothing read from or written
        // to the host document's styles.
        assert!(!js.contains("document.styleSheets"));
        assert!(!js.contains("document.head"));
        assert!(!js.contains("document.adoptedStyleSheets"));
        assert!(!js.contains("<style"));
        assert!(!js.contains("<link"));
        assert!(!js.contains("createElement('style')"));
        assert!(!js.contains("createElement('link')"));
        assert!(!js.contains("data-veneer-component"));
        // No framework runtime.
        assert!(!js.contains("import "));
        assert!(!js.contains("require("));
        assert!(!js.to_lowercase().contains("react"));
    }

    #[test]
    fn web_component_embeds_scoped_css_and_isolates_styles() {
        let structure = make_full_structure();
        let output = generate_web_component("button-preview", &structure, SCOPED_CSS);

        assert_style_isolated(&output);
        // The scoped CSS itself is embedded in the module (JS-escaped).
        assert!(output.contains("background-color: var(--color-primary);"));
        assert!(output.contains(":host {"));
    }

    #[test]
    fn passthrough_web_component_embeds_scoped_css_and_isolates_styles() {
        let output = generate_passthrough_web_component("card-preview", SCOPED_CSS);

        assert_style_isolated(&output);
        assert!(output.contains("background-color: var(--color-primary);"));
        assert!(output.contains("customElements.define('card-preview'"));
    }

    #[test]
    fn web_component_block_carries_scoped_css() {
        let structure = make_full_structure();
        let block = web_component_block("button-preview", &structure, SCOPED_CSS);

        assert_style_isolated(&block.web_component);
        assert_eq!(block.tag_name, "button-preview");
        assert!(block.classes_used.contains(&"bg-primary".to_string()));
    }

    const FULL_CSS: &str = r#"
@theme {
  --color-primary: oklch(0.645 0.12 180);
}

@utility bg-primary {
  background-color: var(--color-primary);
}

@utility h-8 {
  height: 2rem;
}
"#;

    #[test]
    fn scoped_block_resolves_css_from_full_stylesheet() {
        let structure = make_full_structure();
        let block = scoped_web_component_block("button-preview", &structure, FULL_CSS)
            .expect("classes match rules in the stylesheet");

        assert_style_isolated(&block.web_component);
        // The @utility block arrives as a plain class rule, shadow-adoptable.
        assert!(block
            .web_component
            .contains("background-color: var(--color-primary);"));
        assert!(!block.web_component.contains("@utility"));
    }

    #[test]
    fn scoped_block_errors_naming_component_when_no_css_matches() {
        let structure = make_full_structure();
        let result = scoped_web_component_block(
            "button-preview",
            &structure,
            "@utility unrelated {\n  color: red;\n}\n",
        );

        let error = result.expect_err("no class matches any rule");
        let message = error.to_string();
        assert!(
            message.contains("Button"),
            "error must name the component: {message}"
        );
        assert!(matches!(
            error,
            TransformError::RenderFailed { ref component, .. } if component == "Button"
        ));
    }

    #[test]
    fn scoped_block_errors_naming_component_on_empty_stylesheet() {
        let structure = make_full_structure();
        let error = scoped_web_component_block("button-preview", &structure, "")
            .expect_err("empty stylesheet with classes requested");
        assert!(error.to_string().contains("Button"));
    }

    fn make_full_structure() -> ComponentStructure {
        ComponentStructure {
            name: "Button".to_string(),
            variant_lookup: vec![
                ("primary".to_string(), "bg-primary".to_string()),
                ("secondary".to_string(), "bg-secondary".to_string()),
                ("destructive".to_string(), "bg-destructive".to_string()),
            ],
            size_lookup: vec![
                ("sm".to_string(), "h-8 px-3".to_string()),
                ("md".to_string(), "h-10 px-4".to_string()),
                ("lg".to_string(), "h-12 px-6".to_string()),
            ],
            base_classes: "inline-flex items-center".to_string(),
            disabled_classes: "opacity-50".to_string(),
            default_variant: "primary".to_string(),
            default_size: "md".to_string(),
            observed_attributes: vec![
                "variant".to_string(),
                "size".to_string(),
                "disabled".to_string(),
                "loading".to_string(),
            ],
            dynamic_class_patterns: vec![],
        }
    }

    #[test]
    fn controls_contain_variant_select_with_all_options() {
        let structure = make_full_structure();
        let output = generate_controls_panel("button-preview", &structure);

        assert!(output.contains(r#"data-veneer-attr="variant""#));
        assert!(output.contains(r#"<option value="primary""#));
        assert!(output.contains(r#"<option value="secondary""#));
        assert!(output.contains(r#"<option value="destructive""#));
    }

    #[test]
    fn controls_contain_size_select_with_all_options() {
        let structure = make_full_structure();
        let output = generate_controls_panel("button-preview", &structure);

        assert!(output.contains(r#"data-veneer-attr="size""#));
        assert!(output.contains(r#"<option value="sm""#));
        assert!(output.contains(r#"<option value="md""#));
        assert!(output.contains(r#"<option value="lg""#));
    }

    #[test]
    fn controls_contain_boolean_checkboxes() {
        let structure = make_full_structure();
        let output = generate_controls_panel("button-preview", &structure);

        // Checkboxes for disabled and loading
        assert!(output.contains(r#"type="checkbox" data-veneer-attr="disabled""#));
        assert!(output.contains(r#"type="checkbox" data-veneer-attr="loading""#));

        // Labels should be capitalized
        assert!(output.contains("Disabled"));
        assert!(output.contains("Loading"));
    }

    #[test]
    fn controls_skip_variant_and_size_from_checkboxes() {
        let structure = make_full_structure();
        let output = generate_controls_panel("button-preview", &structure);

        // variant and size should NOT appear as checkboxes
        assert!(!output.contains(r#"type="checkbox" data-veneer-attr="variant""#));
        assert!(!output.contains(r#"type="checkbox" data-veneer-attr="size""#));
    }

    #[test]
    fn controls_empty_for_no_controllable_attributes() {
        let structure = ComponentStructure {
            name: "Plain".to_string(),
            variant_lookup: vec![],
            size_lookup: vec![],
            base_classes: String::new(),
            disabled_classes: String::new(),
            default_variant: String::new(),
            default_size: String::new(),
            observed_attributes: vec![],
            dynamic_class_patterns: vec![],
        };

        let output = generate_controls_panel("plain-preview", &structure);
        assert!(output.is_empty());
    }

    #[test]
    fn controls_empty_when_only_variant_and_size_in_attributes() {
        // If observed_attributes only contains "variant" and "size" but
        // variant_lookup and size_lookup are empty, there are no controls
        let structure = ComponentStructure {
            name: "Minimal".to_string(),
            variant_lookup: vec![],
            size_lookup: vec![],
            base_classes: String::new(),
            disabled_classes: String::new(),
            default_variant: String::new(),
            default_size: String::new(),
            observed_attributes: vec!["variant".to_string(), "size".to_string()],
            dynamic_class_patterns: vec![],
        };

        let output = generate_controls_panel("minimal-preview", &structure);
        assert!(output.is_empty());
    }

    #[test]
    fn controls_default_variant_is_preselected() {
        let structure = make_full_structure();
        let output = generate_controls_panel("button-preview", &structure);

        // "primary" is the default variant and should be selected
        assert!(output.contains(r#"<option value="primary" selected>primary</option>"#));
        // "secondary" should NOT be selected
        assert!(output.contains(r#"<option value="secondary">secondary</option>"#));
    }

    #[test]
    fn controls_default_size_is_preselected() {
        let structure = make_full_structure();
        let output = generate_controls_panel("button-preview", &structure);

        // "md" is the default size and should be selected
        assert!(output.contains(r#"<option value="md" selected>md</option>"#));
        // "sm" should NOT be selected
        assert!(output.contains(r#"<option value="sm">sm</option>"#));
    }

    #[test]
    fn controls_data_attribute_matches_tag_name() {
        let structure = make_full_structure();
        let output = generate_controls_panel("my-button-preview", &structure);

        assert!(output.contains(r#"data-veneer-controls-for="my-button-preview""#));
    }

    #[test]
    fn controls_contain_script_targeting_tag_name() {
        let structure = make_full_structure();
        let output = generate_controls_panel("button-preview", &structure);

        assert!(output.contains("<script>"));
        assert!(output.contains("querySelector('button-preview')"));
    }

    #[test]
    fn controls_only_boolean_attrs_no_selects() {
        // Component with no variants/sizes but with boolean attributes
        let structure = ComponentStructure {
            name: "Toggle".to_string(),
            variant_lookup: vec![],
            size_lookup: vec![],
            base_classes: String::new(),
            disabled_classes: String::new(),
            default_variant: String::new(),
            default_size: String::new(),
            observed_attributes: vec!["disabled".to_string(), "checked".to_string()],
            dynamic_class_patterns: vec![],
        };

        let output = generate_controls_panel("toggle-preview", &structure);

        // Should have checkboxes but no selects
        assert!(!output.is_empty());
        assert!(output.contains(r#"type="checkbox""#));
        assert!(!output.contains("<select"));
    }

    #[test]
    fn controls_only_variant_select_no_size() {
        let structure = ComponentStructure {
            name: "Badge".to_string(),
            variant_lookup: vec![
                ("default".to_string(), "bg-default".to_string()),
                ("info".to_string(), "bg-info".to_string()),
            ],
            size_lookup: vec![],
            base_classes: String::new(),
            disabled_classes: String::new(),
            default_variant: "default".to_string(),
            default_size: String::new(),
            observed_attributes: vec!["variant".to_string()],
            dynamic_class_patterns: vec![],
        };

        let output = generate_controls_panel("badge-preview", &structure);

        // Should have variant select but no size select
        assert!(output.contains(r#"data-veneer-attr="variant""#));
        assert!(!output.contains(r#"data-veneer-attr="size""#));
    }
}
