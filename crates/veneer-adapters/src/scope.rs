//! CSS scoping for Web Components using Shadow DOM.
//!
//! Extracts the minimal set of CSS rules from a Tailwind v4 stylesheet so that
//! a Web Component can load only what it needs via a constructable stylesheet.
//!
//! The entry points are:
//! - [`scope_css`] — takes a list of class names and a full CSS file; returns
//!   only the matching `@utility` blocks plus any `@theme` blocks whose custom
//!   properties are referenced by the matched utilities.
//! - [`shadow_css_for_component`] — same matching, but returns browser-ready
//!   CSS (`:host` variables plus plain class rules) suitable for
//!   `CSSStyleSheet.replaceSync`, and returns a [`ScopeError`] naming the
//!   component when extraction produces nothing.
//! - [`extract_classes_from_ts`] — parses a `.classes.ts` source file and
//!   returns every individual Tailwind class token found in string literals,
//!   plus `prefix-*` patterns for classes composed dynamically at render
//!   (template literals or concatenations with runtime values). Tailwind
//!   tree-shakes dynamically-composed names from compiled output, so the
//!   patterns let [`scope_css`]/[`shadow_css_for_component`] pull in every
//!   utility the component can resolve to at render, not just source
//!   literals.

use std::collections::HashSet;

use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;
use regex::Regex;

use crate::ts_helpers::{class_template_value, scopable_class_token, unwrap_type_expressions};

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

    let scoped = match_rules(classes, full_css);

    // Warn about classes that have no matching @utility block.
    for name in &scoped.unmatched {
        eprintln!("veneer/scope_css: no @utility block found for '{name}' (may be a @theme token)");
    }

    if scoped.utilities.is_empty() {
        return String::new();
    }

    assemble_css(&scoped, "@theme", |name| format!("@utility {name}"))
}

/// Failure to extract scoped CSS for a component. Always names the component
/// so a preview never goes missing its styles silently (FR-VEN-018).
#[derive(Debug, thiserror::Error)]
pub enum ScopeError {
    #[error(
        "cannot extract scoped CSS for component '{component}': \
         the project stylesheet is empty (classes requested: {classes})"
    )]
    EmptyStylesheet { component: String, classes: String },

    #[error(
        "cannot extract scoped CSS for component '{component}': \
         no CSS rules matched any of its classes ({missing}); \
         refusing to emit a preview silently missing its styles"
    )]
    NoRulesMatched { component: String, missing: String },
}

/// Browser-ready scoped CSS for one component's shadow root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowCss {
    /// Plain CSS (`:host` custom properties plus `.class` rules) valid for
    /// `CSSStyleSheet.replaceSync` — no Tailwind at-rules remain.
    pub css: String,
    /// Requested classes with no matching `@utility` block (for example
    /// Tailwind built-ins that live outside the `@utility` layer). Sorted.
    pub unmatched: Vec<String>,
}

/// Extract browser-ready CSS for one component, scoped to the classes it
/// resolves to at render (including `prefix-*` patterns from
/// [`extract_classes_from_ts`]).
///
/// The output translates the Tailwind source at-rules to shadow-root CSS:
/// `@theme` declarations become `:host` custom properties and each matched
/// `@utility name` block becomes a `.name` rule, so the result adopts
/// cleanly via `shadowRoot.adoptedStyleSheets`.
///
/// Errors when the component declares classes but nothing can be extracted:
/// the error names the component and the classes that found no selector
/// source. An empty class list returns empty CSS — a component that declares
/// no classes has no styles to scope.
pub fn shadow_css_for_component(
    component: &str,
    classes: &[String],
    full_css: &str,
) -> Result<ShadowCss, ScopeError> {
    if classes.is_empty() {
        return Ok(ShadowCss {
            css: String::new(),
            unmatched: Vec::new(),
        });
    }

    if full_css.trim().is_empty() {
        return Err(ScopeError::EmptyStylesheet {
            component: component.to_string(),
            classes: classes.join(" "),
        });
    }

    let scoped = match_rules(classes, full_css);

    if scoped.utilities.is_empty() {
        return Err(ScopeError::NoRulesMatched {
            component: component.to_string(),
            missing: scoped.unmatched.join(" "),
        });
    }

    Ok(ShadowCss {
        css: assemble_css(&scoped, ":host", |name| format!(".{name}")),
        unmatched: scoped.unmatched,
    })
}

/// Assemble scoped rules into CSS text: one block of theme declarations
/// (when any) followed by one block per matched utility. The two output
/// formats differ only in their selectors: Tailwind source form
/// (`@theme` / `@utility name`) and browser shadow form (`:host` / `.name`).
fn assemble_css(
    scoped: &ScopedRules,
    theme_selector: &str,
    utility_selector: impl Fn(&str) -> String,
) -> String {
    let mut out = String::new();

    if !scoped.theme_decls.is_empty() {
        out.push_str(theme_selector);
        out.push_str(" {\n");
        for declaration in &scoped.theme_decls {
            out.push_str("  ");
            out.push_str(declaration);
            out.push('\n');
        }
        out.push_str("}\n\n");
    }

    for block in &scoped.utilities {
        out.push_str(&utility_selector(&block.name));
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

/// The rules a class list selects out of a full stylesheet: matched
/// `@utility` blocks, the `@theme` declarations they reference, and the
/// requested names that matched nothing.
struct ScopedRules {
    utilities: Vec<UtilityBlock>,
    theme_decls: Vec<String>,
    unmatched: Vec<String>,
}

/// Match the stylesheet's `@utility` blocks against a class list.
///
/// Class tokens are matched by base name (modifier prefixes such as
/// `hover:` stripped). A token ending in `*` is a prefix pattern produced
/// by [`extract_classes_from_ts`] for dynamically-composed class names: it
/// matches every utility whose name starts with the prefix, so the scoped
/// CSS covers whatever the component resolves to at render.
fn match_rules(classes: &[String], full_css: &str) -> ScopedRules {
    let base_names: HashSet<String> = classes
        .iter()
        .flat_map(|c| c.split_whitespace())
        .map(strip_modifiers)
        .collect();

    let (patterns, exact): (Vec<&String>, Vec<&String>) =
        base_names.iter().partition(|n| n.ends_with('*'));
    let exact: HashSet<&str> = exact.into_iter().map(String::as_str).collect();
    let prefixes: Vec<&str> = patterns.iter().map(|p| p.trim_end_matches('*')).collect();

    let mut matched_prefixes: HashSet<&str> = HashSet::new();
    let utilities: Vec<UtilityBlock> = extract_utility_blocks(full_css)
        .into_iter()
        .filter(|u| {
            // Check prefixes even on an exact hit so a pattern that also
            // covers this utility is still recorded as matched.
            let mut hit = exact.contains(u.name.as_str());
            for prefix in &prefixes {
                if u.name.starts_with(prefix) {
                    matched_prefixes.insert(prefix);
                    hit = true;
                }
            }
            hit
        })
        .collect();

    let matched_names: HashSet<&str> = utilities.iter().map(|u| u.name.as_str()).collect();
    let mut unmatched: Vec<String> = base_names
        .iter()
        .filter(|name| {
            if let Some(prefix) = name.strip_suffix('*') {
                !matched_prefixes.contains(prefix)
            } else {
                !matched_names.contains(name.as_str())
            }
        })
        .cloned()
        .collect();
    unmatched.sort();

    let referenced_vars: HashSet<String> = utilities
        .iter()
        .flat_map(|u| collect_var_references(&u.body))
        .collect();
    let theme_decls = extract_relevant_theme_decls(full_css, &referenced_vars);

    ScopedRules {
        utilities,
        theme_decls,
        unmatched,
    }
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

    // --- Dynamically-composed classes (Tailwind tree-shake caveat) ---
    //
    // Tailwind tree-shakes class names that never appear as source literals
    // (`color={tint}` resolving to `text-quality-*`). Extraction must
    // surface a `prefix-*` pattern for those so scoping can include every
    // utility the component resolves to at render.

    #[test]
    fn extract_classes_emits_prefix_pattern_for_template_expression() {
        let source = r#"
export const qualityClasses = {
  badge: `text-quality-${tint} font-bold`,
};
"#;
        let classes = extract_classes_from_ts(source);
        assert!(classes.contains(&"text-quality-*".to_string()));
        assert!(classes.contains(&"font-bold".to_string()));
    }

    #[test]
    fn extract_classes_emits_prefix_pattern_for_concatenation() {
        let source = "export const tintClass = 'text-quality-' + tint;";
        let classes = extract_classes_from_ts(source);
        assert_eq!(classes, vec!["text-quality-*".to_string()]);
    }

    #[test]
    fn extract_classes_emits_prefix_pattern_from_class_builder_arrow() {
        let source = "export const tintClass = (tint: string) => `bg-quality-${tint}`;";
        let classes = extract_classes_from_ts(source);
        assert_eq!(classes, vec!["bg-quality-*".to_string()]);
    }

    #[test]
    fn extract_classes_resolves_conditional_arms() {
        let source =
            "export const stateClass = (active: boolean) => active ? 'bg-primary' : 'bg-muted';";
        let classes = extract_classes_from_ts(source);
        assert!(classes.contains(&"bg-primary".to_string()));
        assert!(classes.contains(&"bg-muted".to_string()));
    }

    #[test]
    fn extract_classes_skips_token_with_no_static_prefix() {
        let source = "export const cls = `${dynamic} flex`;";
        let classes = extract_classes_from_ts(source);
        assert_eq!(classes, vec!["flex".to_string()]);
    }

    const QUALITY_CSS: &str = r#"
@theme {
  --color-quality-500: oklch(0.7 0.14 140);
  --color-quality-600: oklch(0.55 0.14 140);
  --color-border: oklch(0.92 0 0);
}

@utility text-quality-500 {
  color: var(--color-quality-500);
}

@utility text-quality-600 {
  color: var(--color-quality-600);
}

@utility border-border {
  border-color: var(--color-border);
}
"#;

    #[test]
    fn scope_css_expands_wildcard_pattern_to_all_matching_utilities() {
        let classes = vec!["text-quality-*".to_string()];
        let result = scope_css(&classes, QUALITY_CSS);
        assert!(result.contains("@utility text-quality-500"));
        assert!(result.contains("@utility text-quality-600"));
        assert!(!result.contains("border-border"));
    }

    #[test]
    fn wildcard_pattern_overlapping_an_exact_match_is_not_reported_unmatched() {
        // "text-quality-500" matches exactly; "text-quality-*" covers the
        // same utility. The pattern matched and must not surface as
        // unmatched noise.
        let classes = vec!["text-quality-500".to_string(), "text-quality-*".to_string()];
        let css = "@utility text-quality-500 {\n  color: red;\n}\n";
        let shadow = shadow_css_for_component("QualityBadge", &classes, css)
            .expect("both the exact class and the pattern match");
        assert!(shadow.css.contains(".text-quality-500 {"));
        assert!(
            shadow.unmatched.is_empty(),
            "unmatched: {:?}",
            shadow.unmatched
        );
    }

    // --- shadow_css_for_component ---

    #[test]
    fn shadow_css_converts_utilities_to_class_rules() {
        let classes = vec!["bg-primary".to_string()];
        let shadow = shadow_css_for_component("Button", &classes, SAMPLE_CSS)
            .expect("bg-primary matches a utility");
        assert!(shadow.css.contains(".bg-primary {"));
        assert!(shadow
            .css
            .contains("background-color: var(--color-primary);"));
        assert!(!shadow.css.contains("@utility"));
        assert!(!shadow.css.contains("@theme"));
    }

    #[test]
    fn shadow_css_places_theme_vars_on_host() {
        let classes = vec!["bg-primary".to_string()];
        let shadow = shadow_css_for_component("Button", &classes, SAMPLE_CSS)
            .expect("bg-primary matches a utility");
        assert!(shadow.css.contains(":host {"));
        assert!(shadow
            .css
            .contains("--color-primary: oklch(0.645 0.12 180);"));
        // Unrelated theme vars stay out.
        assert!(!shadow.css.contains("--color-muted-foreground:"));
    }

    #[test]
    fn shadow_css_includes_dynamically_resolved_classes() {
        // End to end for the tree-shake caveat: a dynamically-composed
        // class never appears as a literal, yet the scoped CSS carries
        // every utility it can resolve to at render.
        let source = "export const tintClass = (tint: string) => `text-quality-${tint}`;";
        let classes = extract_classes_from_ts(source);
        let shadow = shadow_css_for_component("QualityBadge", &classes, QUALITY_CSS)
            .expect("pattern matches quality utilities");
        assert!(shadow.css.contains(".text-quality-500 {"));
        assert!(shadow.css.contains(".text-quality-600 {"));
        assert!(shadow
            .css
            .contains("--color-quality-600: oklch(0.55 0.14 140);"));
    }

    #[test]
    fn shadow_css_empty_classes_is_ok_and_empty() {
        let shadow = shadow_css_for_component("Plain", &[], SAMPLE_CSS)
            .expect("no classes means no styles to scope");
        assert!(shadow.css.is_empty());
        assert!(shadow.unmatched.is_empty());
    }

    #[test]
    fn shadow_css_errors_on_empty_stylesheet_naming_component() {
        let classes = vec!["bg-primary".to_string()];
        let error = shadow_css_for_component("Button", &classes, "")
            .expect_err("empty stylesheet cannot style a classed component");
        let message = error.to_string();
        assert!(message.contains("Button"));
        assert!(message.contains("bg-primary"));
        assert!(matches!(error, ScopeError::EmptyStylesheet { .. }));
    }

    #[test]
    fn shadow_css_errors_when_nothing_matches_naming_component_and_classes() {
        let classes = vec!["no-such-class".to_string()];
        let error =
            shadow_css_for_component("Badge", &classes, SAMPLE_CSS).expect_err("nothing matches");
        let message = error.to_string();
        assert!(message.contains("Badge"));
        assert!(message.contains("no-such-class"));
        assert!(matches!(error, ScopeError::NoRulesMatched { .. }));
    }

    #[test]
    fn shadow_css_reports_partially_unmatched_classes() {
        let classes = vec!["bg-primary".to_string(), "bg-mystery".to_string()];
        let shadow = shadow_css_for_component("Button", &classes, SAMPLE_CSS)
            .expect("bg-primary still matches");
        assert!(shadow.css.contains(".bg-primary {"));
        assert_eq!(shadow.unmatched, vec!["bg-mystery".to_string()]);
    }
}
