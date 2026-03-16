//! Parser for W3C Design Tokens Community Group (DTCG) token files.
//!
//! DTCG tokens use nested JSON objects where leaf nodes contain `$value` and `$type`
//! fields, and groups contain other groups or tokens. Keys starting with `$` are
//! reserved for token metadata.

use std::collections::HashMap;

use serde_json::Value;

/// Errors from token parsing.
#[derive(Debug, thiserror::Error)]
pub enum TokenParseError {
    #[error("Invalid JSON: {0}")]
    InvalidJson(String),
    #[error("Invalid DTCG format: {0}")]
    InvalidFormat(String),
}

/// A single design token.
#[derive(Debug, Clone)]
pub struct DesignToken {
    /// Token path (e.g., ["color", "neutral", "50"])
    pub path: Vec<String>,
    /// Token value
    pub value: String,
    /// Token type (color, dimension, etc.)
    pub token_type: String,
    /// Description
    pub description: String,
    /// Extensions (vendor-specific metadata)
    pub extensions: HashMap<String, Value>,
}

/// Parsed collection of design tokens from a DTCG file.
#[derive(Debug, Clone, Default)]
pub struct DesignTokens {
    /// All tokens, flattened with their full paths
    pub tokens: Vec<DesignToken>,
}

impl DesignTokens {
    /// Filter tokens by their `$type` value.
    pub fn by_type(&self, token_type: &str) -> Vec<&DesignToken> {
        self.tokens
            .iter()
            .filter(|t| t.token_type == token_type)
            .collect()
    }

    /// Filter tokens whose path starts with the given prefix segments.
    pub fn by_path_prefix(&self, prefix: &[&str]) -> Vec<&DesignToken> {
        self.tokens
            .iter()
            .filter(|t| {
                t.path.len() >= prefix.len()
                    && t.path.iter().zip(prefix.iter()).all(|(a, b)| a == b)
            })
            .collect()
    }

    /// Get tokens that have extensions for a specific vendor key,
    /// returning each matching token paired with its vendor extension value.
    pub fn get_extensions(&self, vendor: &str) -> Vec<(&DesignToken, &Value)> {
        self.tokens
            .iter()
            .filter_map(|t| t.extensions.get(vendor).map(|v| (t, v)))
            .collect()
    }
}

/// Parse a DTCG JSON token file into a flattened collection of design tokens.
pub fn parse_dtcg_tokens(source: &str) -> Result<DesignTokens, TokenParseError> {
    let root: Value =
        serde_json::from_str(source).map_err(|e| TokenParseError::InvalidJson(e.to_string()))?;

    let obj = root
        .as_object()
        .ok_or_else(|| TokenParseError::InvalidFormat("Root must be a JSON object".into()))?;

    let mut tokens = Vec::new();
    let path = Vec::new();
    walk_object(obj, &path, &mut tokens);

    Ok(DesignTokens { tokens })
}

/// Recursively walk a JSON object, collecting tokens.
///
/// If the object contains a `$value` key, it is a token leaf node.
/// Otherwise it is a group, and we recurse into non-`$`-prefixed children.
fn walk_object(
    obj: &serde_json::Map<String, Value>,
    current_path: &[String],
    tokens: &mut Vec<DesignToken>,
) {
    if obj.contains_key("$value") {
        // This is a token leaf
        let value = match &obj["$value"] {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };

        let token_type = obj
            .get("$type")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

        let description = obj
            .get("$description")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

        let extensions = match obj.get("$extensions").and_then(Value::as_object) {
            Some(ext_obj) => ext_obj
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            None => HashMap::new(),
        };

        tokens.push(DesignToken {
            path: current_path.to_vec(),
            value,
            token_type,
            description,
            extensions,
        });
    } else {
        // This is a group -- recurse into children, skipping $-prefixed keys
        for (key, value) in obj {
            if key.starts_with('$') {
                continue;
            }
            if let Some(child_obj) = value.as_object() {
                let mut child_path = current_path.to_vec();
                child_path.push(key.clone());
                walk_object(child_obj, &child_path, tokens);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_color_token() {
        let source = r##"{
          "primary": {
            "$value": "#2563eb",
            "$type": "color",
            "$description": "Primary brand color"
          }
        }"##;
        let tokens = parse_dtcg_tokens(source).unwrap();
        assert_eq!(tokens.tokens.len(), 1);
        assert_eq!(tokens.tokens[0].path, vec!["primary"]);
        assert_eq!(tokens.tokens[0].value, r##"#2563eb"##);
        assert_eq!(tokens.tokens[0].token_type, "color");
    }

    #[test]
    fn parses_nested_token_groups() {
        let source = r#"{
          "color": {
            "neutral": {
              "50": {
                "$value": "oklch(0.985 0 0)",
                "$type": "color"
              },
              "100": {
                "$value": "oklch(0.967 0 0)",
                "$type": "color"
              }
            }
          }
        }"#;
        let tokens = parse_dtcg_tokens(source).unwrap();
        assert_eq!(tokens.tokens.len(), 2);
        // JSON object key ordering is not guaranteed, so check both exist
        let paths: Vec<&Vec<String>> = tokens.tokens.iter().map(|t| &t.path).collect();
        assert!(paths.contains(&&vec![
            "color".to_string(),
            "neutral".to_string(),
            "50".to_string()
        ]));
        assert!(paths.contains(&&vec![
            "color".to_string(),
            "neutral".to_string(),
            "100".to_string()
        ]));
    }

    #[test]
    fn parses_extensions() {
        let source = r##"{
          "primary": {
            "$value": "#2563eb",
            "$type": "color",
            "$extensions": {
              "rafters": {
                "semanticMeaning": "Primary action color",
                "usagePatterns": {
                  "do": ["Use for primary actions"],
                  "never": ["Use for backgrounds"]
                }
              }
            }
          }
        }"##;
        let tokens = parse_dtcg_tokens(source).unwrap();
        assert!(!tokens.tokens[0].extensions.is_empty());
        assert!(tokens.tokens[0].extensions.contains_key("rafters"));
    }

    #[test]
    fn handles_object_values() {
        let source = r#"{
          "background": {
            "$value": {"family": "neutral", "position": "50"},
            "$type": "color",
            "$description": "Page background"
          }
        }"#;
        let tokens = parse_dtcg_tokens(source).unwrap();
        assert_eq!(tokens.tokens.len(), 1);
        // Object values should be serialized to string representation
        assert!(tokens.tokens[0].value.contains("neutral"));
    }

    #[test]
    fn filters_by_type() {
        let source = r##"{
          "primary": {"$value": "#2563eb", "$type": "color"},
          "spacing-sm": {"$value": "0.5rem", "$type": "dimension"},
          "secondary": {"$value": "#64748b", "$type": "color"}
        }"##;
        let tokens = parse_dtcg_tokens(source).unwrap();
        let colors = tokens.by_type("color");
        assert_eq!(colors.len(), 2);
    }

    #[test]
    fn filters_by_path_prefix() {
        let source = r##"{
          "color": {
            "neutral": {
              "50": {"$value": "#fafafa", "$type": "color"},
              "100": {"$value": "#f5f5f5", "$type": "color"}
            },
            "primary": {
              "500": {"$value": "#2563eb", "$type": "color"}
            }
          }
        }"##;
        let tokens = parse_dtcg_tokens(source).unwrap();
        let neutrals = tokens.by_path_prefix(&["color", "neutral"]);
        assert_eq!(neutrals.len(), 2);
    }

    #[test]
    fn handles_empty_json() {
        let tokens = parse_dtcg_tokens("{}").unwrap();
        assert!(tokens.tokens.is_empty());
    }

    #[test]
    fn errors_on_invalid_json() {
        let result = parse_dtcg_tokens("not json");
        assert!(matches!(result, Err(TokenParseError::InvalidJson(_))));
    }

    #[test]
    fn skips_dollar_prefixed_group_keys() {
        // $type, $description at group level should not be treated as child groups
        let source = r##"{
          "color": {
            "$description": "All colors",
            "primary": {
              "$value": "#2563eb",
              "$type": "color"
            }
          }
        }"##;
        let tokens = parse_dtcg_tokens(source).unwrap();
        assert_eq!(tokens.tokens.len(), 1);
        assert_eq!(tokens.tokens[0].path, vec!["color", "primary"]);
    }
}
