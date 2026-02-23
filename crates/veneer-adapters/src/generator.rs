//! Web Component code generator.

use crate::react::ComponentStructure;

/// Generate a Web Component class from the extracted component structure.
/// Uses adoptedStyleSheets to inherit page-level Tailwind CSS.
pub fn generate_web_component(tag_name: &str, structure: &ComponentStructure) -> String {
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

const variantClasses = {{
{variant_entries}
}};

const sizeClasses = {{
{size_entries}
}};

const baseClasses = '{base_classes}';
const disabledClasses = '{disabled_classes}';

// Cache for adopted stylesheets (all page stylesheets)
let cachedSheets = null;

export class {class_name} extends HTMLElement {{
  static observedAttributes = [{attrs_array}];

  #button = null;

  constructor() {{
    super();
    this.attachShadow({{ mode: 'open' }});
  }}

  connectedCallback() {{
    this.#adoptStyles();
    this.#render();
  }}

  attributeChangedCallback() {{
    this.#render();
  }}

  #adoptStyles() {{
    if (!this.shadowRoot) return;

    // Use cached sheets if available
    if (cachedSheets) {{
      this.shadowRoot.adoptedStyleSheets = cachedSheets;
      return;
    }}

    // Find and adopt page stylesheets
    const sheets = [];
    for (const sheet of document.styleSheets) {{
      try {{
        // Clone the stylesheet for adoption
        const clone = new CSSStyleSheet();
        const rules = Array.from(sheet.cssRules).map(r => r.cssText).join('\\n');
        clone.replaceSync(rules);
        sheets.push(clone);
      }} catch (e) {{
        // Cross-origin stylesheets can't be accessed, skip them
      }}
    }}

    if (sheets.length > 0) {{
      cachedSheets = sheets; // Cache all sheets
      this.shadowRoot.adoptedStyleSheets = sheets;
    }}
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
        variant_entries = variant_entries,
        size_entries = size_entries,
        base_classes = base_classes,
        disabled_classes = disabled_classes,
        attrs_array = attrs_array,
        default_variant = default_variant,
        default_size = default_size,
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
        html.push_str(
            r#"<select class="veneer-controls-select" data-veneer-attr="variant">"#,
        );
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
        html.push_str(
            r#"<select class="veneer-controls-select" data-veneer-attr="size">"#,
        );
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
        };

        let output = generate_web_component("my-button", &structure);

        assert!(output.contains("class MyButton extends HTMLElement"));
        assert!(output.contains("static observedAttributes"));
        assert!(output.contains("customElements.define('my-button'"));
        assert!(output.contains("bg-primary"));
        assert!(output.contains("adoptedStyleSheets"));
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
        };

        let output = generate_controls_panel("badge-preview", &structure);

        // Should have variant select but no size select
        assert!(output.contains(r#"data-veneer-attr="variant""#));
        assert!(!output.contains(r#"data-veneer-attr="size""#));
    }
}
