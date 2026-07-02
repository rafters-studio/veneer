//! Web Component code generator.

use crate::intelligence::{Constraint, ConstraintKind, RenderedComponent};
use crate::react::ComponentStructure;
use crate::traits::{TransformError, TransformedBlock};

/// Assemble the full transform result for a component structure: the
/// generated Web Component plus the classes and attributes the structure
/// declares. The single assembly point for structure-based previews.
pub fn web_component_block(tag_name: &str, structure: &ComponentStructure) -> TransformedBlock {
    TransformedBlock {
        web_component: generate_web_component(tag_name, structure),
        tag_name: tag_name.to_string(),
        classes_used: structure.collect_all_classes(),
        attributes: structure.observed_attributes.clone(),
    }
}

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

    if (!this.constructor._sheets) {{
      this.constructor._sheets = [...document.styleSheets]
        .filter(s => s.ownerNode?.hasAttribute('data-veneer-component'))
        .map(s => {{
          try {{
            const clone = new CSSStyleSheet();
            const rules = Array.from(s.cssRules).map(r => r.cssText).join('\\n');
            clone.replaceSync(rules);
            return clone;
          }} catch (e) {{
            return null;
          }}
        }})
        .filter(Boolean);
    }}
    this.shadowRoot.adoptedStyleSheets = this.constructor._sheets;
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

/// Generate a passthrough Web Component that renders a slot with adopted styles.
///
/// Used for compound/structural components (Card, Accordion, Dialog, etc.) that
/// don't have variant/size switching but still need style adoption for previews.
/// The component simply renders its light DOM children inside a shadow root with
/// adopted page-level styles.
pub fn generate_passthrough_web_component(tag_name: &str) -> String {
    let class_name = to_pascal_case(tag_name);

    format!(
        r#"/**
 * {class_name} - Passthrough Web Component Preview
 * Auto-generated for static component preview
 * Tag: <{tag_name}>
 */

export class {class_name} extends HTMLElement {{
  constructor() {{
    super();
    this.attachShadow({{ mode: 'open' }});
  }}

  connectedCallback() {{
    this.#adoptStyles();
    this.#render();
  }}

  #adoptStyles() {{
    if (!this.shadowRoot) return;

    if (!this.constructor._sheets) {{
      this.constructor._sheets = [...document.styleSheets]
        .filter(s => s.ownerNode?.hasAttribute('data-veneer-component'))
        .map(s => {{
          try {{
            const clone = new CSSStyleSheet();
            const rules = Array.from(s.cssRules).map(r => r.cssText).join('\\n');
            clone.replaceSync(rules);
            return clone;
          }} catch (e) {{
            return null;
          }}
        }})
        .filter(Boolean);
    }}
    this.shadowRoot.adoptedStyleSheets = this.constructor._sheets;
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

/// Render the constraints region of a preview surface from the compiled
/// DO/NEVER constraints (FR-VEN-004).
///
/// Constraint text is emitted verbatim from source -- HTML-escaped for
/// markup safety, never reworded -- in source order. An empty slice yields
/// an empty string: a component with no do/never in source shows no
/// constraints region at all, absent rather than empty-invented.
///
/// A constraint whose text is blank is an unparseable rule (for example a
/// bare `DO:` line or an empty `@never` tag in the source JSDoc): the error
/// names the component and the field instead of silently rendering a
/// partial rule.
pub fn generate_constraints_region(
    component_name: &str,
    constraints: &[Constraint],
) -> Result<String, TransformError> {
    if constraints.is_empty() {
        return Ok(String::new());
    }

    let mut items = String::new();
    for constraint in constraints {
        let (field, label) = match constraint.kind {
            ConstraintKind::Do => ("do", "DO"),
            ConstraintKind::Never => ("never", "NEVER"),
        };
        if constraint.text.trim().is_empty() {
            return Err(TransformError::RenderFailed {
                component: component_name.to_string(),
                reason: format!("unparseable {field} constraint: empty rule text"),
            });
        }
        items.push_str(&format!(
            "<li class=\"veneer-constraint veneer-constraint-{field}\">\
<span class=\"veneer-constraint-kind\">{label}</span> \
<span class=\"veneer-constraint-text\">{text}</span></li>\n",
            text = escape_html(&constraint.text),
        ));
    }

    Ok(format!(
        "<section class=\"veneer-constraints\" aria-label=\"Usage constraints\">\n\
<h3 class=\"veneer-constraints-heading\">Constraints</h3>\n\
<ul class=\"veneer-constraints-list\">\n{items}</ul>\n</section>"
    ))
}

/// Assemble the preview surface for a rendered component: the Web Component
/// definition, the preview element, and the constraints region, all inside
/// one page section. The constraints sit beside the preview at the point of
/// decision (FR-VEN-004) -- never behind a link or in a separate document.
pub fn generate_preview_surface(
    component_name: &str,
    rendered: &RenderedComponent,
) -> Result<String, TransformError> {
    let constraints_region =
        generate_constraints_region(component_name, &rendered.intelligence.do_never)?;
    let tag_name = &rendered.preview.tag_name;
    let web_component = &rendered.preview.web_component;

    let mut surface = format!(
        "<section class=\"veneer-preview-surface\" data-veneer-surface-for=\"{tag_name}\">\n\
<script type=\"module\">\n{web_component}</script>\n\
<div class=\"veneer-preview\"><{tag_name}></{tag_name}></div>\n"
    );
    if !constraints_region.is_empty() {
        surface.push_str(&constraints_region);
        surface.push('\n');
    }
    surface.push_str("</section>\n");
    Ok(surface)
}

/// Escape text for use inside an HTML text node. The visible text stays
/// verbatim; only the markup-significant characters are encoded.
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
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

    // ---- constraints region and preview surface (FR-VEN-004) ----

    use crate::intelligence::render_component;
    use crate::rafters_source::read_rafters_namespace;
    use crate::registry::ComponentRegistry;
    use std::path::Path;

    /// Render one item of the render fixture project through the real
    /// pipeline: namespace read, discovery, then render_component.
    fn render_fixture(name: &str) -> (String, RenderedComponent) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/render/project");
        let source = read_rafters_namespace(&root).expect("fixture namespace must read");
        let items =
            ComponentRegistry::discover(&root, &source).expect("fixture discovery must succeed");
        let item = items
            .iter()
            .find(|item| item.name.eq_ignore_ascii_case(name))
            .unwrap_or_else(|| panic!("fixture must discover an item named {name}"))
            .clone();
        let rendered = render_component(&item, &source)
            .unwrap_or_else(|error| panic!("{name} must render: {error}"));
        (item.name, rendered)
    }

    // AC: constraints defined in source are visible on the preview surface,
    // adjacent to the preview, verbatim.
    #[test]
    fn constraints_render_on_the_preview_surface_verbatim() {
        let (name, rendered) = render_fixture("Button");
        let surface =
            generate_preview_surface(&name, &rendered).expect("Button surface must render");

        // The surface is one section holding both the preview and the
        // constraints -- same viewport region, not a separate document.
        assert!(surface.starts_with(
            "<section class=\"veneer-preview-surface\" data-veneer-surface-for=\"button-preview\">"
        ));
        assert!(surface.contains("customElements.define('button-preview'"));
        assert!(surface.contains("<button-preview></button-preview>"));
        assert!(surface.contains("<section class=\"veneer-constraints\""));

        // Verbatim from the fixture JSDoc @usage-patterns lines.
        assert!(surface.contains("Primary: main user goal, maximum 1 per section"));
        assert!(surface.contains("Secondary: alternative paths, supporting actions"));
        assert!(surface.contains("Multiple primary buttons competing for attention"));

        // The constraints region sits beside the preview element, after it,
        // inside the same section.
        let preview_at = surface
            .find("<div class=\"veneer-preview\">")
            .expect("preview element present");
        let constraints_at = surface
            .find("<section class=\"veneer-constraints\"")
            .expect("constraints region present");
        assert!(constraints_at > preview_at);
        assert!(surface.trim_end().ends_with("</section>"));
    }

    // AC: constraint order and kind labels match the source declaration.
    #[test]
    fn constraint_kinds_and_order_match_source() {
        let (name, rendered) = render_fixture("Button");
        let region = generate_constraints_region(&name, &rendered.intelligence.do_never)
            .expect("Button constraints must render");

        let first = region
            .find("Primary: main user goal, maximum 1 per section")
            .expect("first DO");
        let second = region
            .find("Secondary: alternative paths, supporting actions")
            .expect("second DO");
        let third = region
            .find("Multiple primary buttons competing for attention")
            .expect("the NEVER");
        assert!(first < second && second < third, "source order preserved");

        assert_eq!(region.matches("veneer-constraint-do").count(), 2);
        assert_eq!(region.matches("veneer-constraint-never").count(), 1);
        assert_eq!(
            region
                .matches("<span class=\"veneer-constraint-kind\">DO</span>")
                .count(),
            2
        );
        assert_eq!(
            region
                .matches("<span class=\"veneer-constraint-kind\">NEVER</span>")
                .count(),
            1
        );
    }

    // AC: a component with no do/never in source shows no constraints
    // region -- absent, not empty-invented.
    #[test]
    fn no_do_never_in_source_means_no_constraints_region() {
        let (name, rendered) = render_fixture("Plain");
        assert!(rendered.intelligence.do_never.is_empty());

        let region = generate_constraints_region(&name, &rendered.intelligence.do_never)
            .expect("empty constraints must not fail");
        assert!(region.is_empty());

        let surface =
            generate_preview_surface(&name, &rendered).expect("Plain surface must render");
        assert!(!surface.contains("veneer-constraints"));
        assert!(!surface.contains("Constraints"));
        // The preview itself still renders.
        assert!(surface.contains("<plain-preview></plain-preview>"));
    }

    // AC: manifest composites take the same path -- their usagePatterns
    // constraints render beside their preview.
    #[test]
    fn manifest_composite_constraints_render_on_its_surface() {
        let (name, rendered) = render_fixture("hero-banner");
        let surface =
            generate_preview_surface(&name, &rendered).expect("hero-banner surface must render");
        assert!(surface.contains("<hero-banner-preview></hero-banner-preview>"));
        assert!(surface.contains("Single clear CTA above the fold"));
        assert!(surface.contains("Headline under 10 words"));
        assert!(surface.contains("Multiple competing CTAs"));
    }

    // AC: do/never present in source but unparseable yields an error naming
    // the component and field -- never a silently partial rule.
    #[test]
    fn unparseable_do_never_in_source_is_a_named_error() {
        let (name, rendered) = render_fixture("Misrule");
        assert_eq!(
            rendered.intelligence.do_never.len(),
            1,
            "the fixture declares a bare DO: line"
        );

        let error = generate_preview_surface(&name, &rendered)
            .expect_err("an empty rule must not render silently");
        match &error {
            TransformError::RenderFailed { component, reason } => {
                assert!(component.eq_ignore_ascii_case("misrule"), "{component}");
                assert!(reason.contains("do constraint"), "{reason}");
            }
            other => panic!("expected RenderFailed, got {other:?}"),
        }
        let message = error.to_string();
        assert!(message.to_lowercase().contains("misrule"), "{message}");
    }

    #[test]
    fn empty_never_constraint_names_the_never_field() {
        let constraints = vec![
            Constraint {
                kind: ConstraintKind::Do,
                text: "Pair with a label".to_string(),
            },
            Constraint {
                kind: ConstraintKind::Never,
                text: "   ".to_string(),
            },
        ];
        let error = generate_constraints_region("Widget", &constraints)
            .expect_err("a blank rule must not render");
        match &error {
            TransformError::RenderFailed { component, reason } => {
                assert_eq!(component, "Widget");
                assert!(reason.contains("never constraint"), "{reason}");
            }
            other => panic!("expected RenderFailed, got {other:?}"),
        }
    }

    // Markup-significant characters are encoded; the visible text stays
    // verbatim, never reworded.
    #[test]
    fn constraint_text_is_escaped_not_reworded() {
        let constraints = vec![Constraint {
            kind: ConstraintKind::Never,
            text: "Nest <button> elements & other controls".to_string(),
        }];
        let region =
            generate_constraints_region("Widget", &constraints).expect("region must render");
        assert!(region.contains("Nest &lt;button&gt; elements &amp; other controls"));
        assert!(!region.contains("<button>"));
    }

    // Drives the real rafters checkout when available. Run with:
    //   VENEER_REAL_RAFTERS_ROOT=/path/to/rafters \
    //     cargo test -p veneer-adapters -- --ignored real_rafters
    #[test]
    #[ignore = "requires a local rafters checkout via VENEER_REAL_RAFTERS_ROOT"]
    fn real_rafters_constraints_render_verbatim_on_surfaces() {
        let Ok(root) = std::env::var("VENEER_REAL_RAFTERS_ROOT") else {
            eprintln!("VENEER_REAL_RAFTERS_ROOT not set; skipping");
            return;
        };
        let root = std::path::PathBuf::from(root);
        let source = read_rafters_namespace(&root).expect("real namespace must read");
        let items = ComponentRegistry::discover(&root, &source).expect("real discovery");

        let mut with_constraints = 0usize;
        let mut without_constraints = 0usize;
        for item in &items {
            let Ok(rendered) = render_component(item, &source) else {
                continue;
            };
            match generate_preview_surface(&item.name, &rendered) {
                Ok(surface) => {
                    if rendered.intelligence.do_never.is_empty() {
                        assert!(
                            !surface.contains("veneer-constraints"),
                            "{}: no do/never in source must mean no region",
                            item.name
                        );
                        without_constraints += 1;
                    } else {
                        for constraint in &rendered.intelligence.do_never {
                            assert!(
                                surface.contains(&escape_html(&constraint.text)),
                                "{}: constraint text must appear verbatim: {}",
                                item.name,
                                constraint.text
                            );
                        }
                        with_constraints += 1;
                    }
                }
                Err(TransformError::RenderFailed { component, reason }) => {
                    assert_eq!(&component, &item.name);
                    assert!(reason.contains("constraint"), "{reason}");
                }
                Err(other) => panic!("failures must be named RenderFailed, got {other:?}"),
            }
        }
        eprintln!(
            "real rafters: {} surfaces with constraints, {} without",
            with_constraints, without_constraints
        );
        assert!(
            with_constraints > 0,
            "old-constitution do/never must surface beside previews"
        );
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
