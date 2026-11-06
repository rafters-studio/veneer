//! T19: a `.rafter` compile core small enough to measure as wasm.
//!
//! Input: a `.rafter` JSON string (the rafters composite shape). Output: a JSON
//! object with `html`, `mdast` and `email`. No file system, no network.
//! Plausible, not complete: it follows `rule-attrs.ts`, `resolve-block.ts`,
//! `walk-blocks.ts` and `to-mdx.ts` for the shapes, and nothing more.

use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;

#[cfg(feature = "css")]
mod css;

const MAX_DEPTH: usize = 50;

type Attrs = BTreeMap<&'static str, Value>;

/// rule-attrs.ts: the fixed attribute sets of the built-in string rules.
fn string_rule_attrs(name: &str) -> &'static [(&'static str, Attr)] {
    match name {
        "required" => &[("required", Attr::True)],
        "email" => &[("type", Attr::Str("email"))],
        "url" => &[("type", Attr::Str("url"))],
        "password" => &[("type", Attr::Str("password")), ("minlength", Attr::Num(8))],
        _ => &[],
    }
}

#[derive(Clone, Copy)]
enum Attr {
    True,
    Str(&'static str),
    Num(i64),
}

fn numeric_config(config: &Map<String, Value>, key: &str) -> Option<Value> {
    match config.get(key)? {
        Value::Number(n) => Some(Value::Number(n.clone())),
        Value::String(s) if !s.trim().is_empty() => s.trim().parse::<i64>().map(|i| json!(i)).ok().or_else(|| {
            s.trim().parse::<f64>().ok().filter(|f| f.is_finite()).map(|f| json!(f))
        }),
        _ => None,
    }
}

/// rule-attrs.ts `rulesToHtmlAttrs`: later rules win.
pub fn rules_to_html_attrs(rules: Option<&Value>) -> Attrs {
    let mut attrs = Attrs::new();
    let Some(Value::Array(rules)) = rules else {
        return attrs;
    };
    let put_fixed = |attrs: &mut Attrs, name: &str| {
        for (k, v) in string_rule_attrs(name) {
            let v = match *v {
                Attr::True => Value::Bool(true),
                Attr::Str(s) => Value::String(s.into()),
                Attr::Num(n) => json!(n),
            };
            attrs.insert(k, v);
        }
    };
    for rule in rules {
        match rule {
            Value::String(name) => put_fixed(&mut attrs, name),
            Value::Object(obj) => {
                let name = obj.get("name").and_then(Value::as_str).unwrap_or("");
                let empty = Map::new();
                let config = obj.get("config").and_then(Value::as_object).unwrap_or(&empty);
                match name {
                    "pattern" => {
                        if let Some(Value::String(p)) = config.get("pattern") {
                            attrs.insert("pattern", Value::String(p.clone()));
                        }
                    }
                    "minlength" | "maxlength" => {
                        if let Some(v) = numeric_config(config, "value") {
                            attrs.insert(if name == "minlength" { "minlength" } else { "maxlength" }, v);
                        }
                    }
                    other => put_fixed(&mut attrs, other),
                }
            }
            _ => {}
        }
    }
    attrs
}

/// resolve-block.ts `resolveBlockTag`.
pub enum Resolution<'a> {
    Composite(&'a str),
    Component(&'a str),
    Native(&'static str),
}

pub fn resolve_block_tag(ty: &str) -> Resolution<'_> {
    let t = ty.trim();
    if let Some(id) = t.strip_prefix("composite:") {
        return Resolution::Composite(id);
    }
    match t {
        "" => Resolution::Native("div"),
        "link" => Resolution::Native("a"),
        "image" => Resolution::Native("img"),
        "text" => Resolution::Native("span"),
        "heading" => Resolution::Native("h2"),
        "list" => Resolution::Native("ul"),
        other => Resolution::Component(other),
    }
}

fn kebab_to_pascal(kebab: &str) -> String {
    kebab
        .split('-')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let p: String = p.chars().filter(char::is_ascii_alphanumeric).collect();
            let mut c = p.chars();
            c.next().map_or(String::new(), |f| f.to_ascii_uppercase().to_string() + c.as_str())
        })
        .collect()
}

fn escape(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
}

fn text_of(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// The fixed email style map: tag -> inline style.
fn email_style(tag: &str) -> &'static str {
    match tag {
        "p" | "span" => "margin:0 0 16px;font-family:Arial,sans-serif;font-size:16px;line-height:24px;color:#1a1a1a",
        "h1" => "margin:0 0 16px;font-family:Arial,sans-serif;font-size:32px;line-height:40px;font-weight:700",
        "h2" => "margin:0 0 12px;font-family:Arial,sans-serif;font-size:24px;line-height:32px;font-weight:700",
        "h3" | "h4" | "h5" | "h6" => "margin:0 0 8px;font-family:Arial,sans-serif;font-size:18px;line-height:24px;font-weight:700",
        "a" => "color:#0b57d0;text-decoration:underline",
        "ul" | "ol" => "margin:0 0 16px;padding-left:24px",
        "li" => "margin:0 0 4px",
        "blockquote" => "margin:16px 0;padding-left:16px;border-left:4px solid #d0d0d0;font-style:italic",
        "hr" => "border:0;border-top:1px solid #d0d0d0;margin:24px 0",
        "img" => "display:block;max-width:100%;height:auto;border:0",
        "table" => "width:100%;border-collapse:collapse",
        "td" => "vertical-align:top;padding:0 8px",
        _ => "",
    }
}

/// One rendered block in all three targets.
struct Out {
    html: String,
    email: String,
    mdast: Vec<Value>,
}

struct Ctx<'a> {
    by_id: HashMap<&'a str, &'a Value>,
    /// Variant B: selector text -> declarations to inline, from the parsed sheet.
    inline: &'a dyn Fn(&str) -> Option<String>,
}

fn attrs_html(attrs: &Attrs, out: &mut String) {
    for (k, v) in attrs {
        match v {
            Value::Bool(true) => {
                out.push(' ');
                out.push_str(k);
            }
            Value::Bool(false) | Value::Null => {}
            other => {
                let _ = write!(out, " {k}=\"");
                escape(&text_of(Some(other)), out);
                out.push('"');
            }
        }
    }
}

fn element(tag: &str, attrs: &Attrs, extra_style: Option<&str>, inner: &str, email: bool) -> String {
    let mut s = String::new();
    let _ = write!(s, "<{tag}");
    attrs_html(attrs, &mut s);
    if email {
        let base = email_style(tag);
        let style = match (base, extra_style) {
            ("", None) => String::new(),
            (b, None) => b.to_string(),
            ("", Some(e)) => e.to_string(),
            (b, Some(e)) => format!("{b};{e}"),
        };
        if !style.is_empty() {
            s.push_str(" style=\"");
            escape(&style, &mut s);
            s.push('"');
        }
    }
    if matches!(tag, "img" | "hr" | "input" | "br") {
        s.push_str(" />");
    } else {
        let _ = write!(s, ">{inner}</{tag}>");
    }
    s
}

fn visit(block: &Value, children: Vec<Out>, ctx: &Ctx) -> Out {
    let ty = block.get("type").and_then(Value::as_str).unwrap_or("");
    let meta = block.get("meta").and_then(Value::as_object);
    let content = block.get("content");
    let mut attrs = rules_to_html_attrs(block.get("rules"));
    let class = meta.and_then(|m| m.get("class")).and_then(Value::as_str);
    if let Some(c) = class {
        attrs.insert("class", Value::String(c.into()));
    }
    let inlined = class.and_then(|c| (ctx.inline)(&format!(".{c}")));
    let child_html: String = children.iter().map(|c| c.html.as_str()).collect();
    let child_emails: Vec<String> = children.iter().map(|c| c.email.clone()).collect();
    let child_email: String = child_emails.concat();
    let child_md: Vec<Value> = children.into_iter().flat_map(|c| c.mdast).collect();
    let mut esc = String::new();
    escape(&text_of(content), &mut esc);
    let txt = text_of(content);

    let both = |tag: &str, attrs: &Attrs, inner_h: &str, inner_e: &str| {
        (element(tag, attrs, None, inner_h, false), element(tag, attrs, inlined.as_deref(), inner_e, true))
    };

    let (html, email, mdast) = match ty {
        "text" => {
            let (h, e) = both("p", &attrs, &esc, &esc);
            (h, e, vec![json!({"type":"paragraph","children":[{"type":"text","value":txt}]})])
        }
        "heading" => {
            let level = meta.and_then(|m| m.get("level")).and_then(Value::as_i64).unwrap_or(2).clamp(1, 6);
            let tag = format!("h{level}");
            let (h, e) = both(&tag, &attrs, &esc, &esc);
            (h, e, vec![json!({"type":"heading","depth":level,"children":[{"type":"text","value":txt}]})])
        }
        "blockquote" => {
            let (h, e) = both("blockquote", &attrs, &esc, &esc);
            (h, e, vec![json!({"type":"blockquote","children":[{"type":"paragraph","children":[{"type":"text","value":txt}]}]})])
        }
        "list" => {
            let ordered = meta.and_then(|m| m.get("ordered")) == Some(&Value::Bool(true));
            let items: Vec<String> = content
                .and_then(Value::as_array)
                .map(|a| a.iter().map(|v| text_of(Some(v))).collect())
                .unwrap_or_default();
            let tag = if ordered { "ol" } else { "ul" };
            let (mut ih, mut ie) = (String::new(), String::new());
            for it in &items {
                let mut e = String::new();
                escape(it, &mut e);
                ih.push_str(&element("li", &Attrs::new(), None, &e, false));
                ie.push_str(&element("li", &Attrs::new(), None, &e, true));
            }
            let (h, e) = both(tag, &attrs, &ih, &ie);
            let md_items: Vec<Value> = items
                .iter()
                .map(|t| json!({"type":"listItem","children":[{"type":"paragraph","children":[{"type":"text","value":t}]}]}))
                .collect();
            (h, e, vec![json!({"type":"list","ordered":ordered,"children":md_items})])
        }
        "divider" => {
            let (h, e) = both("hr", &attrs, "", "");
            (h, e, vec![json!({"type":"thematicBreak"})])
        }
        "grid" => {
            let columns = meta.and_then(|m| m.get("columns")).and_then(Value::as_i64).unwrap_or(1);
            attrs.insert("data-columns", json!(columns));
            let h = element("div", &attrs, None, &child_html, false);
            // Email has no grid: a one-row table, one cell per child.
            let cells: String = child_emails
                .iter()
                .map(|c| element("td", &Attrs::new(), None, c, true))
                .collect();
            let row = format!("<tr>{cells}</tr>");
            let e = element("table", &Attrs::new(), inlined.as_deref(), &row, true);
            (
                h,
                e,
                vec![json!({"type":"mdxJsxFlowElement","name":"Grid","attributes":[{"type":"mdxJsxAttribute","name":"columns","value":columns}],"children":child_md})],
            )
        }
        _ => match resolve_block_tag(ty) {
            Resolution::Composite(id) => {
                let name = kebab_to_pascal(id);
                attrs.insert("data-composite", Value::String(id.into()));
                let (h, e) = both("div", &attrs, &child_html, &child_email);
                (h, e, vec![json!({"type":"mdxJsxFlowElement","name":name,"attributes":[],"children":[]})])
            }
            Resolution::Native(tag) => {
                if tag == "a" {
                    if let Some(href) = meta.and_then(|m| m.get("href")) {
                        attrs.insert("href", href.clone());
                    }
                }
                if tag == "img" {
                    if let Some(src) = meta.and_then(|m| m.get("src")) {
                        attrs.insert("src", src.clone());
                    }
                    attrs.insert("alt", Value::String(txt.clone()));
                }
                let inner_h = format!("{esc}{child_html}");
                let inner_e = format!("{esc}{child_email}");
                let (h, e) = both(tag, &attrs, &inner_h, &inner_e);
                (h, e, child_md)
            }
            Resolution::Component(name) => {
                if name == "input" {
                    if let Some(n) = meta.and_then(|m| m.get("name")) {
                        attrs.insert("name", n.clone());
                    }
                    let (h, e) = both("input", &attrs, "", "");
                    (h, e, vec![])
                } else {
                    attrs.insert("data-component", Value::String(name.into()));
                    let inner_h = format!("{esc}{child_html}");
                    let inner_e = format!("{esc}{child_email}");
                    let (h, e) = both("div", &attrs, &inner_h, &inner_e);
                    (h, e, child_md)
                }
            }
        },
    };
    Out { html, email, mdast }
}

fn walk(block: &Value, ctx: &Ctx, visited: &mut HashSet<String>, depth: usize) -> Option<Out> {
    let id = block.get("id").and_then(Value::as_str).unwrap_or("");
    if depth > MAX_DEPTH || !visited.insert(id.to_string()) {
        return None;
    }
    let mut kids = Vec::new();
    if let Some(Value::Array(children)) = block.get("children") {
        for cid in children.iter().filter_map(Value::as_str) {
            if let Some(child) = ctx.by_id.get(cid) {
                if let Some(o) = walk(child, ctx, visited, depth + 1) {
                    kids.push(o);
                }
            }
        }
    }
    Some(visit(block, kids, ctx))
}

/// Compile a `.rafter` JSON string. `inline` maps a selector to the
/// declarations to inline into the email output (variant B); A passes none.
pub fn compile_with(src: &str, inline: &dyn Fn(&str) -> Option<String>) -> Result<Value, String> {
    let doc: Value = serde_json::from_str(src).map_err(|e| e.to_string())?;
    let blocks = doc.get("blocks").and_then(Value::as_array).ok_or("no blocks")?;
    let by_id = blocks
        .iter()
        .filter_map(|b| b.get("id").and_then(Value::as_str).map(|id| (id, b)))
        .collect();
    let ctx = Ctx { by_id, inline };
    let (mut html, mut email, mut md) = (String::new(), String::new(), Vec::new());
    for b in blocks.iter().filter(|b| b.get("parentId").is_none()) {
        if let Some(o) = walk(b, &ctx, &mut HashSet::new(), 0) {
            html.push_str(&o.html);
            email.push_str(&o.email);
            md.extend(o.mdast);
        }
    }
    let title = doc.pointer("/manifest/name").and_then(Value::as_str).unwrap_or("");
    let mut t = String::new();
    escape(title, &mut t);
    let email = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{t}</title></head><body style=\"margin:0;padding:24px;background:#ffffff\">{email}</body></html>"
    );
    Ok(json!({ "html": html, "mdast": {"type":"root","children":md}, "email": email }))
}

pub fn compile(src: &str) -> Result<Value, String> {
    compile_with(src, &|_| None)
}

// ---- wasm ABI: plain extern "C" over linear memory ----
//
// alloc(len) -> ptr; the host writes the input there. compile(ptr, len)
// returns a pointer to [u32 LE length][utf-8 JSON]; the host frees it with
// dealloc(ptr, 4 + length).

#[cfg(not(feature = "bindgen"))]
#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len);
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// # Safety
/// `ptr` must come from `alloc(len)` or a result of `compile`.
#[cfg(not(feature = "bindgen"))]
#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    drop(Vec::from_raw_parts(ptr, 0, len));
}

#[cfg_attr(feature = "bindgen", allow(dead_code))]
pub(crate) fn emit(result: Result<Value, String>) -> *mut u8 {
    let body = match result {
        Ok(v) => v.to_string(),
        Err(e) => json!({ "error": e }).to_string(),
    };
    let mut out = Vec::with_capacity(4 + body.len());
    out.extend_from_slice(&u32::try_from(body.len()).unwrap_or(u32::MAX).to_le_bytes());
    out.extend_from_slice(body.as_bytes());
    let p = out.as_mut_ptr();
    std::mem::forget(out);
    p
}

#[cfg_attr(feature = "bindgen", allow(dead_code))]
pub(crate) unsafe fn input<'a>(ptr: *const u8, len: usize) -> &'a str {
    std::str::from_utf8(std::slice::from_raw_parts(ptr, len)).unwrap_or("")
}

/// # Safety
/// `ptr..ptr+len` must be a live buffer from `alloc`.
#[cfg(not(feature = "bindgen"))]
#[no_mangle]
pub unsafe extern "C" fn compile_rafter(ptr: *const u8, len: usize) -> *mut u8 {
    emit(compile(input(ptr, len)))
}

/// The wasm-bindgen form of the same export, measured against plain extern "C".
#[cfg(feature = "bindgen")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn compile_js(src: &str) -> String {
    match compile(src) {
        Ok(v) => v.to_string(),
        Err(e) => json!({ "error": e }).to_string(),
    }
}
