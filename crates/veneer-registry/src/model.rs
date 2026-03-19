use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Namespace token file: .rafters/tokens/{namespace}.rafters.json
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NamespaceTokenFile {
    #[serde(rename = "$schema", default)]
    pub schema: Option<String>,
    pub namespace: String,
    pub version: String,
    #[serde(default)]
    pub generated_at: Option<String>,
    #[serde(default)]
    pub tokens: Vec<Token>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Token {
    pub name: String,
    pub value: TokenValue,
    pub category: String,
    pub namespace: String,

    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub line_height: Option<String>,
    #[serde(default)]
    pub semantic_meaning: Option<String>,
    #[serde(default)]
    pub usage_context: Option<Vec<String>>,
    #[serde(default)]
    pub trust_level: Option<String>,
    #[serde(default)]
    pub cognitive_load: Option<f64>,
    #[serde(default)]
    pub accessibility_level: Option<String>,
    #[serde(default)]
    pub consequence: Option<String>,
    #[serde(default)]
    pub applies_when: Option<Vec<String>>,
    #[serde(default)]
    pub usage_patterns: Option<UsagePatterns>,
    #[serde(default)]
    pub user_override: Option<UserOverride>,
    #[serde(default)]
    pub computed_value: Option<TokenValue>,
    #[serde(default)]
    pub depends_on: Option<Vec<String>>,
    #[serde(default)]
    pub generation_rule: Option<String>,
    #[serde(default)]
    pub progression_system: Option<String>,
    #[serde(default)]
    pub scale_position: Option<f64>,
    #[serde(default)]
    pub math_relationship: Option<String>,
    #[serde(default)]
    pub container_query_aware: Option<bool>,
    #[serde(default)]
    pub reduced_motion_aware: Option<bool>,
    #[serde(default)]
    pub applicable_components: Option<Vec<String>>,
    #[serde(default)]
    pub required_for_components: Option<Vec<String>>,
    #[serde(default)]
    pub paired_with: Option<Vec<String>>,
    #[serde(default)]
    pub conflicts_with: Option<Vec<String>>,
    #[serde(default)]
    pub deprecated: Option<bool>,
    #[serde(default)]
    pub motion_intent: Option<String>,
    #[serde(default)]
    pub easing_curve: Option<[f64; 4]>,
    #[serde(default)]
    pub easing_name: Option<String>,
    #[serde(default)]
    pub elevation_level: Option<String>,
    #[serde(default)]
    pub shadow_token: Option<String>,
    #[serde(default)]
    pub generate_utility_class: Option<bool>,
    #[serde(default)]
    pub tailwind_override: Option<bool>,
    #[serde(default)]
    pub custom_property_only: Option<bool>,
    #[serde(default)]
    pub interaction_type: Option<String>,
    #[serde(default)]
    pub focus_ring_width: Option<String>,
    #[serde(default)]
    pub focus_ring_color: Option<String>,
    #[serde(default)]
    pub focus_ring_offset: Option<String>,
    #[serde(default)]
    pub focus_ring_style: Option<String>,
    #[serde(default)]
    pub keyframe_name: Option<String>,
    #[serde(default)]
    pub animation_name: Option<String>,
    #[serde(default)]
    pub animation_duration: Option<String>,
    #[serde(default)]
    pub animation_easing: Option<String>,
    #[serde(default)]
    pub animation_iterations: Option<String>,
}

/// Token value: either a plain string, a color value object, or a color reference.
/// Order matters for untagged deserialization: try ColorValue first (most specific),
/// then ColorReference, then fall back to String.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TokenValue {
    Color(Box<ColorValue>),
    Reference(ColorReference),
    String(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorValue {
    pub name: String,
    pub scale: Vec<OKLCH>,

    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(rename = "use", default)]
    pub use_field: Option<String>,
    #[serde(default)]
    pub states: Option<serde_json::Value>,
    #[serde(default)]
    pub intelligence: Option<ColorIntelligence>,
    #[serde(default)]
    pub harmonies: Option<serde_json::Value>,
    #[serde(default)]
    pub accessibility: Option<serde_json::Value>,
    #[serde(default)]
    pub analysis: Option<serde_json::Value>,
    #[serde(default)]
    pub atmospheric_weight: Option<f64>,
    #[serde(default)]
    pub perceptual_weight: Option<f64>,
    #[serde(default)]
    pub semantic_suggestions: Option<serde_json::Value>,
    #[serde(default)]
    pub token_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OKLCH {
    pub l: f64,
    pub c: f64,
    pub h: f64,
    #[serde(default)]
    pub alpha: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorIntelligence {
    #[serde(default)]
    pub reasoning: Option<String>,
    #[serde(default)]
    pub emotional_impact: Option<String>,
    #[serde(default)]
    pub cultural_context: Option<String>,
    #[serde(default)]
    pub accessibility_notes: Option<String>,
    #[serde(default)]
    pub usage_guidance: Option<String>,
    #[serde(default)]
    pub balancing_guidance: Option<String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorReference {
    pub family: String,
    pub position: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsagePatterns {
    #[serde(rename = "do")]
    pub do_patterns: Vec<String>,
    #[serde(rename = "never")]
    pub never_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOverride {
    pub previous_value: TokenValue,
    pub reason: String,
    #[serde(default)]
    pub context: Option<String>,
}

// ---------------------------------------------------------------------------
// Registry components: .rafters/registry/{components,primitives,composites}/
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryItem {
    pub name: String,
    #[serde(rename = "type")]
    pub item_type: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub primitives: Vec<String>,
    #[serde(default)]
    pub files: Vec<RegistryFile>,
    #[serde(default)]
    pub rules: Vec<String>,
    #[serde(default)]
    pub composites: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryFile {
    pub path: String,
    pub content: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub dev_dependencies: Vec<String>,
}

// ---------------------------------------------------------------------------
// Registry index: .rafters/registry/index.json
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryIndex {
    pub name: String,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub components: Vec<String>,
    #[serde(default)]
    pub primitives: Vec<String>,
    #[serde(default)]
    pub composites: Vec<String>,
    #[serde(default)]
    pub rules: Vec<String>,
}

// ---------------------------------------------------------------------------
// Config: .rafters/config.rafters.json
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RaftersConfig {
    #[serde(default)]
    pub framework: Option<String>,
    #[serde(default)]
    pub component_target: Option<String>,
    #[serde(default)]
    pub components_path: Option<String>,
    #[serde(default)]
    pub primitives_path: Option<String>,
    #[serde(default)]
    pub composites_path: Option<String>,
    #[serde(default)]
    pub css_path: Option<String>,
    #[serde(default)]
    pub shadcn: Option<bool>,
    #[serde(default)]
    pub exports: Option<ExportsConfig>,
    #[serde(default)]
    pub installed: Option<InstalledConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportsConfig {
    #[serde(default)]
    pub tailwind: Option<bool>,
    #[serde(default)]
    pub typescript: Option<bool>,
    #[serde(default)]
    pub dtcg: Option<bool>,
    #[serde(default)]
    pub compiled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledConfig {
    #[serde(default)]
    pub components: Vec<String>,
    #[serde(default)]
    pub primitives: Vec<String>,
    #[serde(default)]
    pub composites: Vec<String>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_token_string_value() {
        let json = r#"{
            "name": "spacing-4",
            "value": "1rem",
            "category": "spacing",
            "namespace": "spacing"
        }"#;
        let token: Token = serde_json::from_str(json).unwrap();
        assert_eq!(token.name, "spacing-4");
        assert!(matches!(token.value, TokenValue::String(ref s) if s == "1rem"));
        assert_eq!(token.category, "spacing");
    }

    #[test]
    fn deserialize_token_color_value() {
        let json = r#"{
            "name": "color-neutral-500",
            "value": {
                "name": "neutral",
                "scale": [
                    { "l": 0.97, "c": 0.003, "h": 264.5 },
                    { "l": 0.50, "c": 0.005, "h": 264.5, "alpha": 0.8 }
                ],
                "token": "neutral-500",
                "atmosphericWeight": 0.45
            },
            "category": "color",
            "namespace": "color"
        }"#;
        let token: Token = serde_json::from_str(json).unwrap();
        assert_eq!(token.name, "color-neutral-500");
        match &token.value {
            TokenValue::Color(cv) => {
                assert_eq!(cv.name, "neutral");
                assert_eq!(cv.scale.len(), 2);
                assert!((cv.scale[0].l - 0.97).abs() < f64::EPSILON);
                assert_eq!(cv.scale[1].alpha, Some(0.8));
                assert_eq!(cv.atmospheric_weight, Some(0.45));
            }
            other => panic!("expected ColorValue, got {:?}", other),
        }
    }

    #[test]
    fn deserialize_token_color_reference() {
        let json = r#"{
            "name": "semantic-primary",
            "value": { "family": "blue", "position": "500" },
            "category": "color",
            "namespace": "semantic"
        }"#;
        let token: Token = serde_json::from_str(json).unwrap();
        match &token.value {
            TokenValue::Reference(cr) => {
                assert_eq!(cr.family, "blue");
                assert_eq!(cr.position, "500");
            }
            other => panic!("expected ColorReference, got {:?}", other),
        }
    }

    #[test]
    fn deserialize_usage_patterns() {
        let json = r#"{
            "name": "color-error-500",
            "value": "oklch(0.63 0.24 29)",
            "category": "color",
            "namespace": "semantic",
            "usagePatterns": {
                "do": ["Use for error states", "Use for destructive actions"],
                "never": ["Use as a primary brand color"]
            }
        }"#;
        let token: Token = serde_json::from_str(json).unwrap();
        let patterns = token.usage_patterns.expect("usagePatterns should be present");
        assert_eq!(patterns.do_patterns.len(), 2);
        assert_eq!(patterns.never_patterns.len(), 1);
        assert_eq!(patterns.do_patterns[0], "Use for error states");
    }

    #[test]
    fn deserialize_namespace_token_file() {
        let json = r#"{
            "$schema": "https://rafters.studio/schemas/namespace-tokens.json",
            "namespace": "color",
            "version": "1.0.0",
            "generatedAt": "2026-03-18T00:00:00Z",
            "tokens": [
                {
                    "name": "color-neutral-50",
                    "value": "oklch(0.97 0.003 264.5)",
                    "category": "color",
                    "namespace": "color"
                }
            ]
        }"#;
        let file: NamespaceTokenFile = serde_json::from_str(json).unwrap();
        assert_eq!(file.namespace, "color");
        assert_eq!(file.version, "1.0.0");
        assert_eq!(file.tokens.len(), 1);
    }

    #[test]
    fn deserialize_registry_item() {
        let json = r#"{
            "name": "button",
            "type": "ui",
            "description": "Interactive button component",
            "primitives": ["slot", "merge-class"],
            "files": [{
                "path": "components/ui/button.tsx",
                "content": "export function Button() {}",
                "dependencies": ["react@19.2.0"],
                "devDependencies": []
            }],
            "rules": [],
            "composites": []
        }"#;
        let item: RegistryItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.name, "button");
        assert_eq!(item.item_type, "ui");
        assert_eq!(item.files.len(), 1);
        assert_eq!(item.files[0].dependencies[0], "react@19.2.0");
    }

    #[test]
    fn deserialize_registry_index() {
        let json = r#"{
            "name": "rafters",
            "homepage": "https://rafters.studio",
            "components": ["button", "card"],
            "primitives": ["slot"],
            "composites": [],
            "rules": []
        }"#;
        let index: RegistryIndex = serde_json::from_str(json).unwrap();
        assert_eq!(index.name, "rafters");
        assert_eq!(index.components, vec!["button", "card"]);
    }

    #[test]
    fn deserialize_config() {
        let json = r#"{
            "framework": "astro",
            "componentTarget": "react",
            "componentsPath": "src/components/ui",
            "primitivesPath": "src/lib/primitives",
            "compositesPath": "src/composites",
            "cssPath": "src/styles/global.css",
            "shadcn": true,
            "exports": { "tailwind": true, "typescript": true, "dtcg": false, "compiled": false },
            "installed": { "components": ["button", "card"], "primitives": ["slot"], "composites": [] }
        }"#;
        let config: RaftersConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.framework, Some("astro".to_string()));
        assert_eq!(config.shadcn, Some(true));
        let exports = config.exports.unwrap();
        assert_eq!(exports.tailwind, Some(true));
        assert_eq!(exports.dtcg, Some(false));
        let installed = config.installed.unwrap();
        assert_eq!(installed.components, vec!["button", "card"]);
    }

    #[test]
    fn deserialize_token_with_intelligence() {
        let json = r#"{
            "name": "color-blue-500",
            "value": {
                "name": "blue",
                "scale": [{ "l": 0.6, "c": 0.2, "h": 250.0 }],
                "intelligence": {
                    "reasoning": "Primary brand color",
                    "emotionalImpact": "Trust and calm",
                    "culturalContext": "Universal trust signal",
                    "accessibilityNotes": "Good contrast on white",
                    "usageGuidance": "Use for primary actions",
                    "metadata": { "source": "brand-guide-v2" }
                }
            },
            "category": "color",
            "namespace": "color"
        }"#;
        let token: Token = serde_json::from_str(json).unwrap();
        match &token.value {
            TokenValue::Color(cv) => {
                let intel = cv.intelligence.as_ref().unwrap();
                assert_eq!(intel.reasoning, Some("Primary brand color".to_string()));
                assert!(intel.metadata.is_some());
            }
            other => panic!("expected ColorValue, got {:?}", other),
        }
    }

    #[test]
    fn deserialize_user_override() {
        let json = r#"{
            "name": "color-brand-500",
            "value": "oklch(0.6 0.2 250)",
            "category": "color",
            "namespace": "color",
            "userOverride": {
                "previousValue": "oklch(0.5 0.15 240)",
                "reason": "Better contrast ratio",
                "context": "a11y audit"
            }
        }"#;
        let token: Token = serde_json::from_str(json).unwrap();
        let uo = token.user_override.unwrap();
        assert!(matches!(uo.previous_value, TokenValue::String(ref s) if s == "oklch(0.5 0.15 240)"));
        assert_eq!(uo.reason, "Better contrast ratio");
        assert_eq!(uo.context, Some("a11y audit".to_string()));
    }
}
