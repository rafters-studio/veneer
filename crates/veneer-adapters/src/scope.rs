//! CSS scoping for Web Components using Shadow DOM.
//!
//! Extracts the minimal set of CSS rules from a Tailwind v4 stylesheet so that
//! a Web Component can load only what it needs via a constructable stylesheet.
//!
//! The two entry points are:
//! - [`scope_css`] — takes a list of class names and a full CSS file; returns
//!   only the matching `@utility` blocks plus any `@theme` blocks whose custom
//!   properties are referenced by the matched utilities.
//! - [`extract_classes_from_ts`] — parses a `.classes.ts` source file and
//!   returns every individual Tailwind class token found in string literals.

use std::collections::HashSet;

use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;
use regex::Regex;

use crate::ts_helpers::{extract_string_value, unwrap_type_expressions};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Extract only the CSS rules matching the given class names from a full stylesheet.
///
/// Parses the CSS for `@utility` blocks and returns only those whose names
/// appear in the class list (after stripping modifier prefixes such as
/// `hover:`, `focus:`, `@sm:`, `@lg:`, etc.).  Also includes any `@theme`
/// blocks that define custom properties (`var(--...)`) referenced by the
/// matched utilities.
///
/// Warns (via `eprintln!`) for any class names with no matching `@utility`
/// block — these may be Tailwind built-ins or `@theme` tokens.
///
/// Returns an empty string if no classes match.  Never panics on malformed
/// CSS — unclosed blocks are silently skipped.
pub fn scope_css(classes: &[String], full_css: &str) -> String {
    if classes.is_empty() || full_css.is_empty() {
        return String::new();
    }

    // Build a set of base utility names (strip variant/modifier prefixes).
    let base_names: HashSet<String> = classes
        .iter()
        .flat_map(|c| c.split_whitespace())
        .map(strip_modifiers)
        .collect();

    // Extract all @utility blocks from the stylesheet.
    let utilities = extract_utility_blocks(full_css);

    // Keep only utilities whose name appears in the class list.
    let matched: Vec<&UtilityBlock> = utilities
        .iter()
        .filter(|u| base_names.contains(&u.name))
        .collect();

    // Warn about classes that have no matching @utility block.
    let matched_names: HashSet<&str> = matched.iter().map(|u| u.name.as_str()).collect();
    for name in &base_names {
        if !matched_names.contains(name.as_str()) {
            eprintln!(
                "veneer/scope_css: no @utility block found for '{name}' (may be a @theme token)"
            );
        }
    }

    if matched.is_empty() {
        return String::new();
    }

    // Collect all CSS custom properties referenced by the matched utilities.
    let referenced_vars: HashSet<String> = matched
        .iter()
        .flat_map(|u| collect_var_references(&u.body))
        .collect();

    // Collect @theme declarations that define any of the referenced variables.
    let theme_decls = extract_relevant_theme_decls(full_css, &referenced_vars);

    // Assemble the output.
    let mut out = String::new();

    if !theme_decls.is_empty() {
        out.push_str("@theme {\n");
        for declaration in &theme_decls {
            out.push_str("  ");
            out.push_str(declaration);
            out.push('\n');
        }
        out.push_str("}\n\n");
    }

    for block in &matched {
        out.push_str("@utility ");
        out.push_str(&block.name);
        out.push_str(" {\n");
        out.push_str(&block.body);
        out.push_str("}\n\n");
    }

    out.trim_end().to_string()
}

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

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

struct UtilityBlock {
    name: String,
    body: String,
}

// ---------------------------------------------------------------------------
// CSS parsing helpers
// ---------------------------------------------------------------------------

/// Strip all variant/modifier prefixes from a class token, returning the
/// base utility name.
///
/// Examples:
/// - `hover:bg-primary`        → `bg-primary`
/// - `@sm:text-lg`             → `text-lg`
/// - `focus-visible:ring-2`    → `ring-2`
/// - `dark:text-white`         → `text-white`
/// - `@sm:hover:flex`          → `flex`
fn strip_modifiers(class: &str) -> String {
    match class.rfind(':') {
        Some(idx) => class[idx + 1..].to_string(),
        None => class.to_string(),
    }
}

/// Find the position of the closing brace that matches the opening brace
/// at `bytes[start - 1]` (i.e. `start` is the first byte after the `{`).
///
/// Returns `Some(absolute_index_of_closing_brace)` or `None` if the block
/// is unclosed.
fn find_matching_brace(bytes: &[u8], start: usize) -> Option<usize> {
    let mut depth: usize = 1;
    for (i, &b) in bytes[start..].iter().enumerate() {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(start + i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Extract all `@utility name { ... }` blocks from a CSS string.
///
/// Uses brace-depth tracking so nested braces are handled correctly.
/// Unclosed blocks are silently skipped.
fn extract_utility_blocks(css: &str) -> Vec<UtilityBlock> {
    let header_re = match Regex::new(r"@utility\s+([\w-]+)\s*\{") {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let bytes = css.as_bytes();
    let mut blocks = Vec::new();

    for cap in header_re.captures_iter(css) {
        let name = cap[1].to_string();
        let body_start = cap.get(0).map(|m| m.end()).unwrap_or(0);

        if let Some(body_end) = find_matching_brace(bytes, body_start) {
            let body = css[body_start..body_end].to_string();
            blocks.push(UtilityBlock { name, body });
        }
    }

    blocks
}

/// Collect all `var(--xxx)` references from a CSS body string.
fn collect_var_references(css_body: &str) -> Vec<String> {
    let var_re = match Regex::new(r"var\(\s*(--[\w-]+)") {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    var_re
        .captures_iter(css_body)
        .map(|cap| cap[1].to_string())
        .collect()
}

/// Extract `@theme` custom property declarations that define any of the
/// requested variable names.
///
/// Returns individual declaration strings like `--color-primary: oklch(...)`.
fn extract_relevant_theme_decls(css: &str, vars: &HashSet<String>) -> Vec<String> {
    if vars.is_empty() {
        return Vec::new();
    }

    let theme_re = match Regex::new(r"@theme\s*\{") {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let decl_re = match Regex::new(r"(--[\w-]+)\s*:[^;]+;") {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let bytes = css.as_bytes();
    let mut result = Vec::new();

    for cap in theme_re.captures_iter(css) {
        let block_start = cap.get(0).map(|m| m.end()).unwrap_or(0);

        if let Some(block_end) = find_matching_brace(bytes, block_start) {
            let block_body = &css[block_start..block_end];
            for decl_cap in decl_re.captures_iter(block_body) {
                if vars.contains(&decl_cap[1]) {
                    result.push(decl_cap[0].trim().to_string());
                }
            }
        }
    }

    result.sort();
    result.dedup();
    result
}

// ---------------------------------------------------------------------------
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

fn collect_classes_from_expr(
    expr: &Expression<'_>,
    seen: &mut HashSet<String>,
    out: &mut Vec<String>,
) {
    // Flat string literal — split and add tokens.
    if let Some(value) = extract_string_value(expr) {
        add_classes(&value, seen, out);
        return;
    }

    // Object expression — recurse into each property value.
    if let Expression::ObjectExpression(obj) = expr {
        for prop in &obj.properties {
            if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(p) = prop {
                collect_classes_from_expr(unwrap_type_expressions(&p.value), seen, out);
            }
        }
        return;
    }

    // Array expression — recurse into elements.
    if let Expression::ArrayExpression(arr) = expr {
        for elem in &arr.elements {
            let expr_ref = match elem {
                oxc_ast::ast::ArrayExpressionElement::SpreadElement(_) => continue,
                oxc_ast::ast::ArrayExpressionElement::Elision(_) => continue,
                _ => elem.to_expression(),
            };
            collect_classes_from_expr(unwrap_type_expressions(expr_ref), seen, out);
        }
    }
}

/// Split a class string and add individual tokens to the output, deduplicating.
fn add_classes(class_string: &str, seen: &mut HashSet<String>, out: &mut Vec<String>) {
    for token in class_string.split_whitespace() {
        let token = token.to_string();
        if seen.insert(token.clone()) {
            out.push(token);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CSS: &str = r#"
@theme {
  --color-primary: oklch(0.645 0.12 180);
  --color-primary-foreground: oklch(0.985 0 0);
  --color-muted: oklch(0.967 0 0);
  --color-muted-foreground: oklch(0.552 0 0);
  --color-border: oklch(0.92 0 0);
}

@utility bg-primary {
  background-color: var(--color-primary);
}

@utility text-primary-foreground {
  color: var(--color-primary-foreground);
}

@utility text-muted-foreground {
  color: var(--color-muted-foreground);
}

@utility border-border {
  border-color: var(--color-border);
}

@utility rounded-lg {
  border-radius: 0.5rem;
}

@utility flex {
  display: flex;
}

@utility items-center {
  align-items: center;
}
"#;

    #[test]
    fn scope_css_returns_empty_for_no_classes() {
        let result = scope_css(&[], SAMPLE_CSS);
        assert_eq!(result, "");
    }

    #[test]
    fn scope_css_returns_empty_for_empty_css() {
        let result = scope_css(&["bg-primary".to_string()], "");
        assert_eq!(result, "");
    }

    #[test]
    fn scope_css_extracts_matched_utility() {
        let classes = vec!["flex".to_string(), "items-center".to_string()];
        let result = scope_css(&classes, SAMPLE_CSS);
        assert!(result.contains("@utility flex"));
        assert!(result.contains("display: flex"));
        assert!(result.contains("@utility items-center"));
        assert!(result.contains("align-items: center"));
    }

    #[test]
    fn scope_css_excludes_unmatched_utilities() {
        let classes = vec!["flex".to_string()];
        let result = scope_css(&classes, SAMPLE_CSS);
        assert!(!result.contains("@utility bg-primary"));
        assert!(!result.contains("@utility text-muted-foreground"));
    }

    #[test]
    fn scope_css_includes_theme_vars_for_matched_utilities() {
        let classes = vec!["bg-primary".to_string()];
        let result = scope_css(&classes, SAMPLE_CSS);
        assert!(result.contains("@theme"));
        assert!(result.contains("--color-primary:"));
        // Should NOT include unrelated theme vars.
        assert!(!result.contains("--color-muted-foreground:"));
    }

    #[test]
    fn scope_css_strips_hover_modifier() {
        let classes = vec!["hover:bg-primary".to_string()];
        let result = scope_css(&classes, SAMPLE_CSS);
        assert!(result.contains("@utility bg-primary"));
    }

    #[test]
    fn scope_css_strips_container_query_modifier() {
        let classes = vec!["@sm:flex".to_string()];
        let result = scope_css(&classes, SAMPLE_CSS);
        assert!(result.contains("@utility flex"));
    }

    #[test]
    fn scope_css_handles_compound_modifier() {
        let classes = vec!["focus-visible:flex".to_string()];
        let result = scope_css(&classes, SAMPLE_CSS);
        assert!(result.contains("@utility flex"));
    }

    #[test]
    fn scope_css_strips_chained_modifiers() {
        let classes = vec!["@sm:hover:flex".to_string()];
        let result = scope_css(&classes, SAMPLE_CSS);
        assert!(result.contains("@utility flex"));
    }

    #[test]
    fn scope_css_does_not_panic_on_malformed_css() {
        let malformed = "@utility broken { color: red; /* unclosed";
        let result = scope_css(&["broken".to_string()], malformed);
        // Unclosed block is skipped — result is empty (no match found).
        assert_eq!(result, "");
    }

    const SAMPLE_TS: &str = r#"
export const kbdBaseClasses = 'inline-flex items-center rounded border bg-muted px-1.5 py-0.5';

export const typographyClasses = {
  h1: 'scroll-m-20 text-4xl font-bold tracking-tight',
  h2: 'scroll-m-20 text-3xl font-semibold tracking-tight',
  muted: 'text-sm text-muted-foreground',
} as const;
"#;

    #[test]
    fn extract_classes_finds_flat_string() {
        let classes = extract_classes_from_ts(SAMPLE_TS);
        assert!(classes.contains(&"inline-flex".to_string()));
        assert!(classes.contains(&"items-center".to_string()));
        assert!(classes.contains(&"rounded".to_string()));
        assert!(classes.contains(&"bg-muted".to_string()));
    }

    #[test]
    fn extract_classes_finds_object_values() {
        let classes = extract_classes_from_ts(SAMPLE_TS);
        assert!(classes.contains(&"text-4xl".to_string()));
        assert!(classes.contains(&"font-bold".to_string()));
        assert!(classes.contains(&"text-muted-foreground".to_string()));
    }

    #[test]
    fn extract_classes_deduplicates() {
        let classes = extract_classes_from_ts(SAMPLE_TS);
        let count = classes
            .iter()
            .filter(|c| c.as_str() == "scroll-m-20")
            .count();
        assert_eq!(
            count, 1,
            "scroll-m-20 appears twice in source but must be deduped"
        );
    }

    #[test]
    fn extract_classes_returns_empty_for_empty_source() {
        let classes = extract_classes_from_ts("");
        assert!(classes.is_empty());
    }
}
