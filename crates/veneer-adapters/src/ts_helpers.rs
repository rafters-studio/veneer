//! Shared helpers for parsing TypeScript AST expressions via oxc.

use oxc_ast::ast::{BinaryOperator, Expression, ObjectPropertyKind};

/// Marker for the dynamic part of a string composed at render time (for
/// example the `${tint}` hole in `` `text-quality-${tint}` ``). Internal to
/// extraction; never appears in returned class tokens.
pub(crate) const DYNAMIC_HOLE: char = '\u{FFFC}';

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

/// Extract and concatenate all string values from a nested object expression.
pub(crate) fn extract_nested_object_classes(expr: &Expression<'_>) -> Option<String> {
    let Expression::ObjectExpression(obj) = expr else {
        return None;
    };

    let mut parts: Vec<String> = Vec::new();

    for prop in &obj.properties {
        let ObjectPropertyKind::ObjectProperty(prop) = prop else {
            continue;
        };

        let value_expr = unwrap_type_expressions(&prop.value);
        if let Some(value) = extract_string_value(value_expr) {
            if !value.is_empty() {
                parts.push(value);
            }
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

/// Normalize runs of whitespace into single spaces.
pub(crate) fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
