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
use crate::rafters_source::{IntelligenceSource, TokenValue};
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
    kind.as_str()
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
    /// The PROJECT-side remedy, when one is known (FR-VEN-021: observations
    /// name the project's own fix; veneer never takes remedial action).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remedy: Option<String>,
}

/// The `.rafters/veneer/` substrate for one pass: the canonical docs lines
/// and the index lines, both derived from the same assessment.
#[derive(Debug)]
pub struct Substrate {
    pub docs: Vec<DocLine>,
    /// System-page lines (token overview). Sorted after item lines by the
    /// same (kind, name) rule: "component" < "composite" < "system".
    pub system: Vec<SystemLine>,
    pub index: Vec<IndexLine>,
}

impl Substrate {
    /// Serialize the complete docs.jsonl: item lines then system lines,
    /// which preserves the global (kind, name) ordering rule because
    /// "system" sorts after "component" and "composite".
    pub fn docs_jsonl(&self) -> Result<String, serde_json::Error> {
        let mut out = to_jsonl(&self.docs)?;
        let system = to_jsonl(&self.system)?;
        out.push_str(&system);
        Ok(out)
    }

    /// Total docs.jsonl line count (item lines plus system lines).
    pub fn docs_line_count(&self) -> usize {
        self.docs.len() + self.system.len()
    }
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
/// One `docs.jsonl` SYSTEM line: the token-system overview page. Emitted only
/// when the project declares a rafters namespace source -- a project without
/// one has no token system to document (honest absence, never a stub page).
///
/// Carries counts and presence, not token values: the namespace files remain
/// the source of truth and the docs line is the page's identity plus its
/// summary facts (FR-VEN-022 "system page" line).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemLine {
    pub schema: &'static str,
    pub id: String,
    pub kind: &'static str,
    pub name: String,
    /// One entry per namespace file, sorted by namespace name.
    pub namespaces: Vec<SystemNamespace>,
    /// Namespaces that declare accessibility contrast matrices, sorted.
    pub contrast_matrices: Vec<String>,
    /// Where the namespace source lives, relative to the project root.
    pub source: String,
}

/// Per-namespace summary on the system line.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemNamespace {
    pub namespace: String,
    pub tokens: usize,
}

/// Build the token-system line from the namespace source, when one exists.
fn system_line(source: &IntelligenceSource) -> Option<SystemLine> {
    let IntelligenceSource::Namespace(namespace) = source else {
        return None;
    };
    let mut namespaces: Vec<SystemNamespace> = namespace
        .namespaces
        .iter()
        .map(|(name, file)| SystemNamespace {
            namespace: name.clone(),
            tokens: file.tokens.len(),
        })
        .collect();
    namespaces.sort_by(|a, b| a.namespace.cmp(&b.namespace));

    // Accessibility matrices ride on structured token VALUES (brand colors),
    // so a namespace carries matrices when any of its tokens does.
    let mut contrast_matrices: Vec<String> = namespace
        .namespaces
        .iter()
        .filter(|(_, file)| {
            file.tokens.iter().any(|token| {
                matches!(
                    &token.value,
                    TokenValue::Structured(value) if value.accessibility.is_some()
                )
            })
        })
        .map(|(name, _)| name.clone())
        .collect();
    contrast_matrices.sort();

    Some(SystemLine {
        schema: DOC_SCHEMA,
        id: "system:tokens".to_string(),
        kind: "system",
        name: "tokens".to_string(),
        namespaces,
        contrast_matrices,
        source: ".rafters/tokens".to_string(),
    })
}

pub fn build_substrate(
    assessed: &[AssessedItem],
    matrix: &BTreeMap<String, ComponentLine>,
    project_root: &Path,
    source_kind: &IntelligenceSource,
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
        index.push(index_line(
            entry,
            docs_id,
            source,
            matches!(source_kind, IntelligenceSource::NoSource),
        ));
    }
    Substrate {
        docs,
        system: system_line(source_kind).into_iter().collect(),
        index,
    }
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
            } else {
                trimmed.strip_prefix("NEVER:").map(|rest| Constraint {
                    kind: ConstraintKind::Never,
                    text: rest.trim().to_string(),
                })
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

/// The project-side remedy for a coverage refusal, when the reason names a
/// cause veneer knows the fix for. Matching on the reason string is the seam
/// available today; if refusal reasons become structured, this moves onto
/// that structure.
fn coverage_remedy(reason: &str) -> Option<String> {
    reason.contains("stylesheet").then(|| {
        "enable exports.compiled in .rafters/config.rafters.json and re-run the project's rafters export".to_string()
    })
}

fn index_line(
    entry: &AssessedItem,
    docs_id: Option<String>,
    source: String,
    namespace_missing: bool,
) -> IndexLine {
    let state = index_state(entry, namespace_missing);
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
fn index_state(entry: &AssessedItem, namespace_missing: bool) -> IndexState {
    let mut notes = Vec::new();

    // FR-VEN-021: absent rafters state is an observation naming the gap and
    // the project's own remedy -- never acted on, never silently absorbed.
    if namespace_missing {
        notes.push(IndexNote {
            kind: "source",
            severity: "warning",
            detail: "no .rafters namespace source; intelligence limited to what the component source declares".to_string(),
            evidence: ".rafters/".to_string(),
            remedy: Some("run rafters init in the project (or restore its .rafters/ state)".to_string()),
        });
    }

    let coverage = match &entry.state {
        CoverageState::Documented => "pass",
        CoverageState::NotYetDocumented { reason } => {
            notes.push(IndexNote {
                kind: "coverage",
                severity: "error",
                detail: reason.clone(),
                evidence: entry.item.source_path.display().to_string(),
                remedy: coverage_remedy(reason),
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
    if all.contains(&"fail") {
        "red"
    } else if all.contains(&"absent") {
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
        let sub = build_substrate(
            &assessed,
            &BTreeMap::new(),
            Path::new("/proj"),
            &IntelligenceSource::NoSource,
        );
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
        let sub = build_substrate(
            &assessed,
            &BTreeMap::new(),
            Path::new("/proj"),
            &IntelligenceSource::NoSource,
        );
        assert!(sub.docs.is_empty());
        assert_eq!(sub.index.len(), 1);
        assert_eq!(sub.index[0].docs_id, None);
        assert_eq!(sub.index[0].state.dimensions.coverage, "fail");
        assert_eq!(sub.index[0].state.stoplight, "red");
        // Two notes under a NoSource fixture: the coverage refusal AND the
        // missing-namespace observation (FR-VEN-021).
        let coverage_notes: Vec<_> = sub.index[0]
            .state
            .notes
            .iter()
            .filter(|note| note.kind == "coverage")
            .collect();
        assert_eq!(coverage_notes.len(), 1);
        assert_eq!(coverage_notes[0].detail, "unparseable source");
    }

    #[test]
    fn a_rendered_item_is_yellow_while_tests_and_wcag_are_unobserved() {
        let assessed = vec![documented("Button")];
        let sub = build_substrate(
            &assessed,
            &BTreeMap::new(),
            Path::new("/proj"),
            &IntelligenceSource::NoSource,
        );
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
        let sub = build_substrate(
            &assessed,
            &BTreeMap::new(),
            Path::new("/proj"),
            &IntelligenceSource::NoSource,
        );
        let names: Vec<&str> = sub.index.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, ["Alpha", "Mango", "Zebra"]);
    }

    #[test]
    fn jsonl_is_one_object_per_line_with_a_schema_discriminator() {
        let assessed = vec![documented("Button")];
        let sub = build_substrate(
            &assessed,
            &BTreeMap::new(),
            Path::new("/proj"),
            &IntelligenceSource::NoSource,
        );
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
            let sub = build_substrate(
                &assessed,
                &BTreeMap::new(),
                Path::new("/proj"),
                &IntelligenceSource::NoSource,
            );
            (to_jsonl(&sub.docs).unwrap(), to_jsonl(&sub.index).unwrap())
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
        let sub = build_substrate(
            &assessed,
            &matrix,
            Path::new("/proj"),
            &IntelligenceSource::NoSource,
        );
        let doc = &sub.docs[0];

        // Principle-first lead.
        assert_eq!(doc.is.as_deref(), Some("Triggers an action"));
        assert_eq!(doc.does.as_deref(), Some("Runs the action on click"));
        assert_eq!(doc.archetype.as_deref(), Some("simple-interactive"));
        assert_eq!(doc.description.as_deref(), Some("Primary action control"));

        // Intelligence dimensions.
        assert_eq!(
            doc.attention_economics.as_deref(),
            Some("primary draws the eye")
        );
        assert_eq!(doc.trust_building.as_deref(), Some("labels tell the truth"));
        assert_eq!(
            doc.accessibility.as_deref(),
            Some("role=button; focus-visible ring")
        );
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
        let sub = build_substrate(
            &assessed,
            &BTreeMap::new(),
            Path::new("/proj"),
            &IntelligenceSource::NoSource,
        );
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

    fn namespace_fixture() -> IntelligenceSource {
        use crate::rafters_source::{NamespaceFile, RaftersNamespace};
        let mut namespaces = BTreeMap::new();
        for name in ["motion", "color"] {
            namespaces.insert(
                name.to_string(),
                NamespaceFile {
                    schema: None,
                    namespace: name.to_string(),
                    version: None,
                    generated_at: None,
                    tokens: vec![],
                },
            );
        }
        IntelligenceSource::Namespace(RaftersNamespace { namespaces })
    }

    // FR-VEN-022: one line per documented item AND system page.
    #[test]
    fn a_namespace_source_yields_the_tokens_system_line() {
        let source = namespace_fixture();
        let sub = build_substrate(&[], &BTreeMap::new(), Path::new("/proj"), &source);
        assert_eq!(sub.system.len(), 1);
        let line = &sub.system[0];
        assert_eq!(line.id, "system:tokens");
        assert_eq!(line.kind, "system");
        assert_eq!(line.schema, DOC_SCHEMA);
        // Sorted by namespace name regardless of map/file order.
        let names: Vec<&str> = line
            .namespaces
            .iter()
            .map(|n| n.namespace.as_str())
            .collect();
        assert_eq!(names, vec!["color", "motion"]);
    }

    #[test]
    fn no_source_yields_no_system_line_never_a_stub() {
        let sub = build_substrate(
            &[],
            &BTreeMap::new(),
            Path::new("/proj"),
            &IntelligenceSource::NoSource,
        );
        assert!(sub.system.is_empty());
    }

    #[test]
    fn docs_jsonl_orders_system_after_items_and_every_line_parses() {
        let assessed = vec![documented("Button")];
        let source = namespace_fixture();
        let sub = build_substrate(&assessed, &BTreeMap::new(), Path::new("/proj"), &source);
        let jsonl = sub.docs_jsonl().expect("serializes");
        let lines: Vec<&str> = jsonl.lines().collect();
        assert_eq!(lines.len(), sub.docs_line_count());
        let kinds: Vec<String> = lines
            .iter()
            .map(|line| {
                let value: serde_json::Value = serde_json::from_str(line).expect("line parses");
                assert_eq!(value["schema"], DOC_SCHEMA);
                value["kind"].as_str().expect("kind").to_string()
            })
            .collect();
        // Global (kind, name) order: component < composite < system.
        assert_eq!(kinds, vec!["component", "system"]);
    }

    // FR-VEN-022 + FR-VEN-033 line completeness: every intelligence field the
    // model carries surfaces on the line (or its omission is a recorded
    // decision). The exhaustive destructure makes adding a field to
    // CompiledIntelligence a compile error here until the line answers for it.
    #[test]
    fn the_doc_line_answers_for_every_intelligence_field() {
        let full = CompiledIntelligence {
            props: vec![crate::intelligence::PropDoc {
                name: "variant".to_string(),
                type_text: Some("'default' | 'ghost'".to_string()),
                optional: true,
            }],
            config_extends: vec!["SharedConfig".to_string()],
            variants: vec![VariantDoc {
                name: "ghost".to_string(),
                classes: "bg-transparent".to_string(),
            }],
            cognitive_load: Some(CognitiveLoad {
                score: 4,
                description: Some("verbatim description".to_string()),
            }),
            do_never: vec![Constraint {
                kind: ConstraintKind::Do,
                text: "Keep the label visible - placeholder-only fields fail recall".to_string(),
            }],
            tokens: vec![TokenRef {
                token: "primary".to_string(),
                namespace: "semantic".to_string(),
                referenced_by: vec!["bg-primary".to_string()],
            }],
            dependencies: vec![],
        };

        // The destructure is the completeness contract.
        let CompiledIntelligence {
            props,
            config_extends,
            variants,
            cognitive_load,
            do_never,
            tokens,
            // npm imports are deliberately NOT carried on the line: the
            // rendered pages do not show them (primitives_used carries the
            // matrix dependency surface instead). A recorded decision, not
            // an oversight.
            dependencies: _,
        } = &full;

        let mut item = documented("Button");
        item.rendered.as_mut().expect("rendered").intelligence = full.clone();
        let sub = build_substrate(
            &[item],
            &BTreeMap::new(),
            Path::new("/proj"),
            &IntelligenceSource::NoSource,
        );
        let line = serde_json::to_value(&sub.docs[0]).expect("serializes");

        assert_eq!(line["props"][0]["name"], props[0].name.as_str());
        assert_eq!(line["configExtends"][0], config_extends[0].as_str());
        assert_eq!(line["variants"][0]["name"], variants[0].name.as_str());
        assert_eq!(
            line["cognitiveLoad"]["score"],
            cognitive_load.as_ref().expect("load").score
        );
        // DO/NEVER text rides VERBATIM (FR-VEN-022).
        assert_eq!(line["constraints"][0]["text"], do_never[0].text.as_str());
        assert_eq!(line["tokens"][0]["token"], tokens[0].token.as_str());
    }

    // FR-VEN-021: a stylesheet-caused refusal names the project-side remedy.
    #[test]
    fn a_stylesheet_refusal_note_names_the_project_side_remedy() {
        let assessed = vec![undocumented("Button", "project stylesheet is empty")];
        let sub = build_substrate(
            &assessed,
            &BTreeMap::new(),
            Path::new("/proj"),
            &IntelligenceSource::NoSource,
        );
        let note = sub.index[0]
            .state
            .notes
            .iter()
            .find(|note| note.kind == "coverage")
            .expect("coverage note");
        let remedy = note.remedy.as_deref().expect("remedy named");
        assert!(
            remedy.contains("exports.compiled"),
            "remedy names the project fix: {remedy}"
        );
    }

    // FR-VEN-021: missing namespace state is an observation with a remedy,
    // and veneer still documents what the component source declares.
    #[test]
    fn missing_namespace_is_an_observation_naming_the_remedy() {
        let assessed = vec![documented("Button")];
        let sub = build_substrate(
            &assessed,
            &BTreeMap::new(),
            Path::new("/proj"),
            &IntelligenceSource::NoSource,
        );
        let note = sub.index[0]
            .state
            .notes
            .iter()
            .find(|note| note.kind == "source")
            .expect("source observation");
        assert_eq!(note.severity, "warning");
        assert!(note
            .remedy
            .as_deref()
            .expect("remedy")
            .contains("rafters init"));
        // Still documented -- the observation reports, it never gates.
        assert_eq!(sub.docs.len(), 1);
    }
}
