//! Shared helpers for parsing TypeScript AST expressions via oxc.

use oxc_ast::ast::{BinaryOperator, Expression, ObjectPropertyKind};

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

/// Extract a string value from an expression, handling string literals,
/// template literals, binary concatenation, and TS type wrappers.
pub(crate) fn extract_string_value(expr: &Expression<'_>) -> Option<String> {
    match expr {
        Expression::StringLiteral(s) => Some(s.value.as_str().to_string()),
        Expression::TemplateLiteral(tpl) => {
            if tpl.expressions.is_empty() && !tpl.quasis.is_empty() {
                let value = tpl
                    .quasis
                    .iter()
                    .map(|q| q.value.raw.as_str())
                    .collect::<Vec<_>>()
                    .join("");
                Some(value)
            } else {
                None
            }
        }
        Expression::BinaryExpression(bin) => {
            if bin.operator == BinaryOperator::Addition {
                let left = extract_string_value(&bin.left)?;
                let right = extract_string_value(&bin.right)?;
                Some(format!("{left}{right}"))
            } else {
                None
            }
        }
        Expression::TSAsExpression(as_expr) => extract_string_value(&as_expr.expression),
        Expression::TSSatisfiesExpression(sat) => extract_string_value(&sat.expression),
        Expression::ParenthesizedExpression(paren) => extract_string_value(&paren.expression),
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
