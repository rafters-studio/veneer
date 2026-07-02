//! Agent-complete intelligence artifact (FR-VEN-016): serialization of
//! [`CompiledIntelligence`] to a stable, versioned JSON artifact per
//! component or composite, at parity with what the rafters MCP serves.
//!
//! Parity grounding: the field set was captured from a live
//! `rafters_component` MCP call for `button` (2026-07-01). The MCP's
//! `intelligence` block serves exactly these fields (the canonical six of
//! `packages/shared/src/component-intelligence.ts`):
//!
//! - `cognitiveLoad`            -> [`IntelligenceArtifact::cognitive_load`]
//! - `attentionEconomics`       -> [`IntelligenceArtifact::attention_economics`]
//! - `trustBuilding`            -> [`IntelligenceArtifact::trust_building`]
//! - `accessibility`            -> [`IntelligenceArtifact::accessibility`]
//! - `semanticMeaning`          -> [`IntelligenceArtifact::semantic_meaning`]
//! - `usagePatterns.dos/nevers` -> [`IntelligenceArtifact::do_never`]
//!
//! The MCP additionally serves per-file `dependencies`, covered by
//! [`IntelligenceArtifact::dependencies`]. Its remaining top-level fields
//! (`files[].content`, `primitives`, `rules`, `composites`) are source
//! distribution, not intelligence, and are out of scope here. The MCP has
//! no token-read tool today, so `tokens` and `override_reasons` are
//! additive, cross-checked against the `.rafters/` namespace source only.
//!
//! Absence is explicit: a field genuinely absent from source serializes as
//! `{"status": "absent_from_source"}`, never silently omitted -- an agent
//! must see where the answers stop (journey.agent.phase-4).
//!
//! Output is deterministic: fixed field order, sorted collections upstream
//! (see `intelligence.rs`), no timestamps. Identical input produces
//! byte-identical artifacts, so they diff cleanly in CI.
//!
//! This module is static-form only. Serving the data live is the MCP's
//! job; veneer keeps its zero platform footprint (FR-VEN-012).

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use crate::intelligence::{
    family_files, jsdoc_blocks, jsdoc_tags, read_source_file, CognitiveLoad, CompiledIntelligence,
    Constraint, DependencyRef, PropDoc, TokenRef, VariantDoc,
};
use crate::rafters_source::IntelligenceSource;
use crate::registry::{is_composite_manifest, DiscoveredItem, DiscoveredKind};
use crate::ts_helpers::normalize_whitespace;

/// Version of the artifact JSON schema. Bump on any shape change so agents
/// can detect what they are reading.
pub const ARTIFACT_SCHEMA_VERSION: &str = "1";

/// Errors while building or emitting an intelligence artifact. Every
/// variant names the component; a partial artifact is never emitted
/// silently (emission serializes fully before the first byte is written).
#[derive(Debug, thiserror::Error)]
pub enum ArtifactError {
    /// The component's source file (or a family sibling) could not be read
    /// while extracting the MCP prose intelligence tags.
    #[error("failed to read intelligence source for {component}: {reason}")]
    SourceUnreadable { component: String, reason: String },
    /// A field of the artifact failed to serialize. Names the component
    /// and the field, per FR-VEN-016 error handling.
    #[error("failed to serialize field \"{field}\" of {component}: {reason}")]
    Serialization {
        component: String,
        field: String,
        reason: String,
    },
    /// The fully serialized artifact could not be written to disk.
    #[error("failed to write artifact for {component} at {path}: {reason}")]
    Write {
        component: String,
        path: PathBuf,
        reason: String,
    },
}

/// Absence is explicit -- an agent must see where the answers stop.
///
/// Serializes as `{"status": "present", "value": ...}` or
/// `{"status": "absent_from_source"}`: the field key always exists in the
/// artifact, so absence is data, never an omission.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "status", content = "value", rename_all = "snake_case")]
pub enum FieldValue<T> {
    Present(T),
    /// genuinely absent from source -- marked, never silently omitted
    AbsentFromSource,
}

impl<T> FieldValue<T> {
    /// True when the field carries a value.
    pub fn is_present(&self) -> bool {
        matches!(self, FieldValue::Present(_))
    }
}

/// `Some` becomes `Present`; a `None` the source never declared becomes
/// explicit absence.
fn from_option<T>(value: Option<T>) -> FieldValue<T> {
    match value {
        Some(value) => FieldValue::Present(value),
        None => FieldValue::AbsentFromSource,
    }
}

/// An empty collection means the source declares nothing for the field:
/// that is absence, not an empty answer.
fn from_collection<T>(items: Vec<T>) -> FieldValue<Vec<T>> {
    if items.is_empty() {
        FieldValue::AbsentFromSource
    } else {
        FieldValue::Present(items)
    }
}

/// One user override carried by a namespace token the component references,
/// from `token.userOverride` in the `.rafters/` namespace source -- the
/// data the DTCG export drops (FR-VEN-002). `reason` mirrors the source:
/// an override without a declared reason is still reported, with the
/// reason explicitly `null`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OverrideReason {
    /// Token name as declared in the namespace source.
    pub token: String,
    /// Namespace that declares the token.
    pub namespace: String,
    /// The declared override reason, verbatim.
    pub reason: Option<String>,
    /// The value the override replaced, verbatim from source.
    pub previous_value: Option<Value>,
}

/// The compiled intelligence of one component or composite as a versioned,
/// machine-readable artifact. Field-for-field this covers every
/// intelligence field the rafters MCP serves (see the module docs for the
/// captured mapping) plus the namespace-source token data the MCP does not
/// serve yet.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IntelligenceArtifact {
    /// Artifact schema version ([`ARTIFACT_SCHEMA_VERSION`]).
    pub schema_version: String,
    /// Declared component or composite name.
    pub component: String,
    /// Component or composite, from what the source declares.
    pub kind: DiscoveredKind,
    /// Declared props. Empty when no `*Props` interface exists -- props
    /// and variants are structural declarations, so an empty list IS the
    /// complete answer, unlike the `FieldValue` fields below.
    pub props: Vec<PropDoc>,
    /// Declared variants with the classes they map to.
    pub variants: Vec<VariantDoc>,
    /// `@cognitive-load` (or composite manifest `cognitiveLoad`).
    pub cognitive_load: FieldValue<CognitiveLoad>,
    /// `@attention-economics` prose, served by the MCP as
    /// `intelligence.attentionEconomics`.
    pub attention_economics: FieldValue<String>,
    /// `@trust-building` prose, served by the MCP as
    /// `intelligence.trustBuilding`.
    pub trust_building: FieldValue<String>,
    /// `@accessibility` prose, served by the MCP as
    /// `intelligence.accessibility`.
    pub accessibility: FieldValue<String>,
    /// `@semantic-meaning` prose, served by the MCP as
    /// `intelligence.semanticMeaning`.
    pub semantic_meaning: FieldValue<String>,
    /// DO/NEVER constraints (`@usage-patterns`, legacy `@do`/`@never`, or
    /// a composite manifest's `usagePatterns`).
    pub do_never: FieldValue<Vec<Constraint>>,
    /// Namespace tokens the component's classes reference by exact name.
    pub tokens: FieldValue<Vec<TokenRef>>,
    /// Declared dependencies (imports and `@dependencies` tags).
    pub dependencies: FieldValue<Vec<DependencyRef>>,
    /// User overrides carried by the referenced tokens, with reasons.
    pub override_reasons: FieldValue<Vec<OverrideReason>>,
}

impl IntelligenceArtifact {
    /// Serialize the artifact to its canonical JSON form: pretty-printed,
    /// fields in declaration order, trailing newline, no timestamps.
    ///
    /// A serialization failure names the component and the failing field
    /// ([`ArtifactError::Serialization`]); nothing partial is returned.
    pub fn to_json(&self) -> Result<String, ArtifactError> {
        // Per-field pass first, solely so a failure can name the field --
        // serde_json's own error carries no field path.
        self.named_field_check("schema_version", &self.schema_version)?;
        self.named_field_check("component", &self.component)?;
        self.named_field_check("kind", &self.kind)?;
        self.named_field_check("props", &self.props)?;
        self.named_field_check("variants", &self.variants)?;
        self.named_field_check("cognitive_load", &self.cognitive_load)?;
        self.named_field_check("attention_economics", &self.attention_economics)?;
        self.named_field_check("trust_building", &self.trust_building)?;
        self.named_field_check("accessibility", &self.accessibility)?;
        self.named_field_check("semantic_meaning", &self.semantic_meaning)?;
        self.named_field_check("do_never", &self.do_never)?;
        self.named_field_check("tokens", &self.tokens)?;
        self.named_field_check("dependencies", &self.dependencies)?;
        self.named_field_check("override_reasons", &self.override_reasons)?;

        let mut json =
            serde_json::to_string_pretty(self).map_err(|error| ArtifactError::Serialization {
                component: self.component.clone(),
                field: "artifact".to_string(),
                reason: error.to_string(),
            })?;
        json.push('\n');
        Ok(json)
    }

    /// Serialize one field, naming it on failure.
    fn named_field_check<T: Serialize>(
        &self,
        field: &'static str,
        data: &T,
    ) -> Result<(), ArtifactError> {
        serde_json::to_value(data)
            .map(|_| ())
            .map_err(|error| ArtifactError::Serialization {
                component: self.component.clone(),
                field: field.to_string(),
                reason: error.to_string(),
            })
    }
}

/// Build the intelligence artifact for one rendered component or composite:
/// the [`CompiledIntelligence`] fields, the MCP prose intelligence tags
/// read from the item's source, and the override reasons carried by the
/// namespace tokens the component references. Every field holds exactly
/// what the source declares; absence is marked, never synthesized.
pub fn build_artifact(
    item: &DiscoveredItem,
    intelligence: &CompiledIntelligence,
    source: &IntelligenceSource,
) -> Result<IntelligenceArtifact, ArtifactError> {
    let prose = read_prose_tags(item)?;
    let overrides = override_reasons(&intelligence.tokens, source);

    Ok(IntelligenceArtifact {
        schema_version: ARTIFACT_SCHEMA_VERSION.to_string(),
        component: item.name.clone(),
        kind: item.kind,
        props: intelligence.props.clone(),
        variants: intelligence.variants.clone(),
        cognitive_load: from_option(intelligence.cognitive_load.clone()),
        attention_economics: from_option(prose.attention_economics),
        trust_building: from_option(prose.trust_building),
        accessibility: from_option(prose.accessibility),
        semantic_meaning: from_option(prose.semantic_meaning),
        do_never: from_collection(intelligence.do_never.clone()),
        tokens: from_collection(intelligence.tokens.clone()),
        dependencies: from_collection(intelligence.dependencies.clone()),
        override_reasons: from_collection(overrides),
    })
}

/// Emit one artifact as `<component>.intelligence.json` under `output_dir`
/// (the per-component extract/watch output tree). The artifact serializes
/// fully before anything touches disk, so a partial artifact is never
/// emitted silently.
pub fn write_artifact(
    artifact: &IntelligenceArtifact,
    output_dir: &Path,
) -> Result<PathBuf, ArtifactError> {
    let json = artifact.to_json()?;
    let path = output_dir.join(format!("{}.intelligence.json", artifact.component));
    let write_error = |error: std::io::Error| ArtifactError::Write {
        component: artifact.component.clone(),
        path: path.clone(),
        reason: error.to_string(),
    };
    fs::create_dir_all(output_dir).map_err(&write_error)?;
    fs::write(&path, json).map_err(&write_error)?;
    Ok(path)
}

/// The four MCP prose intelligence tags [`CompiledIntelligence`] does not
/// carry. `None` exactly when the source declares no such tag.
#[derive(Debug, Default)]
struct McpProseTags {
    attention_economics: Option<String>,
    trust_building: Option<String>,
    accessibility: Option<String>,
    semantic_meaning: Option<String>,
}

/// Read the MCP prose tags from the item's source file and its same-stem
/// family files (the same family walk `intelligence.rs` uses). The item's
/// own file is read first and the first declaration of each tag wins, so
/// own-file prose takes precedence over a sibling's. Composite manifests
/// declare none of these fields (verified against the real
/// `packages/ui/src/composites/*.composite.json` manifests), so a manifest
/// item renders them absent.
fn read_prose_tags(item: &DiscoveredItem) -> Result<McpProseTags, ArtifactError> {
    let mut tags = McpProseTags::default();

    if is_composite_manifest(&item.source_path) {
        return Ok(tags);
    }

    let read = |path: &Path| {
        read_source_file(path).map_err(|reason| ArtifactError::SourceUnreadable {
            component: item.name.clone(),
            reason,
        })
    };

    collect_prose_tags(&read(&item.source_path)?, &mut tags);
    for sibling in family_files(&item.source_path) {
        collect_prose_tags(&read(&sibling)?, &mut tags);
    }
    Ok(tags)
}

/// Scan every JSDoc block of one source file for the four MCP prose tags,
/// merging first-wins into `tags`. Tag names and their no-hyphen aliases
/// match the canonical rafters parser
/// (`packages/shared/src/component-intelligence.ts`).
fn collect_prose_tags(source: &str, tags: &mut McpProseTags) {
    for block in jsdoc_blocks(source) {
        for tag in jsdoc_tags(&block) {
            let slot = match tag.name.as_str() {
                "attention-economics" | "attentioneconomics" => &mut tags.attention_economics,
                "trust-building" | "trustbuilding" => &mut tags.trust_building,
                "accessibility" => &mut tags.accessibility,
                "semantic-meaning" | "semanticmeaning" => &mut tags.semantic_meaning,
                _ => continue,
            };
            if slot.is_none() {
                let value = normalize_whitespace(&tag.value);
                if !value.is_empty() {
                    *slot = Some(value);
                }
            }
        }
    }
}

/// Join the tokens the component references to the `userOverride` data the
/// namespace source declares for them. Order follows `tokens` (already
/// sorted), so the result is deterministic. Only tokens that actually
/// carry an override contribute; with no namespace source there is nothing
/// declared to read.
fn override_reasons(tokens: &[TokenRef], source: &IntelligenceSource) -> Vec<OverrideReason> {
    let IntelligenceSource::Namespace(namespace) = source else {
        return Vec::new();
    };

    let mut reasons: Vec<OverrideReason> = Vec::new();
    for token_ref in tokens {
        let Some(file) = namespace.namespaces.get(&token_ref.namespace) else {
            continue;
        };
        let Some(token) = file.tokens.iter().find(|t| t.name == token_ref.token) else {
            continue;
        };
        let Some(user_override) = &token.user_override else {
            continue;
        };
        reasons.push(OverrideReason {
            token: token_ref.token.clone(),
            namespace: token_ref.namespace.clone(),
            reason: user_override.reason.clone(),
            previous_value: user_override.previous_value.clone(),
        });
    }
    reasons
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intelligence::render_component;
    use crate::rafters_source::read_rafters_namespace;
    use crate::registry::ComponentRegistry;

    fn fixture_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/artifact/project")
    }

    fn discovered_items() -> (Vec<DiscoveredItem>, IntelligenceSource) {
        let root = fixture_root();
        let source = read_rafters_namespace(&root).expect("fixture namespace must read");
        let items =
            ComponentRegistry::discover(&root, &source).expect("fixture discovery must succeed");
        (items, source)
    }

    fn artifact_named(name: &str) -> IntelligenceArtifact {
        let (items, source) = discovered_items();
        let item = items
            .iter()
            .find(|item| item.name.eq_ignore_ascii_case(name))
            .unwrap_or_else(|| panic!("fixture must discover {name}"))
            .clone();
        let rendered = render_component(&item, &source)
            .unwrap_or_else(|error| panic!("{name} must render: {error}"));
        build_artifact(&item, &rendered.intelligence, &source)
            .unwrap_or_else(|error| panic!("{name} artifact must build: {error}"))
    }

    fn json_value(artifact: &IntelligenceArtifact) -> Value {
        let json = artifact.to_json().expect("artifact must serialize");
        serde_json::from_str(&json).expect("artifact JSON must parse")
    }

    // AC: the full intelligence is consumable as machine-readable data per
    // component -- no prose scraping.
    #[test]
    fn full_intelligence_is_machine_readable_data() {
        let artifact = artifact_named("Button");
        let value = json_value(&artifact);

        assert_eq!(value["schema_version"], "1");
        assert_eq!(value["component"], "Button");
        assert_eq!(value["kind"], "component");

        // Props as data.
        assert_eq!(value["props"][0]["name"], "variant");
        assert_eq!(value["props"][0]["optional"], true);
        assert_eq!(value["props"][0]["type_text"], "'default' | 'secondary'");

        // Variants as data.
        assert_eq!(value["variants"][0]["name"], "default");
        assert!(value["variants"][0]["classes"]
            .as_str()
            .expect("classes are a string")
            .contains("bg-primary"));

        // Cognitive load as data.
        assert_eq!(value["cognitive_load"]["status"], "present");
        assert_eq!(value["cognitive_load"]["value"]["score"], 3);

        // The four MCP prose fields as data.
        assert_eq!(value["attention_economics"]["status"], "present");
        assert!(value["attention_economics"]["value"]
            .as_str()
            .expect("prose is a string")
            .contains("maximum 1 per section"));
        assert_eq!(value["trust_building"]["status"], "present");
        assert_eq!(value["accessibility"]["status"], "present");
        assert!(value["accessibility"]["value"]
            .as_str()
            .expect("prose is a string")
            .contains("WCAG AAA"));
        assert_eq!(value["semantic_meaning"]["status"], "present");

        // Do/never as data.
        assert_eq!(value["do_never"]["status"], "present");
        assert_eq!(value["do_never"]["value"][0]["kind"], "do");
        assert_eq!(value["do_never"]["value"][2]["kind"], "never");

        // Tokens as data.
        assert_eq!(value["tokens"]["status"], "present");
        assert_eq!(value["tokens"]["value"][0]["token"], "primary");
        assert_eq!(value["tokens"]["value"][0]["namespace"], "semantic");
        assert_eq!(
            value["tokens"]["value"][0]["referenced_by"][0],
            "bg-primary"
        );

        // Dependencies as data.
        assert_eq!(value["dependencies"]["status"], "present");
        assert_eq!(value["dependencies"]["value"][0]["name"], "react");
        assert_eq!(value["dependencies"]["value"][0]["origin"], "import");
        assert_eq!(
            value["dependencies"]["value"][1]["name"],
            "@radix-ui/react-slot"
        );
        assert_eq!(value["dependencies"]["value"][1]["origin"], "js_doc_tag");

        // Override reasons as data, from the namespace userOverride.
        assert_eq!(value["override_reasons"]["status"], "present");
        assert_eq!(value["override_reasons"]["value"][0]["token"], "primary");
        assert!(value["override_reasons"]["value"][0]["reason"]
            .as_str()
            .expect("reason is a string")
            .contains("true red as primary"));
        assert!(value["override_reasons"]["value"][0]["previous_value"].is_string());
    }

    // AC: the artifact field set covers every component intelligence field
    // the rafters MCP serves. The list below is the intelligence block of a
    // live rafters_component("button") response captured 2026-07-01, mapped
    // to the artifact key that carries it.
    #[test]
    fn field_set_covers_every_mcp_served_intelligence_field() {
        let mcp_parity: [(&str, &str); 6] = [
            ("cognitiveLoad", "cognitive_load"),
            ("attentionEconomics", "attention_economics"),
            ("trustBuilding", "trust_building"),
            ("accessibility", "accessibility"),
            ("semanticMeaning", "semantic_meaning"),
            ("usagePatterns", "do_never"),
        ];
        let value = json_value(&artifact_named("Button"));
        let object = value.as_object().expect("artifact is a JSON object");
        for (mcp_field, artifact_key) in mcp_parity {
            assert!(
                object.contains_key(artifact_key),
                "MCP field {mcp_field} has no artifact twin {artifact_key}"
            );
        }
        // The MCP also serves per-file dependencies.
        assert!(object.contains_key("dependencies"));
    }

    // AC: a field genuinely absent from source is marked absent explicitly,
    // never silently omitted.
    #[test]
    fn absent_fields_are_marked_never_omitted() {
        let artifact = artifact_named("Plain");
        let value = json_value(&artifact);
        let object = value.as_object().expect("artifact is a JSON object");

        for field in [
            "cognitive_load",
            "attention_economics",
            "trust_building",
            "accessibility",
            "semantic_meaning",
            "do_never",
            "tokens",
            "dependencies",
            "override_reasons",
        ] {
            let entry = object
                .get(field)
                .unwrap_or_else(|| panic!("{field} must exist in the artifact even when absent"));
            assert_eq!(
                entry["status"], "absent_from_source",
                "{field} must be marked absent"
            );
            assert!(
                entry.get("value").is_none(),
                "{field} carries no value when absent"
            );
        }
        // Structural declarations are complete empty lists, not absence.
        assert_eq!(value["props"], Value::Array(Vec::new()));
        assert_eq!(value["variants"], Value::Array(Vec::new()));
    }

    // AC: identical input produces byte-identical artifacts.
    #[test]
    fn output_is_deterministic_byte_identical() {
        let build = || {
            artifact_named("Button")
                .to_json()
                .expect("artifact must serialize")
        };
        let first = build();
        let second = build();
        assert_eq!(
            first, second,
            "two full pipeline runs must agree byte-for-byte"
        );
        assert!(
            !first.contains("generated_at") && !first.contains("generatedAt"),
            "no timestamps in the artifact"
        );
    }

    // The hand-checked emission path and the derived Serialize impl must
    // never disagree -- same intelligence, one shape.
    #[test]
    fn to_json_agrees_with_derived_serialization() {
        let artifact = artifact_named("Button");
        let via_to_json = json_value(&artifact);
        let via_derive = serde_json::to_value(&artifact).expect("derive path must serialize");
        assert_eq!(via_to_json, via_derive);
    }

    // Override reasons come only from tokens the component references and
    // that actually carry a userOverride in the namespace source.
    #[test]
    fn override_reasons_join_referenced_tokens_to_namespace_overrides() {
        let artifact = artifact_named("Button");
        let FieldValue::Present(reasons) = &artifact.override_reasons else {
            panic!("Button references overridden tokens");
        };
        let named: Vec<&str> = reasons.iter().map(|r| r.token.as_str()).collect();
        // primary and primary-ring carry userOverride in the fixture;
        // primary-foreground and secondary do not.
        assert_eq!(named, ["primary", "primary-ring"]);
        assert_eq!(reasons[0].namespace, "semantic");
        assert_eq!(
            reasons[1].reason.as_deref(),
            Some("Auto-cascaded from primary -> primary")
        );
    }

    // Composites go through the same artifact path: manifest intelligence
    // present, everything the manifest does not declare marked absent.
    #[test]
    fn composite_manifest_artifact_marks_undeclared_fields_absent() {
        let artifact = artifact_named("hero-banner");
        assert_eq!(artifact.kind, DiscoveredKind::Composite);
        assert!(artifact.cognitive_load.is_present());
        assert!(artifact.do_never.is_present());
        // Manifests declare none of these (verified against the real
        // rafters composite manifests).
        assert_eq!(artifact.attention_economics, FieldValue::AbsentFromSource);
        assert_eq!(artifact.trust_building, FieldValue::AbsentFromSource);
        assert_eq!(artifact.accessibility, FieldValue::AbsentFromSource);
        assert_eq!(artifact.semantic_meaning, FieldValue::AbsentFromSource);
        assert_eq!(artifact.tokens, FieldValue::AbsentFromSource);
        assert_eq!(artifact.dependencies, FieldValue::AbsentFromSource);
        assert_eq!(artifact.override_reasons, FieldValue::AbsentFromSource);
        assert!(artifact.props.is_empty());
        assert!(artifact.variants.is_empty());
    }

    // Emission writes the versioned JSON artifact per component into the
    // output tree, fully serialized before the first byte hits disk.
    #[test]
    fn write_artifact_emits_versioned_json_per_component() {
        let artifact = artifact_named("Button");
        let output = tempfile::tempdir().expect("tempdir");
        let path = write_artifact(&artifact, output.path()).expect("emission must succeed");
        assert!(path.ends_with("Button.intelligence.json"));
        let written = fs::read_to_string(&path).expect("artifact file must read");
        assert_eq!(written, artifact.to_json().expect("serialize"));
        assert!(written.starts_with("{\n  \"schema_version\": \"1\""));
    }

    // Error handling: a failure names the component; nothing partial and
    // nothing silent.
    #[test]
    fn unreadable_source_is_a_named_error() {
        let item = DiscoveredItem {
            name: "Ghost".to_string(),
            kind: DiscoveredKind::Component,
            source_path: fixture_root().join("components/does-not-exist.tsx"),
            generated: true,
        };
        let error = build_artifact(
            &item,
            &CompiledIntelligence::default(),
            &IntelligenceSource::NoSource,
        )
        .expect_err("a missing source file must be a named error");
        let message = error.to_string();
        assert!(message.contains("Ghost"), "{message}");
        assert!(message.contains("does-not-exist.tsx"), "{message}");
    }

    #[test]
    fn unwritable_output_is_a_named_error_and_no_partial_file() {
        let artifact = artifact_named("Button");
        let dir = tempfile::tempdir().expect("tempdir");
        // A file where the output directory should be.
        let blocker = dir.path().join("out");
        fs::write(&blocker, "not a directory").expect("write blocker");
        let error =
            write_artifact(&artifact, &blocker).expect_err("emission into a file must fail");
        assert!(error.to_string().contains("Button"), "{error}");
        assert!(
            !blocker.is_dir(),
            "nothing partial appears where the artifact would go"
        );
    }

    // With no namespace source there is nothing declared to join against:
    // tokens and override reasons are absent, not guessed.
    #[test]
    fn no_source_marks_token_data_absent() {
        let (items, _) = discovered_items();
        let item = items
            .iter()
            .find(|item| item.name.eq_ignore_ascii_case("Button"))
            .expect("Button discovered")
            .clone();
        let rendered = render_component(&item, &IntelligenceSource::NoSource)
            .expect("Button renders without a namespace source");
        let artifact = build_artifact(&item, &rendered.intelligence, &IntelligenceSource::NoSource)
            .expect("artifact builds without a namespace source");
        assert_eq!(artifact.tokens, FieldValue::AbsentFromSource);
        assert_eq!(artifact.override_reasons, FieldValue::AbsentFromSource);
        // Intelligence declared in the component source itself is still there.
        assert!(artifact.cognitive_load.is_present());
        assert!(artifact.accessibility.is_present());
    }

    // Drives the real rafters checkout when available. Run with:
    //   VENEER_REAL_RAFTERS_ROOT=/path/to/rafters \
    //     cargo test -p veneer-adapters -- --ignored real_rafters
    #[test]
    #[ignore = "requires a local rafters checkout via VENEER_REAL_RAFTERS_ROOT"]
    fn real_rafters_artifacts_are_complete_and_deterministic() {
        let Ok(root) = std::env::var("VENEER_REAL_RAFTERS_ROOT") else {
            eprintln!("VENEER_REAL_RAFTERS_ROOT not set; skipping");
            return;
        };
        let root = PathBuf::from(root);

        let emit_all = || -> Vec<(String, String)> {
            let source = read_rafters_namespace(&root).expect("real namespace must read");
            let items = ComponentRegistry::discover(&root, &source).expect("real discovery");
            let mut artifacts: Vec<(String, String)> = Vec::new();
            for item in &items {
                let Ok(rendered) = render_component(item, &source) else {
                    continue;
                };
                let artifact = build_artifact(item, &rendered.intelligence, &source)
                    .unwrap_or_else(|error| panic!("{} artifact must build: {error}", item.name));
                let json = artifact.to_json().expect("artifact must serialize");
                artifacts.push((item.name.clone(), json));
            }
            artifacts
        };

        let first = emit_all();
        let second = emit_all();
        assert_eq!(
            first, second,
            "artifacts must be byte-identical across runs"
        );
        assert!(!first.is_empty(), "the real checkout must emit artifacts");

        let with_prose = first
            .iter()
            .filter(|(_, json)| {
                json.contains("\"attention_economics\": {\n    \"status\": \"present\"")
            })
            .count();
        let with_overrides = first
            .iter()
            .filter(|(_, json)| {
                json.contains("\"override_reasons\": {\n    \"status\": \"present\"")
            })
            .count();
        eprintln!(
            "real rafters: {} artifacts, {} with attention economics prose, {} with override reasons",
            first.len(),
            with_prose,
            with_overrides
        );
        assert!(with_prose > 0, "old-constitution prose tags must surface");
        assert!(
            with_overrides > 0,
            "namespace override reasons must surface"
        );
    }
}
