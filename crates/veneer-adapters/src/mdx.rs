//! Self-contained component/composite MDX page generation: the
//! human-facing parallel to the agent-facing JSON artifact in
//! [`crate::artifact`]. Both read the same [`CompiledIntelligence`]; this
//! module renders it as a documentation page, that one serializes it for
//! agents.
//!
//! veneer documents; it does not design. A generated page carries only what
//! source declares -- the facts and the live preview -- and nothing that
//! belongs to the host that renders it: no `layout` frontmatter, no CSS
//! classes, no wrapper markup, no page template. shingle lays the page out
//! and styles it; veneer states what is true.
//!
//! The page is self-contained (the design choice locked for this pipeline):
//! the preview is the self-defining Web Component. Its definition is emitted
//! as a co-located `.js` sidecar the page references with
//! `<script type="module" src="./name.js">`, not inlined: an inline module
//! script full of `{` and `export` does not parse as MDX (MDX reads `{` in
//! element children as a JS expression). The sidecar keeps the page
//! MDX-compilable while staying self-contained.
//!
//! Absence is explicit the same way it is everywhere in veneer: a field the
//! source does not declare (an empty `Vec`, a `None`) emits no section at
//! all -- never an empty table, never a synthesized value. Every line a page
//! carries is a fact source declares: the props, variants, tokens, and
//! dependencies as Markdown tables, and the `@cognitive-load` description
//! and do/never rules verbatim. Editorial prose (a summary, when-to-use,
//! examples) is not veneer's to write and not stubbed here.

use std::fmt::Write;

use crate::intelligence::{
    CognitiveLoad, Constraint, ConstraintKind, DependencyOrigin, DependencyRef, PropDoc,
    RenderedComponent, TokenRef, VariantDoc,
};
use crate::registry::{DiscoveredItem, DiscoveredKind};
use crate::traits::TransformError;
use crate::ts_helpers::kebab_case;

/// A generated documentation page and the preview script it references. The
/// page is written as `<name>.mdx` and the script as its `sidecar_name`,
/// side by side in the components directory.
#[derive(Debug, Clone)]
pub struct GeneratedComponentPage {
    /// The MDX page content.
    pub page: String,
    /// File name of the preview script the page references (for example
    /// `button-preview.js`).
    pub sidecar_name: String,
    /// The Web Component definition the sidecar holds.
    pub sidecar: String,
}

/// The file name a component's generated page is written under: its
/// kebab-cased name plus `.mdx`. Matches the name a coverage placeholder
/// uses for the same item, so documenting an item overwrites its
/// placeholder in place.
pub fn component_page_file_name(name: &str) -> String {
    format!("{}.mdx", kebab_case(name))
}

/// Human-readable label for a discovered kind, used in the page frontmatter.
fn kind_label(kind: DiscoveredKind) -> &'static str {
    match kind {
        DiscoveredKind::Component => "component",
        DiscoveredKind::Composite => "composite",
    }
}

/// Escape a value for a Markdown table cell. A literal `|` would end the
/// cell, so it is backslash-escaped; the rest is left verbatim.
fn table_cell(text: &str) -> String {
    text.replace('|', "\\|")
}

/// Wrap a value as an inline-code table cell. MDX does not evaluate inside a
/// code span, so a type like `ReactNode<T>` or a class list is safe there;
/// the `|` inside the span is still escaped so it cannot break the row.
fn code_cell(text: &str) -> String {
    format!("`{}`", table_cell(text))
}

/// Neutralize source-derived free text for MDX. Beyond HTML's `& < >`, a
/// literal `{` opens a JS expression in MDX and would fail to compile, so it
/// (and its `}`) is entity-encoded. The visible text is unchanged: the
/// entities render back to the original characters.
fn mdx_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('{', "&#123;")
        .replace('}', "&#125;")
}

/// Generate an MDX documentation page for one rendered component,
/// composite, or primitive, plus the preview script it references.
///
/// The page carries only what source declares: frontmatter (title, kind, and
/// the `cognitiveLoad` score when source declares one), the preview element
/// and its sidecar `<script src>`, then one section per intelligence field
/// the source declares -- the do/never constraints, the `@cognitive-load`
/// rationale, and Markdown tables for props, variants, tokens, and
/// dependencies. No layout, no CSS classes, no wrapper markup, no editorial
/// template: how the page looks is the host's to decide.
///
/// A section is emitted only when the source declares its field; absence is
/// explicit (no section), never an empty table.
///
/// Returns [`TransformError::RenderFailed`] naming the item if a declared
/// do/never rule has no text (an unparseable constraint), so a page never
/// renders a partial rule silently.
pub fn generate_component_page(
    item: &DiscoveredItem,
    rendered: &RenderedComponent,
) -> Result<GeneratedComponentPage, TransformError> {
    let mut out = String::new();
    let name = &item.name;
    let intelligence = &rendered.intelligence;
    let tag = &rendered.preview.tag_name;
    let sidecar_name = format!("{tag}.js");

    // Frontmatter: only the data source declares.
    writeln!(out, "---").unwrap();
    writeln!(out, "title: {name}").unwrap();
    writeln!(out, "kind: {}", kind_label(item.kind)).unwrap();
    if let Some(load) = &intelligence.cognitive_load {
        writeln!(out, "cognitiveLoad: {}", load.score).unwrap();
    }
    writeln!(out, "---").unwrap();
    writeln!(out).unwrap();

    // The preview: the self-defining element and the script that defines it.
    // No wrapper, no classes -- placement and styling are the host's.
    writeln!(out, "<{tag}></{tag}>").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "<script type=\"module\" src=\"./{sidecar_name}\"></script>").unwrap();
    writeln!(out).unwrap();

    write_constraints(&mut out, name, &intelligence.do_never)?;
    write_cognitive_load(&mut out, intelligence.cognitive_load.as_ref());
    write_props(&mut out, &intelligence.props);
    write_variants(&mut out, &intelligence.variants);
    write_tokens(&mut out, &intelligence.tokens);
    write_dependencies(&mut out, &intelligence.dependencies);

    Ok(GeneratedComponentPage {
        page: out.trim_end().to_string(),
        sidecar_name,
        sidecar: rendered.preview.web_component.clone(),
    })
}

/// The declared do/never rules, as a plain Markdown list in source order --
/// the fact of the rule, not a styled callout. A rule with blank text is
/// unparseable and fails the page, naming the item, instead of rendering a
/// partial rule.
fn write_constraints(
    out: &mut String,
    name: &str,
    constraints: &[Constraint],
) -> Result<(), TransformError> {
    if constraints.is_empty() {
        return Ok(());
    }
    writeln!(out, "## Constraints").unwrap();
    writeln!(out).unwrap();
    for constraint in constraints {
        let (field, label) = match constraint.kind {
            ConstraintKind::Do => ("do", "DO"),
            ConstraintKind::Never => ("never", "NEVER"),
        };
        if constraint.text.trim().is_empty() {
            return Err(TransformError::RenderFailed {
                component: name.to_string(),
                reason: format!("unparseable {field} constraint: empty rule text"),
            });
        }
        writeln!(out, "- {label}: {}", mdx_text(&constraint.text)).unwrap();
    }
    writeln!(out).unwrap();
    Ok(())
}

/// The declared cognitive-load rationale, verbatim. The bare score already
/// lives in frontmatter; a composite manifest declares only the number and
/// so carries no description -- absence stays absent.
fn write_cognitive_load(out: &mut String, load: Option<&CognitiveLoad>) {
    let Some(load) = load else { return };
    let Some(description) = &load.description else {
        return;
    };
    writeln!(out, "## Cognitive load").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "{}/10 -- {}", load.score, mdx_text(description)).unwrap();
    writeln!(out).unwrap();
}

/// The declared props interface. No props declared means no section.
fn write_props(out: &mut String, props: &[PropDoc]) {
    if props.is_empty() {
        return;
    }
    writeln!(out, "## Props").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "| Name | Type | Optional |").unwrap();
    writeln!(out, "| --- | --- | --- |").unwrap();
    for prop in props {
        let type_text = prop
            .type_text
            .as_deref()
            .map(code_cell)
            .unwrap_or_else(|| "-".to_string());
        let optional = if prop.optional { "Yes" } else { "No" };
        writeln!(
            out,
            "| {} | {} | {} |",
            code_cell(&prop.name),
            type_text,
            optional
        )
        .unwrap();
    }
    writeln!(out).unwrap();
}

/// The declared variants and the classes each maps to.
fn write_variants(out: &mut String, variants: &[VariantDoc]) {
    if variants.is_empty() {
        return;
    }
    writeln!(out, "## Variants").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "| Variant | Classes |").unwrap();
    writeln!(out, "| --- | --- |").unwrap();
    for variant in variants {
        writeln!(
            out,
            "| {} | {} |",
            code_cell(&variant.name),
            code_cell(&variant.classes)
        )
        .unwrap();
    }
    writeln!(out).unwrap();
}

/// The namespace tokens the component's classes reference by name.
fn write_tokens(out: &mut String, tokens: &[TokenRef]) {
    if tokens.is_empty() {
        return;
    }
    writeln!(out, "## Tokens").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "| Token | Namespace | Referenced by |").unwrap();
    writeln!(out, "| --- | --- | --- |").unwrap();
    for token in tokens {
        let referenced_by = token
            .referenced_by
            .iter()
            .map(|class| code_cell(class))
            .collect::<Vec<_>>()
            .join(" ");
        writeln!(
            out,
            "| {} | {} | {} |",
            code_cell(&token.token),
            code_cell(&token.namespace),
            referenced_by
        )
        .unwrap();
    }
    writeln!(out).unwrap();
}

/// The dependencies the source declares, with where each was declared.
fn write_dependencies(out: &mut String, dependencies: &[DependencyRef]) {
    if dependencies.is_empty() {
        return;
    }
    writeln!(out, "## Dependencies").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "| Package | Declared in |").unwrap();
    writeln!(out, "| --- | --- |").unwrap();
    for dependency in dependencies {
        let origin = match dependency.origin {
            DependencyOrigin::Import => "import",
            DependencyOrigin::JsDocTag => "@dependencies",
        };
        writeln!(out, "| {} | {} |", code_cell(&dependency.name), origin).unwrap();
    }
    writeln!(out).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intelligence::CompiledIntelligence;
    use crate::traits::TransformedBlock;
    use std::path::PathBuf;

    fn preview(tag: &str) -> TransformedBlock {
        TransformedBlock {
            web_component: format!("customElements.define('{tag}', class extends HTMLElement {{}});"),
            tag_name: tag.to_string(),
            classes_used: vec![],
            attributes: vec![],
        }
    }

    fn button_item() -> DiscoveredItem {
        DiscoveredItem {
            name: "Button".to_string(),
            kind: DiscoveredKind::Component,
            source_path: PathBuf::from("components/button.tsx"),
            generated: true,
        }
    }

    fn rich_component() -> RenderedComponent {
        RenderedComponent {
            preview: preview("button-preview"),
            intelligence: CompiledIntelligence {
                props: vec![
                    PropDoc {
                        name: "variant".to_string(),
                        type_text: Some("\"primary\" | \"secondary\"".to_string()),
                        optional: true,
                    },
                    PropDoc {
                        name: "children".to_string(),
                        type_text: Some("ReactNode".to_string()),
                        optional: false,
                    },
                ],
                config_extends: vec![],
                variants: vec![VariantDoc {
                    name: "primary".to_string(),
                    classes: "bg-primary text-primary-foreground".to_string(),
                }],
                cognitive_load: Some(CognitiveLoad {
                    score: 3,
                    description: Some("simple action trigger".to_string()),
                }),
                do_never: vec![Constraint {
                    kind: ConstraintKind::Never,
                    text: "use two primary buttons in one view".to_string(),
                }],
                tokens: vec![TokenRef {
                    token: "primary".to_string(),
                    namespace: "semantic".to_string(),
                    referenced_by: vec!["bg-primary".to_string()],
                }],
                dependencies: vec![DependencyRef {
                    name: "clsx".to_string(),
                    origin: DependencyOrigin::Import,
                }],
            },
        }
    }

    #[test]
    fn frontmatter_carries_only_source_declared_fields() {
        let generated = generate_component_page(&button_item(), &rich_component())
            .expect("generation must succeed");
        let mdx = &generated.page;
        assert!(mdx.starts_with("---\n"));
        assert!(mdx.contains("title: Button"));
        assert!(mdx.contains("kind: component"));
        assert!(mdx.contains("cognitiveLoad: 3"));
        // No host concerns: veneer documents, it does not lay out or style.
        assert!(!mdx.contains("layout:"));
        assert!(!mdx.contains("status:"));
    }

    #[test]
    fn preview_is_the_bare_element_and_its_sidecar_script() {
        let generated = generate_component_page(&button_item(), &rich_component())
            .expect("generation must succeed");
        assert_eq!(generated.sidecar_name, "button-preview.js");
        assert!(generated.sidecar.contains("customElements.define"));
        let mdx = &generated.page;
        assert!(mdx.contains("<button-preview></button-preview>"));
        assert!(mdx.contains("<script type=\"module\" src=\"./button-preview.js\"></script>"));
        // No wrapper markup, no CSS classes -- nothing veneer invented for
        // the host to style.
        assert!(!mdx.contains("class="));
        assert!(!mdx.contains("veneer-"));
        // The definition is never inlined -- that is what breaks MDX.
        assert!(
            !mdx.contains("customElements.define"),
            "the WC definition lives in the sidecar, not the page"
        );
    }

    #[test]
    fn constraints_are_a_plain_markdown_list() {
        let mdx = generate_component_page(&button_item(), &rich_component())
            .expect("generation must succeed")
            .page;
        assert!(mdx.contains("## Constraints"));
        assert!(mdx.contains("- NEVER: use two primary buttons in one view"));
    }

    #[test]
    fn generates_a_table_per_declared_field() {
        let mdx = generate_component_page(&button_item(), &rich_component())
            .expect("generation must succeed")
            .page;
        assert!(mdx.contains("## Props"));
        assert!(mdx.contains("| `variant` | `\"primary\" \\| \"secondary\"` | Yes |"));
        assert!(mdx.contains("| `children` | `ReactNode` | No |"));
        assert!(mdx.contains("## Variants"));
        assert!(mdx.contains("| `primary` | `bg-primary text-primary-foreground` |"));
        assert!(mdx.contains("## Tokens"));
        assert!(mdx.contains("| `primary` | `semantic` | `bg-primary` |"));
        assert!(mdx.contains("## Dependencies"));
        assert!(mdx.contains("| `clsx` | import |"));
        assert!(mdx.contains("## Cognitive load"));
        assert!(mdx.contains("3/10 -- simple action trigger"));
    }

    #[test]
    fn source_prose_with_braces_is_mdx_safe() {
        let mut rc = rich_component();
        rc.intelligence.do_never = vec![Constraint {
            kind: ConstraintKind::Never,
            text: "wrap in {expr} or a <Foo> tag".to_string(),
        }];
        rc.intelligence.cognitive_load = Some(CognitiveLoad {
            score: 4,
            description: Some("uses a {token} reference".to_string()),
        });
        let mdx = generate_component_page(&button_item(), &rc)
            .expect("generation must succeed")
            .page;
        // No raw brace or angle bracket from source prose survives into MDX.
        assert!(!mdx.contains("{expr}"));
        assert!(!mdx.contains("<Foo>"));
        assert!(!mdx.contains("{token}"));
        assert!(mdx.contains("&#123;expr&#125;"));
        assert!(mdx.contains("&#123;token&#125;"));
    }

    #[test]
    fn carries_no_editorial_template() {
        // Editorial prose is not veneer's to write, and no empty template
        // slot is stubbed for it -- the page is documentation, not a form.
        let mdx = generate_component_page(&button_item(), &rich_component())
            .expect("generation must succeed")
            .page;
        assert!(!mdx.contains("veneer:overview"));
        assert!(!mdx.contains("veneer:when-to-use"));
        assert!(!mdx.contains("veneer:examples"));
        assert!(!mdx.contains("veneer:gotchas"));
    }

    #[test]
    fn absent_fields_emit_no_section() {
        // A composite manifest declares only cognitive load (a bare number,
        // no description) and do/never -- no props, variants, tokens, or
        // dependencies. Those sections must be absent, never empty tables.
        let bare = RenderedComponent {
            preview: preview("hero-banner-preview"),
            intelligence: CompiledIntelligence {
                cognitive_load: Some(CognitiveLoad {
                    score: 5,
                    description: None,
                }),
                ..CompiledIntelligence::default()
            },
        };
        let item = DiscoveredItem {
            name: "hero-banner".to_string(),
            kind: DiscoveredKind::Composite,
            source_path: PathBuf::from("composites/hero-banner.composite.json"),
            generated: false,
        };
        let mdx = generate_component_page(&item, &bare)
            .expect("generation must succeed")
            .page;
        assert!(mdx.contains("kind: composite"));
        assert!(mdx.contains("cognitiveLoad: 5"));
        assert!(!mdx.contains("## Props"));
        assert!(!mdx.contains("## Variants"));
        assert!(!mdx.contains("## Tokens"));
        assert!(!mdx.contains("## Dependencies"));
        // A bare score with no declared rationale emits no rationale section.
        assert!(!mdx.contains("## Cognitive load"));
    }

    #[test]
    fn empty_constraint_text_fails_naming_the_item() {
        let mut rc = rich_component();
        rc.intelligence.do_never = vec![Constraint {
            kind: ConstraintKind::Do,
            text: "   ".to_string(),
        }];
        let err = generate_component_page(&button_item(), &rc)
            .expect_err("an empty rule must fail the page");
        match err {
            TransformError::RenderFailed { component, .. } => assert_eq!(component, "Button"),
            other => panic!("expected RenderFailed, got {other:?}"),
        }
    }

    #[test]
    fn file_name_is_kebab_cased_mdx() {
        assert_eq!(component_page_file_name("Button"), "button.mdx");
        assert_eq!(component_page_file_name("hero-banner"), "hero-banner.mdx");
    }
}
