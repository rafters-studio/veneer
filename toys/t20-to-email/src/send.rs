//! What a send needs beyond the styled HTML: the plain-text alternative, the
//! preheader, links a mail client can follow, lines a mail server will carry,
//! and the `{ subject, text, html }` shape huttspawn's sender already takes.

use crate::rafter::{Block, RafterFile};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct Send {
    pub subject: String,
    pub preview: Option<String>,
    pub text: String,
    pub html: String,
}

/// The inbox preview line: the `preview` block's text, if the file has one.
pub fn preview_text(file: &RafterFile) -> Option<String> {
    file.blocks
        .iter()
        .find(|b| b.kind == "preview")
        .map(Block::text)
}

/// Clients show the first text they find as the preview line. The preheader puts the
/// chosen line first, hidden every way caniemail shows one way failing, then pads
/// with invisible characters so body text does not run on into the preview
/// (the padding is react-email's `Preview`, to 150 characters).
pub fn preheader(file: &RafterFile) -> String {
    const MAX: usize = 150;
    const PAD: &str = "\u{a0}\u{200c}\u{200b}\u{200d}\u{200e}\u{200f}\u{feff}";
    let Some(text) = preview_text(file) else {
        return String::new();
    };
    let text: String = text.chars().take(MAX).collect();
    // A plain space between groups: invisible in the preview, and a place the
    // line wrap may break, since the padding itself holds no ASCII space.
    let pad = vec![PAD; MAX.saturating_sub(text.chars().count())].join(" ");
    format!(
        "<div style=\"display:none;overflow:hidden;line-height:1px;opacity:0;max-height:0;max-width:0;mso-hide:all\">{}{pad}</div>\n",
        crate::email::esc(&text)
    )
}

/// The plain-text alternative, written from the blocks rather than scraped from
/// the HTML: headings and paragraphs as written, a button as its label over its
/// URL, a separator as a rule, images and the preheader left out.
pub fn plain_text(file: &RafterFile) -> String {
    let by_id: HashMap<&str, &Block> = file.blocks.iter().map(|b| (b.id.as_str(), b)).collect();
    let mut parts = Vec::new();
    for b in file.blocks.iter().filter(|b| b.parent_id.is_none()) {
        walk(b, &by_id, &mut parts);
    }
    let mut out = parts.join("\n\n");
    out.push('\n');
    out
}

fn walk(b: &Block, by_id: &HashMap<&str, &Block>, out: &mut Vec<String>) {
    match b.kind.as_str() {
        "heading" | "text" => out.push(b.text()),
        "button" => out.push(format!(
            "{}\n{}",
            b.text(),
            b.meta_str("href").unwrap_or_default()
        )),
        "separator" => out.push("-".repeat(40)),
        "image" | "preview" => {}
        _ => {
            for c in &b.children {
                if let Some(k) = by_id.get(c.as_str()) {
                    walk(k, by_id, out);
                }
            }
        }
    }
}

/// A mail client has no base URL to resolve against (caniemail: Gmail removes
/// `<base>`), so every link and image must be absolute.
pub fn check_urls(file: &RafterFile) -> Result<(), String> {
    for b in &file.blocks {
        for key in ["href", "src"] {
            if let Some(u) = b.meta_str(key) {
                let ok = ["https://", "http://", "mailto:"]
                    .iter()
                    .any(|p| u.starts_with(p));
                if !ok {
                    return Err(format!("block {}: {key} \"{u}\" is not an absolute URL; a mail client cannot resolve it", b.id));
                }
            }
        }
    }
    Ok(())
}

/// RFC 5322 2.1.1: a line is at most 998 octets. Break any line past `limit` at a
/// space a newline cannot change: between a tag's attributes (outside quotes) or
/// between words of text. Never inside an attribute value or between two tags.
pub fn wrap_lines(html: &str, limit: usize) -> String {
    let mut out = String::with_capacity(html.len() + html.len() / limit);
    let mut line = 0usize;
    let (mut in_tag, mut quote) = (false, None::<char>);
    for ch in html.chars() {
        match ch {
            '<' if quote.is_none() => in_tag = true,
            '>' if quote.is_none() => in_tag = false,
            '"' | '\'' if in_tag => {
                quote = match quote {
                    None => Some(ch),
                    Some(q) if q == ch => None,
                    q => q,
                }
            }
            _ => {}
        }
        if ch == '\n' {
            line = 0;
            out.push(ch);
            continue;
        }
        if ch == ' ' && quote.is_none() && line >= limit {
            out.push('\n');
            line = 0;
            continue;
        }
        line += ch.len_utf8();
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_paragraph_wraps_under_998_octets_without_touching_attributes() {
        let words = "word ".repeat(700);
        let html = format!("<p style=\"margin:0; color:#000\" class=\"r1\">{words}</p>");
        let out = wrap_lines(&html, 900);
        assert!(out.lines().all(|l| l.len() <= 998));
        assert!(out.contains("style=\"margin:0; color:#000\""));
        assert_eq!(out.replace('\n', " "), html);
    }

    #[test]
    fn a_relative_link_is_refused() {
        let f: RafterFile = serde_json::from_str(
            r#"{"manifest":{"id":"x","name":"x"},"blocks":[{"id":"b","type":"button","meta":{"href":"/verify"}}]}"#,
        )
        .unwrap();
        assert!(check_urls(&f).unwrap_err().contains("block b"));
    }
}
