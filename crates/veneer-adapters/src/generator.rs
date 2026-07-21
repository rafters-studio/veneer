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
}
