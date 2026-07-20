//! Does the project stylesheet actually cover the classes a component uses?
//!
//! Previews adopt the compiled stylesheet whole rather than tree-shaking a
//! per-component subset, so nothing in the render path can fail when a class
//! has no rule — it simply renders unstyled. That silence is the hazard this
//! module exists to break: a component whose class records drifted from what
//! the sheet was compiled against produces a preview that looks plausible
//! and is wrong.
//!
//! The check runs against the COMPILED stylesheet, where a utility appears
//! as an escaped class selector (`.hover\:bg-primary-hover`), not against
//! the authored `@utility` form.

/// Classes the stylesheet has no rule for, in the order given.
///
/// An empty stylesheet is an error rather than "everything is uncovered":
/// it means the caller was handed nothing, which is a pipeline fault, not a
/// component fault.
pub fn uncovered_classes(classes: &[String], sheet: &str) -> Result<Vec<String>, String> {
    if sheet.trim().is_empty() {
        return Err("project stylesheet is empty".to_string());
    }

    Ok(classes
        .iter()
        .filter(|class| !class.trim().is_empty())
        .filter(|class| !sheet_covers(sheet, class))
        .cloned()
        .collect())
}

/// Whether the sheet carries a rule whose selector includes this class.
fn sheet_covers(sheet: &str, class: &str) -> bool {
    let needle = escape_class_selector(class);

    sheet.match_indices(&needle).any(|(start, _)| {
        // The match must END at a class-name boundary, or `.bg-primary`
        // would be "covered" by a `.bg-primary-hover` rule it has nothing
        // to do with.
        let tail = &sheet[start + needle.len()..];
        let ends_cleanly = tail
            .chars()
            .next()
            .is_none_or(|c| !(c.is_alphanumeric() || c == '-' || c == '_' || c == '\\'));

        // ...and the leading dot must be a real selector dot, not an escaped
        // one inside a class name (`.w-1\.5`), which is what would let `.5`
        // match inside it. Any other preceding character is fine: `.btn.p-4`
        // and `,.p-4` are both legitimate places for the class to appear.
        let head_ok = sheet[..start].chars().next_back().is_none_or(|c| c != '\\');

        ends_cleanly && head_ok
    })
}

/// Render a class name as it appears in a compiled selector: a leading dot
/// plus CSS-escaped special characters. Tailwind emits `hover:bg-x` as
/// `.hover\:bg-x`.
fn escape_class_selector(class: &str) -> String {
    let mut out = String::with_capacity(class.len() + 4);
    out.push('.');
    for ch in class.chars() {
        if matches!(
            ch,
            ':' | '.' | '/' | '%' | '[' | ']' | '(' | ')' | ',' | '#' | '!' | '@' | '$' | '&' | '*'
        ) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn empty_stylesheet_is_a_pipeline_fault_not_a_coverage_result() {
        let error = uncovered_classes(&owned(&["p-4"]), "   ").unwrap_err();
        assert!(error.contains("empty"), "got: {error}");
    }

    #[test]
    fn reports_only_classes_with_no_rule() {
        let sheet = ".p-4{padding:1rem}.flex{display:flex}";
        let uncovered = uncovered_classes(&owned(&["p-4", "gap-2", "flex"]), sheet).unwrap();
        assert_eq!(uncovered, owned(&["gap-2"]));
    }

    #[test]
    fn escaped_pseudo_variants_count_as_covered() {
        // The compiled form of `hover:bg-primary-hover`.
        let sheet = r#".hover\:bg-primary-hover:hover{background-color:var(--primary-hover)}"#;
        let uncovered = uncovered_classes(&owned(&["hover:bg-primary-hover"]), sheet).unwrap();
        assert!(uncovered.is_empty(), "got: {uncovered:?}");
    }

    #[test]
    fn a_longer_class_does_not_cover_its_prefix() {
        // The exact drift shape: the sheet has the -hover variant compiled
        // but never the bare utility. Tailwind only emits what it saw.
        let sheet = r#".hover\:bg-primary-hover:hover{background-color:var(--primary-hover)}"#;
        let uncovered = uncovered_classes(&owned(&["bg-primary-hover"]), sheet).unwrap();
        assert_eq!(uncovered, owned(&["bg-primary-hover"]));
    }

    #[test]
    fn a_prefixed_variant_does_not_cover_the_bare_class() {
        let sheet = r#".group-hover\:p-4{padding:1rem}"#;
        let uncovered = uncovered_classes(&owned(&["p-4"]), sheet).unwrap();
        assert_eq!(uncovered, owned(&["p-4"]));
    }

    #[test]
    fn matches_in_selector_lists_and_compound_selectors() {
        let sheet = ".flex,.grid{display:flex}.btn.p-4>span{color:red}";
        let uncovered = uncovered_classes(&owned(&["grid", "p-4", "btn"]), sheet).unwrap();
        assert!(uncovered.is_empty(), "got: {uncovered:?}");
    }
}
