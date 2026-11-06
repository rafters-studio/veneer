//! Variant B: parse and minify a stylesheet with lightningcss, and inline the
//! declarations of a class rule into the email output.

use lightningcss::rules::CssRule;
use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};
use lightningcss::traits::ToCss;
use serde_json::Value;
use std::collections::HashMap;

/// Parse, minify, print; and collect top-level style rules as selector -> declarations.
pub fn process(css: &str) -> Result<(String, HashMap<String, String>), String> {
    let opts = ParserOptions { error_recovery: true, ..ParserOptions::default() };
    let mut sheet = StyleSheet::parse(css, opts).map_err(|e| e.to_string())?;
    sheet.minify(MinifyOptions::default()).map_err(|e| e.to_string())?;

    let code = sheet.to_css(min()).map_err(|e| e.to_string())?.code;

    let mut rules = HashMap::new();
    for rule in &sheet.rules.0 {
        let CssRule::Style(style) = rule else { continue };
        let decls: Vec<String> = style
            .declarations
            .declarations
            .iter()
            .map(|p| p.to_css_string(false, min()))
            .chain(style.declarations.important_declarations.iter().map(|p| p.to_css_string(true, min())))
            .filter_map(Result::ok)
            .collect();
        for sel in &style.selectors.0 {
            if let Ok(s) = sel.to_css_string(min()) {
                rules.entry(s).or_insert_with(|| decls.join(";"));
            }
        }
    }
    Ok((code, rules))
}

pub fn compile_css(rafter: &str, css: &str) -> Result<Value, String> {
    let (code, rules) = process(css)?;
    let mut out = crate::compile_with(rafter, &|sel| rules.get(sel).cloned())?;
    if let Value::Object(map) = &mut out {
        map.insert("css".into(), Value::String(code));
    }
    Ok(out)
}

/// getrandom custom backend: ahash only wants hash seeds, and a compile core
/// should be deterministic anyway, so the seed is a fixed pattern.
///
/// # Safety
/// `dest..dest+len` must be writable.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
unsafe extern "Rust" fn __getrandom_v03_custom(dest: *mut u8, len: usize) -> Result<(), getrandom::Error> {
    for i in 0..len {
        #[allow(clippy::cast_possible_truncation)]
        dest.add(i).write((i as u8).wrapping_mul(167).wrapping_add(13));
    }
    Ok(())
}

/// # Safety
/// Both ranges must be live buffers from `alloc`.
#[no_mangle]
pub unsafe extern "C" fn compile_rafter_css(rp: *const u8, rl: usize, cp: *const u8, cl: usize) -> *mut u8 {
    crate::emit(compile_css(crate::input(rp, rl), crate::input(cp, cl)))
}

fn min<'a>() -> PrinterOptions<'a> {
    PrinterOptions { minify: true, ..PrinterOptions::default() }
}
