//! Shared helpers for parsing TypeScript AST expressions via oxc, plus
//! small name-shape utilities used across the extraction pipeline.

use oxc_ast::ast::{BinaryOperator, Expression, ObjectPropertyKind};

/// Marker for the dynamic part of a string composed at render time (for
/// example the `${tint}` hole in `` `text-quality-${tint}` ``). Internal to
/// extraction; never appears in returned class tokens.
pub(crate) const DYNAMIC_HOLE: char = '\u{FFFC}';

/// Kebab-case an item name: `SplitPanel` -> `split-panel`,
/// `hero-banner` -> `hero-banner`. Underscores and spaces become dashes;
/// consecutive separators collapse.
pub(crate) fn kebab_case(name: &str) -> String {
    let mut kebab = String::with_capacity(name.len());
    for character in name.chars() {
        if character.is_uppercase() {
            if !kebab.is_empty() && !kebab.ends_with('-') {
                kebab.push('-');
            }
            kebab.extend(character.to_lowercase());
        } else if character == '_' || character == ' ' {
            if !kebab.is_empty() && !kebab.ends_with('-') {
                kebab.push('-');
            }
        } else {
            kebab.push(character);
        }
    }
    kebab
}

/// Unwrap TSAsExpression, TSSatisfiesExpression, TSTypeAssertion, and
/// ParenthesizedExpression to reach the underlying value expression.
pub(crate) fn unwrap_type_expressions<'a>(expr: &'a Expression<'a>) -> &'a Expression<'a> {
    match expr {
        Expression::TSAsExpression(as_expr) => unwrap_type_expressions(&as_expr.expression),
        Expression::TSSatisfiesExpression(sat_expr) => {
            unwrap_type_expressions(&sat_expr.expression)
        }
        Expression::TSTypeAssertion(ta_expr) => unwrap_type_expressions(&ta_expr.expression),
        Expression::ParenthesizedExpression(paren) => unwrap_type_expressions(&paren.expression),
        other => other,
    }
}

/// Extract a fully static string value from an expression, handling string
/// literals, template literals, binary concatenation, and TS type wrappers.
/// Returns `None` when any part of the string is dynamic.
pub(crate) fn extract_string_value(expr: &Expression<'_>) -> Option<String> {
    class_template_value(expr).filter(|value| !value.contains(DYNAMIC_HOLE))
}

/// A class-shaped expression value split into its two scoping halves.
#[derive(Debug, Default)]
pub(crate) struct ClassValue {
    /// Space-joined tokens that are fully static in source.
    pub(crate) static_classes: String,
    /// `prefix-*` patterns for tokens composed dynamically at render
    /// (the Tailwind tree-shake caveat, FR-VEN-018).
    pub(crate) patterns: Vec<String>,
}

/// Convert one class token that may contain a [`DYNAMIC_HOLE`] into its
/// scopable form: a hole-free token passes through, a token with static
/// text before the hole becomes a `prefix-*` pattern, and a token with no
/// static prefix cannot be scoped (`None`).
pub(crate) fn scopable_class_token(token: &str) -> Option<String> {
    match token.find(DYNAMIC_HOLE) {
        Some(0) => None,
        Some(idx) => Some(format!("{}*", &token[..idx])),
        None => Some(token.to_string()),
    }
}

/// Resolve a string-shaped expression to class tokens, keeping the parts a
/// component composes dynamically at render: static tokens land in
/// [`ClassValue::static_classes`], dynamically-composed tokens become
/// `prefix-*` patterns in [`ClassValue::patterns`]. `None` when the
/// expression is not string-shaped at all.
pub(crate) fn extract_class_value(expr: &Expression<'_>) -> Option<ClassValue> {
    let value = class_template_value(expr)?;
    let mut result = ClassValue::default();
    let mut static_tokens: Vec<&str> = Vec::new();
    for token in value.split_whitespace() {
        if token.contains(DYNAMIC_HOLE) {
            if let Some(pattern) = scopable_class_token(token) {
                result.patterns.push(pattern);
            }
        } else {
            static_tokens.push(token);
        }
    }
    result.static_classes = static_tokens.join(" ");
    Some(result)
}

/// Resolve a string-shaped expression to its value where every dynamic part
/// is a [`DYNAMIC_HOLE`] marker. Handles string literals, template literals
/// (expressions become holes unless themselves string-shaped), `+`
/// concatenation (string-shaped as long as one side is), and TS type
/// wrappers. Returns `None` for expressions that are not string-shaped at
/// all.
pub(crate) fn class_template_value(expr: &Expression<'_>) -> Option<String> {
    match expr {
        Expression::StringLiteral(s) => Some(s.value.as_str().to_string()),
        Expression::TemplateLiteral(tpl) => {
            let mut value = String::new();
            for (i, quasi) in tpl.quasis.iter().enumerate() {
                value.push_str(quasi.value.raw.as_str());
                if i < tpl.expressions.len() {
                    match class_template_value(&tpl.expressions[i]) {
                        Some(inner) => value.push_str(&inner),
                        None => value.push(DYNAMIC_HOLE),
                    }
                }
            }
            Some(value)
        }
        Expression::BinaryExpression(bin) => {
            if bin.operator == BinaryOperator::Addition {
                let left = class_template_value(&bin.left);
                let right = class_template_value(&bin.right);
                if left.is_none() && right.is_none() {
                    return None;
                }
                let mut value = left.unwrap_or_else(|| DYNAMIC_HOLE.to_string());
                value.push_str(&right.unwrap_or_else(|| DYNAMIC_HOLE.to_string()));
                Some(value)
            } else {
                None
            }
        }
        Expression::TSAsExpression(as_expr) => class_template_value(&as_expr.expression),
        Expression::TSSatisfiesExpression(sat) => class_template_value(&sat.expression),
        Expression::ParenthesizedExpression(paren) => class_template_value(&paren.expression),
        _ => None,
    }
}

/// Extract and concatenate all string values from a nested object
/// expression. Dynamically-composed tokens inside the nested values are
/// appended to `patterns` as `prefix-*` entries. `None` when nothing at
/// all (neither static classes nor patterns) could be extracted.
pub(crate) fn extract_nested_object_classes(
    expr: &Expression<'_>,
    patterns: &mut Vec<String>,
) -> Option<String> {
    let Expression::ObjectExpression(obj) = expr else {
        return None;
    };

    let mut parts: Vec<String> = Vec::new();
    let patterns_before = patterns.len();

    for prop in &obj.properties {
        let ObjectPropertyKind::ObjectProperty(prop) = prop else {
            continue;
        };

        let value_expr = unwrap_type_expressions(&prop.value);
        if let Some(value) = extract_class_value(value_expr) {
            if !value.static_classes.is_empty() {
                parts.push(value.static_classes);
            }
            patterns.extend(value.patterns);
        }
    }

    if parts.is_empty() && patterns.len() == patterns_before {
        None
    } else {
        Some(parts.join(" "))
    }
}

/// Normalize runs of whitespace into single spaces.
pub(crate) fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
