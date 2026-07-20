//! Class extraction from `.classes.ts` sources.
//!
//! Parses a TypeScript class-record file and returns every Tailwind class
//! token the component can resolve to at render: string literals, object and
//! array members, both arms of a conditional, and the bodies of class-builder
//! arrows. Classes composed dynamically (template literals with runtime
//! values) surface as `prefix-*` patterns, because the concrete name never
//! appears in source.
//!
//! This is the surviving half of the former `scope` module. The other half
//! subset the stylesheet per component; previews now adopt the compiled sheet
//! whole, so the subsetting is gone and only the reading remains.


use std::collections::HashSet;

use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;

use crate::ts_helpers::{class_template_value, scopable_class_token, unwrap_type_expressions};


/// Extract class names from a `.classes.ts` file.
///
/// Parses the TypeScript source to find all string literals used as Tailwind
/// class names in variant/size/state maps and string constants.  Returns
/// individual space-separated class tokens deduplicated and sorted.
pub fn extract_classes_from_ts(source: &str) -> Vec<String> {
    let allocator = Allocator::default();
    let source_type = SourceType::ts();
    let ret = Parser::new(&allocator, source, source_type).parse();

    if ret.panicked {
        eprintln!("veneer/extract_classes_from_ts: parser panicked");
        return Vec::new();
    }

    let mut seen: HashSet<String> = HashSet::new();
    let mut classes: Vec<String> = Vec::new();

    for stmt in &ret.program.body {
        collect_classes_from_statement(stmt, &mut seen, &mut classes);
    }

    classes.sort();
    classes
}

// TypeScript AST walking helpers
// ---------------------------------------------------------------------------

fn collect_classes_from_statement(
    stmt: &Statement<'_>,
    seen: &mut HashSet<String>,
    out: &mut Vec<String>,
) {
    match stmt {
        Statement::ExportNamedDeclaration(export) => {
            if let Some(oxc_ast::ast::Declaration::VariableDeclaration(var_decl)) =
                &export.declaration
            {
                for declarator in &var_decl.declarations {
                    if let Some(ref init) = declarator.init {
                        collect_classes_from_expr(unwrap_type_expressions(init), seen, out);
                    }
                }
            }
        }
        Statement::VariableDeclaration(var_decl) => {
            for declarator in &var_decl.declarations {
                if let Some(ref init) = declarator.init {
                    collect_classes_from_expr(unwrap_type_expressions(init), seen, out);
                }
            }
        }
        _ => {}
    }
}

/// Collect every class token an expression can resolve to at render:
/// string-shaped values (with dynamically-composed tokens as `prefix-*`
/// patterns), object/array members, conditional arms, and class-builder
/// arrow bodies. Shared with registry export discovery so the real
/// extraction pipeline surfaces the same tokens the tests prove.
pub(crate) fn collect_classes_from_expr(
    expr: &Expression<'_>,
    seen: &mut HashSet<String>,
    out: &mut Vec<String>,
) {
    // String-shaped value (literal, template, concatenation) — split into
    // tokens; dynamically-composed parts become `prefix-*` patterns.
    if let Some(value) = class_template_value(expr) {
        add_classes(&value, seen, out);
        return;
    }

    match expr {
        // Object expression — recurse into each property value.
        Expression::ObjectExpression(obj) => {
            for prop in &obj.properties {
                if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(p) = prop {
                    collect_classes_from_expr(unwrap_type_expressions(&p.value), seen, out);
                }
            }
        }
        // Array expression — recurse into elements.
        Expression::ArrayExpression(arr) => {
            for elem in &arr.elements {
                let expr_ref = match elem {
                    oxc_ast::ast::ArrayExpressionElement::SpreadElement(_) => continue,
                    oxc_ast::ast::ArrayExpressionElement::Elision(_) => continue,
                    _ => elem.to_expression(),
                };
                collect_classes_from_expr(unwrap_type_expressions(expr_ref), seen, out);
            }
        }
        // Conditional — the component resolves to either arm at render.
        Expression::ConditionalExpression(cond) => {
            collect_classes_from_expr(unwrap_type_expressions(&cond.consequent), seen, out);
            collect_classes_from_expr(unwrap_type_expressions(&cond.alternate), seen, out);
        }
        // Class-builder arrow (for example `(tint) => \`text-quality-${tint}\``)
        // — the returned expressions are what the component resolves to.
        Expression::ArrowFunctionExpression(arrow) => {
            for stmt in &arrow.body.statements {
                match stmt {
                    Statement::ExpressionStatement(expr_stmt) => {
                        collect_classes_from_expr(
                            unwrap_type_expressions(&expr_stmt.expression),
                            seen,
                            out,
                        );
                    }
                    Statement::ReturnStatement(ret) => {
                        if let Some(ref argument) = ret.argument {
                            collect_classes_from_expr(unwrap_type_expressions(argument), seen, out);
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

/// Split a class string and add individual tokens to the output,
/// deduplicating. A token containing a dynamic hole is a
/// dynamically-composed class: [`scopable_class_token`] turns it into a
/// `prefix-*` pattern from the static text before the hole (a token with
/// no static prefix cannot be scoped and is skipped).
fn add_classes(class_string: &str, seen: &mut HashSet<String>, out: &mut Vec<String>) {
    for raw in class_string.split_whitespace() {
        let Some(token) = scopable_class_token(raw) else {
            continue;
        };
        if seen.insert(token.clone()) {
            out.push(token);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

