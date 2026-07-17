//! Reader for the rafters component matrix (`docs/spec/matrix/components.jsonl`,
//! line schema `rafters.component-line/1`). The matrix is rafters' tracking
//! file for the component catalog: what each component is, what it composes,
//! its states and motion intents, per-target port status, and -- in the
//! `metadata` block -- the old-tree JSDoc header carried forward (cognitive
//! load, attention economics, trust, accessibility, semantics, do/never).
//!
//! This is veneer's canonical STANDALONE intelligence source (interface
//! contract with rafters, 2026-07-16): the intelligence that used to be read
//! from `.tsx` JSDoc now lives here, extracted and richer. veneer reads the
//! matrix; it never writes it. Every field is deserialized exactly as rafters
//! declares it -- an absent optional means the source declares nothing for
//! that field (honest-absence), never a synthesized value.
//!
//! The matrix carries components only; composites live in `*.composite.json`
//! manifests and are read elsewhere. Props and variant->class maps are not in
//! the matrix either: props come from the behavior's exported `Config`
//! interface, variants from `.classes.ts` -- both from the component source
//! tree, not from here.

use std::path::Path;

use serde::Deserialize;

/// The line-shape discriminator every matrix line carries. veneer refuses a
/// line whose schema it does not recognize rather than guess at its shape.
pub const COMPONENT_LINE_SCHEMA: &str = "rafters.component-line/1";

/// An error reading or parsing the matrix.
#[derive(Debug, thiserror::Error)]
pub enum MatrixError {
    #[error("failed to read matrix at {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    /// A line did not parse as a `rafters.component-line/1` object. Carries
    /// the 1-based line number so the offending line is locatable.
    #[error("matrix line {line}: {source}")]
    Parse {
        line: usize,
        #[source]
        source: serde_json::Error,
    },
    /// A line parsed but carried an unrecognized schema discriminator.
    #[error("matrix line {line}: unexpected schema {schema:?} (expected \"rafters.component-line/1\")")]
    Schema { line: usize, schema: String },
}

/// The archetype vocabulary rafters assigns each component. A closed set:
/// an unknown value is a matrix veneer was not built against and fails the
/// parse rather than rendering a guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Archetype {
    Static,
    SimpleInteractive,
    ToggleFamily,
    TextInputFamily,
    Disclosure,
    ModalOverlay,
    NonModalOverlay,
    MenuCollectionPopup,
    Compound,
}

impl Archetype {
    /// The kebab-case wire name, matching the matrix's own spelling. Kept in
    /// lockstep with the `serde(rename_all = "kebab-case")` above so the doc
    /// line carries exactly the archetype the matrix declares.
    pub fn as_str(self) -> &'static str {
        match self {
            Archetype::Static => "static",
            Archetype::SimpleInteractive => "simple-interactive",
            Archetype::ToggleFamily => "toggle-family",
            Archetype::TextInputFamily => "text-input-family",
            Archetype::Disclosure => "disclosure",
            Archetype::ModalOverlay => "modal-overlay",
            Archetype::NonModalOverlay => "non-modal-overlay",
            Archetype::MenuCollectionPopup => "menu-collection-popup",
            Archetype::Compound => "compound",
        }
    }
}

/// Whether a component has been ported to the new constitution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PortStatus {
    Ported,
    Pending,
}

/// Per-file port status for one framework target. `Verified` requires the
/// conformance suite green; `Missing` means no such layer exists to preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileStatus {
    Missing,
    Specced,
    Ported,
    Verified,
}

/// Published-artifact provenance (RFC 2026-07-10). Minted by the publish
/// pipeline; absent until an item is published, so this is `None` for every
/// line until the sidecar/provenance phase (phase 2) goes live.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Provenance {
    pub version: String,
    /// sha256 over the file bytes with the fingerprint line removed.
    pub fingerprint: String,
    /// ed25519 signature over the fingerprint, rafters publish key.
    #[serde(default)]
    pub signature: Option<String>,
}

/// Cognitive load as the matrix declares it: the 0-10 intrinsic score and an
/// optional one-line rationale.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MatrixCognitiveLoad {
    pub score: u8,
    #[serde(default)]
    pub note: Option<String>,
}

/// The `metadata` block: the old-tree JSDoc header carried forward, extracted
/// and never re-authored. Optional as a whole (a component with no old-tree
/// ancestor omits it) and optional per field, so honest-absence holds all the
/// way down.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentMetadata {
    /// The old-tree file the header was lifted from.
    pub source: String,
    /// The JSDoc lead prose (what the component is).
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub cognitive_load: Option<MatrixCognitiveLoad>,
    #[serde(default)]
    pub attention_economics: Option<String>,
    #[serde(default)]
    pub trust_building: Option<String>,
    #[serde(default)]
    pub accessibility: Option<String>,
    #[serde(default)]
    pub semantic_meaning: Option<String>,
    /// The `@usage-patterns` DO / NEVER lines, one per entry. Each entry is
    /// prefixed `DO:` or `NEVER:` (interface contract); the classifier reads
    /// the prefix.
    #[serde(default)]
    pub usage_patterns: Option<Vec<String>>,
}

/// The primitives a component composes: those the implementation imports today
/// (`current`, evidence) and those the port adds (`planned`). `current` is
/// veneer's dependency surface (interface contract: render primitives-used,
/// not npm deps).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Uses {
    pub current: Vec<String>,
    pub planned: Vec<String>,
    /// Set on controller-era components whose behavior lives in rejected code.
    #[serde(default)]
    pub note: Option<String>,
}

/// Motion facts: the utilities found in the current classes (`current`,
/// evidence) and the Spec 04 intents the port declares.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Motion {
    pub current: String,
    pub intents: Vec<String>,
}

/// Per-target behavior-layer port status. `wc` gates the Web Component
/// preview: a value other than `Ported`/`Verified` means there is no WC layer
/// to compile, so the preview is honestly absent rather than a failure.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BehaviorLayer {
    pub react: FileStatus,
    pub astro: FileStatus,
    pub wc: FileStatus,
    pub vue: FileStatus,
}

/// The port status per framework, plus what the old tree ships for interim use.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Frameworks {
    pub behavior_layer: BehaviorLayer,
    #[serde(default)]
    pub old_tree: Vec<String>,
}

/// One line of the matrix: the full `rafters.component-line/1` record for one
/// component. Field-for-field with rafters' `ComponentLineSchema`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentLine {
    pub schema: String,
    pub name: String,
    pub archetype: Archetype,
    pub status: PortStatus,
    #[serde(default)]
    pub provenance: Option<Provenance>,
    /// The old-tree JSDoc header. `None` when the component has no old-tree
    /// ancestor -- its intelligence is honestly absent, never synthesized.
    #[serde(default)]
    pub metadata: Option<ComponentMetadata>,
    /// One sentence: what it is.
    pub is: String,
    /// One sentence: what it does.
    pub does: String,
    /// State vocabulary, descriptive (open, checked, value...). Empty for
    /// statics. Not a variant->class map; those live in `.classes.ts`.
    pub states: Vec<String>,
    pub uses: Uses,
    pub motion: Motion,
    pub frameworks: Frameworks,
}

impl ComponentLine {
    /// Whether a Web Component preview can be compiled for this component:
    /// its WC behavior layer is ported or verified. When false, the preview is
    /// honestly absent.
    pub fn has_wc_preview(&self) -> bool {
        matches!(
            self.frameworks.behavior_layer.wc,
            FileStatus::Ported | FileStatus::Verified
        )
    }
}

/// Parse the matrix text (one JSON object per line) into its component lines,
/// in file order. Blank lines are skipped; every non-blank line must be a
/// `rafters.component-line/1` object or the parse fails, naming the line -- a
/// malformed catalog is loud, never silently short.
pub fn parse_matrix(text: &str) -> Result<Vec<ComponentLine>, MatrixError> {
    let mut lines = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        if raw.trim().is_empty() {
            continue;
        }
        let line_no = index + 1;
        let line: ComponentLine =
            serde_json::from_str(raw).map_err(|source| MatrixError::Parse {
                line: line_no,
                source,
            })?;
        if line.schema != COMPONENT_LINE_SCHEMA {
            return Err(MatrixError::Schema {
                line: line_no,
                schema: line.schema,
            });
        }
        lines.push(line);
    }
    Ok(lines)
}

/// Read and parse the matrix at `path`.
pub fn read_matrix(path: &Path) -> Result<Vec<ComponentLine>, MatrixError> {
    let text = std::fs::read_to_string(path).map_err(|source| MatrixError::Read {
        path: path.display().to_string(),
        source,
    })?;
    parse_matrix(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    // A static component with the full metadata block (alert-shaped).
    const ALERT_LINE: &str = r#"{"schema":"rafters.component-line/1","name":"alert","archetype":"static","status":"ported","is":"Inline status banner","does":"Displays feedback; role=alert announces","states":[],"uses":{"current":["classy"],"planned":[]},"motion":{"current":"","intents":[]},"frameworks":{"behaviorLayer":{"react":"verified","astro":"missing","wc":"missing","vue":"missing"},"oldTree":["astro","react","wc"]},"metadata":{"source":"src/old/ui/alert.tsx","description":"Status message component","cognitiveLoad":{"score":3,"note":"Simple message display"},"attentionEconomics":"Variant hierarchy","trustBuilding":"Clear feedback builds confidence","accessibility":"role=alert; never color-only","semanticMeaning":"Variant mapping","usagePatterns":["DO: keep messages concise","NEVER: use for non-status content"]}}"#;

    // A component with no old-tree metadata and a ported WC layer.
    const TOOLTIP_LINE: &str = r#"{"schema":"rafters.component-line/1","name":"tooltip","archetype":"non-modal-overlay","status":"ported","is":"Contextual hint on hover/focus","does":"Shows a floating label near its trigger","states":["open"],"uses":{"current":["classy","portal"],"planned":["disclosure"],"note":"controller-era"},"motion":{"current":"fade","intents":["enter: opacity"]},"frameworks":{"behaviorLayer":{"react":"verified","astro":"ported","wc":"ported","vue":"missing"}}}"#;

    #[test]
    fn parses_a_line_with_the_full_metadata_block() {
        let lines = parse_matrix(ALERT_LINE).expect("parse");
        assert_eq!(lines.len(), 1);
        let line = &lines[0];
        assert_eq!(line.name, "alert");
        assert_eq!(line.archetype, Archetype::Static);
        assert_eq!(line.status, PortStatus::Ported);

        let meta = line.metadata.as_ref().expect("metadata present");
        assert_eq!(meta.cognitive_load.as_ref().unwrap().score, 3);
        assert_eq!(meta.attention_economics.as_deref(), Some("Variant hierarchy"));
        assert_eq!(
            meta.usage_patterns.as_ref().unwrap(),
            &["DO: keep messages concise", "NEVER: use for non-status content"]
        );
    }

    #[test]
    fn honest_absence_a_line_without_metadata_parses_with_none() {
        let lines = parse_matrix(TOOLTIP_LINE).expect("parse");
        let line = &lines[0];
        assert!(line.metadata.is_none(), "no old-tree metadata is None, not synthesized");
        assert!(line.provenance.is_none(), "unpublished item has no provenance");
        assert_eq!(line.uses.current, ["classy", "portal"]);
        assert_eq!(line.uses.note.as_deref(), Some("controller-era"));
    }

    #[test]
    fn wc_port_status_gates_the_preview() {
        let ported = &parse_matrix(TOOLTIP_LINE).expect("parse")[0];
        assert!(ported.has_wc_preview(), "wc:ported can be previewed");

        let missing = &parse_matrix(ALERT_LINE).expect("parse")[0];
        assert!(!missing.has_wc_preview(), "wc:missing is honestly absent, not previewable");
    }

    #[test]
    fn blank_lines_are_skipped_and_order_is_preserved() {
        let text = format!("{ALERT_LINE}\n\n{TOOLTIP_LINE}\n");
        let lines = parse_matrix(&text).expect("parse");
        let names: Vec<&str> = lines.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, ["alert", "tooltip"]);
    }

    #[test]
    fn an_unknown_schema_discriminator_is_rejected_by_line() {
        let bad = r#"{"schema":"rafters.component-line/2","name":"x","archetype":"static","status":"pending","is":"a","does":"b","states":[],"uses":{"current":[],"planned":[]},"motion":{"current":"","intents":[]},"frameworks":{"behaviorLayer":{"react":"missing","astro":"missing","wc":"missing","vue":"missing"}}}"#;
        let err = parse_matrix(bad).expect_err("unknown schema must fail");
        assert!(matches!(err, MatrixError::Schema { line: 1, .. }), "got {err:?}");
    }

    #[test]
    fn a_malformed_line_fails_loudly_with_its_line_number() {
        let text = format!("{ALERT_LINE}\n{{not json}}");
        let err = parse_matrix(&text).expect_err("malformed line must fail");
        assert!(matches!(err, MatrixError::Parse { line: 2, .. }), "got {err:?}");
    }

    #[test]
    fn an_unknown_archetype_fails_rather_than_guessing() {
        let bad = r#"{"schema":"rafters.component-line/1","name":"x","archetype":"holographic","status":"pending","is":"a","does":"b","states":[],"uses":{"current":[],"planned":[]},"motion":{"current":"","intents":[]},"frameworks":{"behaviorLayer":{"react":"missing","astro":"missing","wc":"missing","vue":"missing"}}}"#;
        let err = parse_matrix(bad).expect_err("unknown archetype must fail");
        assert!(matches!(err, MatrixError::Parse { line: 1, .. }), "got {err:?}");
    }
}
