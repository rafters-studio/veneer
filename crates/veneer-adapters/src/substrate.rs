//! The `.rafters/veneer/` substrate: the canonical docs-as-data artifact
//! (`docs.jsonl`, FR-VEN-022) and the veneer index (`index.jsonl`,
//! FR-VEN-031), both derived from one assessment pass so studio's badges and
//! the docs pages can never diverge.
//!
//! - `docs.jsonl` is canonical: one line per documented component or
//!   composite, carrying identity, the intelligence the source declares, and
//!   the preview source. Every other output format derives from it.
//! - `index.jsonl` is the roster: one line per discovered item -- identity,
//!   pointers into the other outputs, and observed state (a stoplight from a
//!   versioned rule, per-dimension statuses, structured notes).
//!
//! Both files are deterministic: lines are sorted by `(kind, name)`, structs
//! serialize in a fixed field order (no `HashMap` on this path), and no
//! timestamp is written -- two runs over unchanged input are byte-identical
//! (FR-VEN-022/031). Absence is explicit: a dimension veneer cannot observe
//! is reported `absent`, never rounded up to a pass.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

use crate::coverage::{AssessedItem, CoverageState};
use crate::intelligence::{
    CognitiveLoad, Constraint, ConstraintKind, PropDoc, RenderedComponent, TokenRef, VariantDoc,
};
use crate::matrix::{ComponentLine, ComponentMetadata};
use crate::registry::DiscoveredKind;
use crate::ts_helpers::kebab_case;

/// Per-line schema discriminator for `docs.jsonl` (FR-VEN-022).
pub const DOC_SCHEMA: &str = "veneer.doc/1";
/// Per-line schema discriminator for `index.jsonl` (FR-VEN-031).
pub const INDEX_SCHEMA: &str = "veneer.index/1";
/// Version of the stoplight derivation rule; bump on any rule change so a
/// light is always reproducible from the rule that produced it.
pub const STOPLIGHT_RULE_VERSION: &str = "1";

fn kind_str(kind: DiscoveredKind) -> &'static str {
    match kind {
        DiscoveredKind::Component => "component",
        DiscoveredKind::Composite => "composite",
    }
}

/// The stable line id an index entry points at: `<kind>:<name>`. Deterministic
/// and unique across the discovered set (kind disambiguates a shared name).
fn line_id(kind: DiscoveredKind, name: &str) -> String {
    format!("{}:{}", kind_str(kind), name)
}

/// Source path relative to the project root when it sits under it, so the
/// substrate is portable and not machine-specific; otherwise verbatim.
fn relative_source(source: &Path, project_root: &Path) -> String {
    source
        .strip_prefix(project_root)
        .unwrap_or(source)
        .display()
        .to_string()
}

/// One `docs.jsonl` line: the canonical record of one documented item.
///
/// Field order follows the Audi styleguide page shape (the doc-site render
/// target, reflection 019f6dc9): principle-first lead (what it is, why) ->
/// preview -> the intelligence dimensions -> variant/state/motion structure ->
/// the DO/NEVER constraints as the "don'ts" -> the primitives it composes as
/// cross-references. The lead and intelligence come from the rafters matrix
/// `metadata` block when the component matrix is present; the preview,
/// variants, and tokens come from the compiled component source. Every field
/// is honest-absence: absent when the source declares nothing, never
/// synthesized.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocLine {
    pub schema: &'static str,
    /// Stable id the index points at.
    pub id: String,
    pub kind: &'static str,
    pub name: String,
    pub source: String,

    // -- Principle-first lead (Audi: what it is + why/when) --
    /// The matrix `metadata.description`: the JSDoc lead prose. `None` without
    /// a matrix entry.
    pub description: Option<String>,
    /// One sentence: what it is (matrix `is`). `None` without a matrix entry.
    pub is: Option<String>,
    /// One sentence: what it does (matrix `does`). `None` without a matrix.
    pub does: Option<String>,
    /// The matrix archetype (e.g. `simple-interactive`). `None` without a
    /// matrix entry.
    pub archetype: Option<String>,

    /// The framework-less Web Component preview source.
    pub preview: DocPreview,

    // -- Intelligence (Audi: hierarchy, tone, semantics, accessibility) --
    /// `None` when neither the matrix nor the source declares a cognitive load.
    pub cognitive_load: Option<CognitiveLoad>,
    /// Matrix `metadata.attentionEconomics`: the variant/hierarchy guidance.
    pub attention_economics: Option<String>,
    /// Matrix `metadata.trustBuilding`: the tone/trust note.
    pub trust_building: Option<String>,
    /// Matrix `metadata.accessibility`: the a11y and validation note.
    pub accessibility: Option<String>,
    /// Matrix `metadata.semanticMeaning`: the variant-role mapping.
    pub semantic_meaning: Option<String>,

    // -- Structure (Audi: named variants, states, motion) --
    /// The component's own prop/API surface (from the behavior `Config`
    /// interface for new-constitution components, else the `*Props` interface).
    pub props: Vec<PropDoc>,
    /// The `Config` interfaces the prop surface extends, by name -- the
    /// unresolved remainder of the attribute surface. Empty for the `*Props`
    /// path and composites.
    pub config_extends: Vec<String>,
    pub variants: Vec<VariantDoc>,
    /// Matrix `states`: the descriptive state vocabulary. Empty for statics
    /// and without a matrix entry.
    pub states: Vec<String>,
    /// Matrix `motion.intents`: the Spec 04 motion intents the port declares.
    pub motion_intents: Vec<String>,

    // -- The "don'ts" (Audi: examples + don'ts at the bottom) --
    /// DO/NEVER rules, in source order, from the matrix `usagePatterns` when
    /// present, else the compiled source constraints.
    pub constraints: Vec<Constraint>,

    // -- Cross-references (Audi: "more under X >") --
    pub tokens: Vec<TokenRef>,
    /// The primitives this component composes (matrix `uses.current`): the
    /// dependency surface veneer renders, per the interface contract. Empty
    /// without a matrix entry; npm imports are deliberately not carried here.
    pub primitives_used: Vec<String>,
}

/// The preview source carried on a docs line: the custom element tag and the
/// self-defining Web Component module.
#[derive(Debug, Serialize)]
pub struct DocPreview {
    pub tag: String,
    pub source: String,
}

/// One `index.jsonl` line: identity, pointers, and observed state for one
/// discovered item.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexLine {
    pub schema: &'static str,
    pub name: String,
    pub kind: &'static str,
    pub source: String,
    /// Pointer to this item's `docs.jsonl` line id, or `None` when it is not
    /// documented (nothing rendered).
    pub docs_id: Option<String>,
    /// Serialized page paths for this item. Empty until a serializer tier
    /// (FR-VEN-023) runs; the machine substrate does not depend on them.
    pub pages: Vec<String>,
    /// Pointer to the item's intelligence artifact, when one is emitted.
    pub artifact: Option<String>,
    pub state: IndexState,
}

/// The observed state of one index line: a stoplight, the per-dimension
/// statuses it derives from, and structured notes.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexState {
    /// `green` | `yellow` | `red`, from the versioned rule.
    pub stoplight: &'static str,
    /// The rule version that produced `stoplight`.
    pub rule: &'static str,
    pub dimensions: IndexDimensions,
    pub notes: Vec<IndexNote>,
}

/// Per-dimension status. Each is `pass`, `fail`, or `absent` -- a dimension
/// veneer cannot observe is `absent`, never a silent pass.
#[derive(Debug, Serialize)]
pub struct IndexDimensions {
    pub tests: &'static str,
    pub wcag: &'static str,
    pub metadata: &'static str,
    pub coverage: &'static str,
    pub freshness: &'static str,
}

/// A structured observation on an index line -- never freetext-only.
#[derive(Debug, Serialize)]
pub struct IndexNote {
    pub kind: &'static str,
    pub severity: &'static str,
    pub detail: String,
    pub evidence: String,
}

/// The `.rafters/veneer/` substrate for one pass: the canonical docs lines
/// and the index lines, both derived from the same assessment.
#[derive(Debug)]
pub struct Substrate {
    pub docs: Vec<DocLine>,
    pub index: Vec<IndexLine>,
}

/// Build the substrate from an assessed discovered set and the rafters
/// component matrix. Items are sorted by `(kind, name)` so line order is
/// stable regardless of discovery/filesystem order; a documented item yields
/// both a docs line and an index line that points at it, an undocumented item
/// yields an index line alone.
///
/// `matrix` is keyed by the matrix's component name (word-safe, kebab-cased);
/// each item is matched to its line by the kebab-cased discovered name. The
/// matrix is the canonical intelligence source (interface contract): it
/// supplies the principle-first lead and the intelligence dimensions. When an
/// item has no matrix line (a consumer project has no matrix, an item is
/// unlisted), those fields are honestly absent and the compiled source
/// intelligence is the fallback for cognitive load and do/never.
pub fn build_substrate(
    assessed: &[AssessedItem],
    matrix: &BTreeMap<String, ComponentLine>,
    project_root: &Path,
) -> Substrate {
    let mut order: Vec<&AssessedItem> = assessed.iter().collect();
    order.sort_by(|a, b| {
        kind_str(a.item.kind)
            .cmp(kind_str(b.item.kind))
            .then_with(|| a.item.name.cmp(&b.item.name))
    });

    let mut docs = Vec::new();
    let mut index = Vec::new();
    for entry in order {
        let id = line_id(entry.item.kind, &entry.item.name);
        let source = relative_source(&entry.item.source_path, project_root);
        let line = matrix.get(&kebab_case(&entry.item.name));
        let docs_id = entry.rendered.as_ref().map(|rendered| {
            docs.push(doc_line(&id, entry, rendered, line, source.clone()));
            id.clone()
        });
        index.push(index_line(entry, docs_id, source));
    }
    Substrate { docs, index }
}

/// Map the matrix cognitive-load `{score, note}` onto veneer's
/// `CognitiveLoad {score, description}` -- the matrix note is the description.
fn matrix_cognitive_load(meta: &ComponentMetadata) -> Option<CognitiveLoad> {
    meta.cognitive_load.as_ref().map(|load| CognitiveLoad {
        score: load.score,
        description: load.note.clone(),
    })
}

/// Classify the matrix `usagePatterns` into DO/NEVER constraints on the
/// prefix (interface contract: every entry is `DO:`/`NEVER:`-prefixed). An
/// unprefixed entry is plain guidance, not a constraint, so it is not carried
/// here -- per the contract it should not occur.
fn constraints_from_usage(patterns: &[String]) -> Vec<Constraint> {
    patterns
        .iter()
        .filter_map(|line| {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("DO:") {
                Some(Constraint {
                    kind: ConstraintKind::Do,
                    text: rest.trim().to_string(),
                })
            } else if let Some(rest) = trimmed.strip_prefix("NEVER:") {
                Some(Constraint {
                    kind: ConstraintKind::Never,
                    text: rest.trim().to_string(),
                })
            } else {
                None
            }
        })
        .collect()
}

fn doc_line(
    id: &str,
    entry: &AssessedItem,
    rendered: &RenderedComponent,
    matrix: Option<&ComponentLine>,
    source: String,
) -> DocLine {
    let intelligence = &rendered.intelligence;
    let meta = matrix.and_then(|line| line.metadata.as_ref());

    // The matrix metadata is the canonical intelligence; the compiled source
    // intelligence is the fallback only where the matrix declares none, so a
    // matrixed component never loses a source-declared load or do/never.
    let cognitive_load = meta
        .and_then(matrix_cognitive_load)
        .or_else(|| intelligence.cognitive_load.clone());
    let constraints = match meta.and_then(|meta| meta.usage_patterns.as_deref()) {
        Some(patterns) => constraints_from_usage(patterns),
        None => intelligence.do_never.clone(),
    };

    DocLine {
        schema: DOC_SCHEMA,
        id: id.to_string(),
        kind: kind_str(entry.item.kind),
        name: entry.item.name.clone(),
        source,

        description: meta.and_then(|meta| meta.description.clone()),
        is: matrix.map(|line| line.is.clone()),
        does: matrix.map(|line| line.does.clone()),
        archetype: matrix.map(|line| line.archetype.as_str().to_string()),

        preview: DocPreview {
            tag: rendered.preview.tag_name.clone(),
            source: rendered.preview.web_component.clone(),
        },

        cognitive_load,
        attention_economics: meta.and_then(|meta| meta.attention_economics.clone()),
        trust_building: meta.and_then(|meta| meta.trust_building.clone()),
        accessibility: meta.and_then(|meta| meta.accessibility.clone()),
        semantic_meaning: meta.and_then(|meta| meta.semantic_meaning.clone()),

        props: intelligence.props.clone(),
        config_extends: intelligence.config_extends.clone(),
        variants: intelligence.variants.clone(),
        states: matrix.map(|line| line.states.clone()).unwrap_or_default(),
        motion_intents: matrix
            .map(|line| line.motion.intents.clone())
            .unwrap_or_default(),

        constraints,

        tokens: intelligence.tokens.clone(),
        primitives_used: matrix
            .map(|line| line.uses.current.clone())
            .unwrap_or_default(),
    }
}

fn index_line(entry: &AssessedItem, docs_id: Option<String>, source: String) -> IndexLine {
    let state = index_state(entry);
    IndexLine {
        schema: INDEX_SCHEMA,
        name: entry.item.name.clone(),
        kind: kind_str(entry.item.kind),
        source,
        docs_id,
        pages: Vec::new(),
        artifact: None,
        state,
    }
}

/// Derive an item's observed state. Coverage (does it render) and metadata
/// (does the source declare intelligence) are observable now; tests, wcag,
/// and freshness are not yet wired, so they are honestly `absent`.
fn index_state(entry: &AssessedItem) -> IndexState {
    let mut notes = Vec::new();

    let coverage = match &entry.state {
        CoverageState::Documented => "pass",
        CoverageState::NotYetDocumented { reason } => {
            notes.push(IndexNote {
                kind: "coverage",
                severity: "error",
                detail: reason.clone(),
                evidence: entry.item.source_path.display().to_string(),
            });
            "fail"
        }
    };

    let metadata = match &entry.rendered {
        Some(rendered)
            if rendered.intelligence.cognitive_load.is_some()
                || !rendered.intelligence.do_never.is_empty() =>
        {
            "pass"
        }
        _ => "absent",
    };

    let dimensions = IndexDimensions {
        tests: "absent",
        wcag: "absent",
        metadata,
        coverage,
        freshness: "absent",
    };

    IndexState {
        stoplight: stoplight(&dimensions),
        rule: STOPLIGHT_RULE_VERSION,
        dimensions,
        notes,
    }
}

/// Stoplight rule v1: a failing observed fact is red; otherwise any absence
/// is yellow; green requires that everything renders and every present fact
/// passes. With tests/wcag/freshness not yet observable, a rendered item is
/// yellow (honest) and a non-rendering item is red.
fn stoplight(dimensions: &IndexDimensions) -> &'static str {
    let all = [
        dimensions.tests,
        dimensions.wcag,
        dimensions.metadata,
        dimensions.coverage,
        dimensions.freshness,
    ];
    if all.iter().any(|status| *status == "fail") {
        "red"
    } else if all.iter().any(|status| *status == "absent") {
        "yellow"
    } else {
        "green"
    }
}

/// Serialize lines as JSONL: one compact JSON object per line, newline
/// terminated. Deterministic given deterministic input.
pub fn to_jsonl<T: Serialize>(lines: &[T]) -> Result<String, serde_json::Error> {
    let mut out = String::new();
    for line in lines {
        out.push_str(&serde_json::to_string(line)?);
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coverage::CoverageState;
    use crate::intelligence::{CompiledIntelligence, ConstraintKind, RenderedComponent};
    use crate::registry::DiscoveredItem;
    use crate::traits::TransformedBlock;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn rendered(tag: &str) -> RenderedComponent {
        RenderedComponent {
            preview: TransformedBlock {
                web_component: format!("customElements.define('{tag}', class {{}});"),
                tag_name: tag.to_string(),
                classes_used: vec![],
                attributes: vec![],
            },
            intelligence: CompiledIntelligence {
                cognitive_load: Some(CognitiveLoad {
                    score: 3,
                    description: Some("simple".to_string()),
                }),
                do_never: vec![Constraint {
                    kind: ConstraintKind::Never,
                    text: "two primary CTAs".to_string(),
                }],
                ..CompiledIntelligence::default()
            },
        }
    }

    fn documented(name: &str) -> AssessedItem {
        AssessedItem {
            item: DiscoveredItem {
                name: name.to_string(),
                kind: DiscoveredKind::Component,
                source_path: PathBuf::from(format!("/proj/components/{name}.tsx")),
                generated: true,
            },
            state: CoverageState::Documented,
            rendered: Some(rendered(&format!("{name}-preview"))),
        }
    }

    fn undocumented(name: &str, reason: &str) -> AssessedItem {
        AssessedItem {
            item: DiscoveredItem {
                name: name.to_string(),
                kind: DiscoveredKind::Component,
                source_path: PathBuf::from(format!("/proj/components/{name}.tsx")),
                generated: false,
            },
            state: CoverageState::NotYetDocumented {
                reason: reason.to_string(),
            },
            rendered: None,
        }
    }

    #[test]
    fn documented_item_yields_a_docs_line_and_a_pointing_index_line() {
        let assessed = vec![documented("Button")];
        let sub = build_substrate(&assessed, &BTreeMap::new(), Path::new("/proj"));
        assert_eq!(sub.docs.len(), 1);
        assert_eq!(sub.index.len(), 1);
        assert_eq!(sub.docs[0].id, "component:Button");
        assert_eq!(sub.docs[0].source, "components/Button.tsx");
        assert_eq!(sub.index[0].docs_id.as_deref(), Some("component:Button"));
        assert_eq!(sub.docs[0].preview.tag, "Button-preview");
    }

    #[test]
    fn undocumented_item_has_an_index_line_but_no_docs_line() {
        let assessed = vec![undocumented("Broken", "unparseable source")];
        let sub = build_substrate(&assessed, &BTreeMap::new(), Path::new("/proj"));
        assert!(sub.docs.is_empty());
        assert_eq!(sub.index.len(), 1);
        assert_eq!(sub.index[0].docs_id, None);
        assert_eq!(sub.index[0].state.dimensions.coverage, "fail");
        assert_eq!(sub.index[0].state.stoplight, "red");
        assert_eq!(sub.index[0].state.notes.len(), 1);
        assert_eq!(sub.index[0].state.notes[0].detail, "unparseable source");
    }

    #[test]
    fn a_rendered_item_is_yellow_while_tests_and_wcag_are_unobserved() {
        let assessed = vec![documented("Button")];
        let sub = build_substrate(&assessed, &BTreeMap::new(), Path::new("/proj"));
        let state = &sub.index[0].state;
        assert_eq!(state.dimensions.coverage, "pass");
        assert_eq!(state.dimensions.metadata, "pass");
        assert_eq!(state.dimensions.tests, "absent");
        assert_eq!(state.dimensions.wcag, "absent");
        // Honest: no green while whole dimensions are unobserved.
        assert_eq!(state.stoplight, "yellow");
    }

    #[test]
    fn lines_are_sorted_by_kind_then_name() {
        let assessed = vec![
            documented("Zebra"),
            undocumented("Alpha", "no source"),
            documented("Mango"),
        ];
        let sub = build_substrate(&assessed, &BTreeMap::new(), Path::new("/proj"));
        let names: Vec<&str> = sub.index.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, ["Alpha", "Mango", "Zebra"]);
    }

    #[test]
    fn jsonl_is_one_object_per_line_with_a_schema_discriminator() {
        let assessed = vec![documented("Button")];
        let sub = build_substrate(&assessed, &BTreeMap::new(), Path::new("/proj"));
        let docs = to_jsonl(&sub.docs).expect("serialize docs");
        let index = to_jsonl(&sub.index).expect("serialize index");
        assert_eq!(docs.lines().count(), 1);
        assert_eq!(index.lines().count(), 1);
        assert!(docs.contains("\"schema\":\"veneer.doc/1\""));
        assert!(index.contains("\"schema\":\"veneer.index/1\""));
        // camelCase keys reach the wire.
        assert!(docs.contains("\"cognitiveLoad\""));
        assert!(index.contains("\"docsId\""));
        // Each line parses as its own JSON object.
        for line in docs.lines().chain(index.lines()) {
            serde_json::from_str::<serde_json::Value>(line).expect("each line is valid JSON");
        }
    }

    #[test]
    fn build_is_deterministic_across_passes() {
        let build = || {
            let assessed = vec![documented("Button"), undocumented("Broken", "unparseable")];
            let sub = build_substrate(&assessed, &BTreeMap::new(), Path::new("/proj"));
            (
                to_jsonl(&sub.docs).unwrap(),
                to_jsonl(&sub.index).unwrap(),
            )
        };
        assert_eq!(build(), build(), "two passes must be byte-identical");
    }

    // AC (interface contract): a matrixed item carries the principle-first
    // lead and the intelligence dimensions from the matrix metadata, in the
    // Audi page shape; usagePatterns classify into DO/NEVER on their prefix.
    #[test]
    fn a_matrixed_item_carries_the_audi_intelligence_block() {
        let json = r#"{"schema":"rafters.component-line/1","name":"button","archetype":"simple-interactive","status":"ported","is":"Triggers an action","does":"Runs the action on click","states":["disabled"],"uses":{"current":["classy"],"planned":[]},"motion":{"current":"","intents":["press: scale"]},"frameworks":{"behaviorLayer":{"react":"verified","astro":"missing","wc":"ported","vue":"missing"}},"metadata":{"source":"src/old/ui/button.tsx","description":"Primary action control","cognitiveLoad":{"score":2,"note":"one clear action"},"attentionEconomics":"primary draws the eye","trustBuilding":"labels tell the truth","accessibility":"role=button; focus-visible ring","semanticMeaning":"primary=main action","usagePatterns":["DO: label the action","NEVER: two primary buttons in one view"]}}"#;
        let lines = crate::matrix::parse_matrix(json).expect("parse matrix line");
        let mut matrix = BTreeMap::new();
        matrix.insert(lines[0].name.clone(), lines[0].clone());

        // The discovered item is PascalCase; it matches the matrix line by its
        // kebab-cased name.
        let assessed = vec![documented("Button")];
        let sub = build_substrate(&assessed, &matrix, Path::new("/proj"));
        let doc = &sub.docs[0];

        // Principle-first lead.
        assert_eq!(doc.is.as_deref(), Some("Triggers an action"));
        assert_eq!(doc.does.as_deref(), Some("Runs the action on click"));
        assert_eq!(doc.archetype.as_deref(), Some("simple-interactive"));
        assert_eq!(doc.description.as_deref(), Some("Primary action control"));

        // Intelligence dimensions.
        assert_eq!(doc.attention_economics.as_deref(), Some("primary draws the eye"));
        assert_eq!(doc.trust_building.as_deref(), Some("labels tell the truth"));
        assert_eq!(doc.accessibility.as_deref(), Some("role=button; focus-visible ring"));
        assert_eq!(doc.semantic_meaning.as_deref(), Some("primary=main action"));

        // Structure and cross-references.
        assert_eq!(doc.states, ["disabled"]);
        assert_eq!(doc.motion_intents, ["press: scale"]);
        assert_eq!(doc.primitives_used, ["classy"]);

        // Cognitive load comes from the matrix (note -> description).
        let load = doc.cognitive_load.as_ref().expect("matrix cognitive load");
        assert_eq!(load.score, 2);
        assert_eq!(load.description.as_deref(), Some("one clear action"));

        // usagePatterns classified into DO/NEVER.
        assert_eq!(doc.constraints.len(), 2);
        assert_eq!(doc.constraints[0].kind, ConstraintKind::Do);
        assert_eq!(doc.constraints[0].text, "label the action");
        assert_eq!(doc.constraints[1].kind, ConstraintKind::Never);
        assert_eq!(doc.constraints[1].text, "two primary buttons in one view");
    }

    // Honest-absence: without a matrix line the lead is absent, and the
    // compiled source intelligence is the fallback for load and do/never so
    // an unmatrixed item never loses what its source declares.
    #[test]
    fn without_a_matrix_the_lead_is_absent_and_intelligence_falls_back_to_source() {
        let assessed = vec![documented("Button")];
        let sub = build_substrate(&assessed, &BTreeMap::new(), Path::new("/proj"));
        let doc = &sub.docs[0];

        assert!(doc.is.is_none());
        assert!(doc.does.is_none());
        assert!(doc.archetype.is_none());
        assert!(doc.attention_economics.is_none());
        assert!(doc.states.is_empty());
        assert!(doc.motion_intents.is_empty());
        assert!(doc.primitives_used.is_empty());

        // Fallback to the source-declared load and do/never from rendered().
        assert!(doc.cognitive_load.is_some());
        assert_eq!(doc.constraints.len(), 1);
        assert_eq!(doc.constraints[0].kind, ConstraintKind::Never);
    }
}
