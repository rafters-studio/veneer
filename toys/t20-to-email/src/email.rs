//! Blocks to email HTML: each element's declarations inlined into `style`, table
//! layout where email needs it, and a `<style>` block holding only dark mode and hover.

use crate::css::{self, Rule, Sheet};
use crate::rafter::{self, Block, RafterFile};
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Write as _;

/// One resolved declaration and the class it came from.
#[derive(Clone, Debug)]
struct Decl {
    class: String,
    name: String,
    value: String,
}

#[derive(Default)]
struct Styled {
    base: Vec<Decl>,
    dark: Vec<Decl>,
    hover: Vec<Decl>,
    hover_dark: Vec<Decl>,
}

impl Styled {
    fn get(&self, name: &str) -> Option<&str> {
        self.base
            .iter()
            .rev()
            .find(|d| d.name == name)
            .map(|d| d.value.as_str())
    }
    /// Remove declarations the layout consumed, recording why.
    fn take(&mut self, names: &[&str], el: &str, reason: &str, report: &mut Report) {
        let mut keep = Vec::new();
        for d in self.base.drain(..) {
            if names.contains(&d.name.as_str()) {
                report.drop(el, &d, reason);
            } else {
                keep.push(d);
            }
        }
        self.base = keep;
        self.dark.retain(|d| !names.contains(&d.name.as_str()));
    }
}

#[derive(Serialize, Default)]
pub struct Report {
    pub inlined: Vec<Entry>,
    pub in_style: Vec<Entry>,
    pub dropped: Vec<Entry>,
}

#[derive(Serialize, Clone)]
pub struct Entry {
    pub element: String,
    pub class: String,
    pub property: String,
    pub value: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub reason: String,
}

impl Report {
    fn drop(&mut self, el: &str, d: &Decl, reason: &str) {
        self.dropped.push(Entry {
            element: el.into(),
            class: d.class.clone(),
            property: d.name.clone(),
            value: d.value.clone(),
            reason: reason.into(),
        });
    }
}

/// What every inlined declaration goes through: logical properties to physical
/// (left-to-right), and the properties a mail client has no use for dropped.
fn policy(d: Decl) -> Result<Vec<Decl>, (Decl, &'static str)> {
    let n = d.name.as_str();
    let drop: &[(&str, &str)] = &[
        ("transition", "motion: no mail client runs transitions"),
        (
            "animation",
            "motion: a load fade has no moment in a mail client",
        ),
        ("cursor", "interaction: no pointer states in a static email"),
        (
            "scale",
            "interaction: the press zoom needs :active and transitions",
        ),
        (
            "scroll-margin",
            "page scrolling: an email has no anchored scroll",
        ),
        (
            "container-type",
            "container queries: no mail client evaluates @container",
        ),
        (
            "position",
            "positioning: unsupported or stripped in Gmail and Outlook",
        ),
        ("flex-shrink", "flex: the element is a table cell in email"),
        ("outline", "focus ring: no keyboard focus in a static email"),
    ];
    if let Some((_, why)) = drop
        .iter()
        .find(|(p, _)| n == *p || n.starts_with(&format!("{p}-")))
    {
        return Err((d, why));
    }
    let split = |a: &str, b: &str| -> Vec<Decl> {
        let parts: Vec<&str> = d.value.split_whitespace().collect();
        let (x, y) = (
            parts.first().copied().unwrap_or(""),
            parts.get(1).or(parts.first()).copied().unwrap_or(""),
        );
        vec![
            Decl {
                name: a.into(),
                value: x.into(),
                ..d.clone()
            },
            Decl {
                name: b.into(),
                value: y.into(),
                ..d.clone()
            },
        ]
    };
    let rename = |to: &str| {
        vec![Decl {
            name: to.into(),
            ..d.clone()
        }]
    };
    Ok(match n {
        "padding-inline" => split("padding-left", "padding-right"),
        "padding-block" => split("padding-top", "padding-bottom"),
        "margin-inline" => split("margin-left", "margin-right"),
        "margin-block" => split("margin-top", "margin-bottom"),
        "padding-inline-start" => rename("padding-left"),
        "padding-inline-end" => rename("padding-right"),
        "margin-inline-start" => rename("margin-left"),
        "margin-inline-end" => rename("margin-right"),
        _ => vec![d],
    })
}

pub struct Emitter<'s, 'a, 'i> {
    sheet: &'s mut Sheet<'a, 'i>,
    pub report: Report,
    rules: Vec<String>,
    dark: Vec<String>,
    next: usize,
}

impl<'s, 'a, 'i> Emitter<'s, 'a, 'i> {
    pub fn new(sheet: &'s mut Sheet<'a, 'i>) -> Self {
        Emitter {
            sheet,
            report: Report::default(),
            rules: Vec::new(),
            dark: Vec::new(),
            next: 0,
        }
    }

    /// Resolve every declaration the classes carry, light and dark, and sort out
    /// the variant rules: hover is kept for `<style>`; the rest is dropped with its reason.
    fn resolve(&mut self, classes: &str, el: &str) -> Styled {
        let mut rules: Vec<(&str, Rule<'a, 'i>)> = Vec::new();
        for c in classes.split_whitespace() {
            match self.sheet.by_class.get(c) {
                Some(rs) => rules.extend(rs.iter().map(|r| (c, r.clone()))),
                None => self.report.dropped.push(Entry {
                    element: el.into(),
                    class: c.into(),
                    property: String::new(),
                    value: String::new(),
                    reason: "no rule in the compiled CSS".into(),
                }),
            }
        }
        rules.sort_by_key(|(_, r)| r.order);

        let mut out = Styled::default();
        let base: Vec<_> = rules
            .iter()
            .filter(|(_, r)| r.inlinable())
            .cloned()
            .collect();
        for dark in [false, true] {
            let vars = if dark {
                &mut self.sheet.dark
            } else {
                &mut self.sheet.light
            };
            let saved = overlay(vars, &base);
            for (class, r) in &base {
                for (p, imp) in &r.decls {
                    if css::custom(p).is_some() {
                        continue;
                    }
                    match css::resolve(p, *imp, vars) {
                        Ok((name, value)) => {
                            let d = Decl {
                                class: (*class).into(),
                                name,
                                value,
                            };
                            if dark {
                                out.dark.push(d)
                            } else {
                                out.base.push(d)
                            }
                        }
                        Err(why) if !dark => self.report.dropped.push(Entry {
                            element: el.into(),
                            class: (*class).into(),
                            property: p.property_id().name().into(),
                            value: String::new(),
                            reason: why,
                        }),
                        Err(_) => {}
                    }
                }
            }
            restore(vars, saved);
        }
        // Dark keeps only what differs from light.
        let light: HashMap<_, _> = out
            .base
            .iter()
            .map(|d| (d.name.clone(), d.value.clone()))
            .collect();
        out.dark.retain(|d| light.get(&d.name) != Some(&d.value));

        for (class, r) in rules.iter().filter(|(_, r)| !r.inlinable()) {
            let hover = r.suffix == ":hover"
                && r.ctx
                    .iter()
                    .all(|c| matches!(c, css::Ctx::Media(m) if m.contains("hover")));
            let mut saved = overlay(&mut self.sheet.light, &base);
            saved.extend(overlay(
                &mut self.sheet.light,
                std::slice::from_ref(&(*class, r.clone())),
            ));
            for (p, imp) in &r.decls {
                if css::custom(p).is_some() {
                    continue;
                }
                let resolved = css::resolve(p, *imp, &self.sheet.light)
                    .unwrap_or_else(|e| (p.property_id().name().into(), e));
                let d = Decl {
                    class: (*class).into(),
                    name: resolved.0,
                    value: resolved.1,
                };
                if hover {
                    // The same hover under the dark variables, so a dark reader does not hover to the light colour.
                    let mut dsaved = overlay(&mut self.sheet.dark, &base);
                    dsaved.extend(overlay(
                        &mut self.sheet.dark,
                        std::slice::from_ref(&(*class, r.clone())),
                    ));
                    if let Ok((n, v)) = css::resolve(p, *imp, &self.sheet.dark) {
                        out.hover_dark.push(Decl {
                            class: (*class).into(),
                            name: n,
                            value: v,
                        });
                    }
                    restore(&mut self.sheet.dark, dsaved);
                    out.hover.push(d);
                } else {
                    let why = variant_reason(r);
                    self.report.drop(el, &d, why);
                }
            }
            restore(&mut self.sheet.light, saved);
        }

        // The policy applies to everything inlined.
        let mut base = Vec::new();
        for d in out.base.drain(..) {
            match policy(d) {
                Ok(ds) => base.extend(ds),
                Err((d, why)) => self.report.drop(el, &d, why),
            }
        }
        out.base = base;
        out.dark = out
            .dark
            .into_iter()
            .filter_map(|d| policy(d).ok())
            .flatten()
            .collect();
        out
    }

    /// Print `style="..."` and, when dark or hover rules exist, a class naming them.
    fn attrs(&mut self, s: &Styled, el: &str) -> String {
        let s = &Styled {
            base: last_wins(&s.base),
            dark: last_wins(&s.dark),
            hover: last_wins(&s.hover),
            hover_dark: last_wins(&s.hover_dark),
        };
        let mut a = String::new();
        if !s.dark.is_empty() || !s.hover.is_empty() {
            self.next += 1;
            let cls = format!("r{}", self.next);
            if !s.dark.is_empty() {
                let body: Vec<String> = s
                    .dark
                    .iter()
                    .map(|d| format!("{}:{} !important", d.name, d.value))
                    .collect();
                self.dark.push(format!(".{cls}{{{}}}", body.join(";")));
                for d in &s.dark {
                    self.report.in_style.push(Entry {
                        element: el.into(),
                        class: d.class.clone(),
                        property: d.name.clone(),
                        value: d.value.clone(),
                        reason: "dark: @media (prefers-color-scheme: dark)".into(),
                    });
                }
            }
            if !s.hover.is_empty() {
                let body: Vec<String> = s
                    .hover
                    .iter()
                    .map(|d| format!("{}:{} !important", d.name, d.value))
                    .collect();
                self.rules
                    .push(format!(".{cls}:hover{{{}}}", body.join(";")));
                if !s.hover_dark.is_empty() {
                    let body: Vec<String> = s
                        .hover_dark
                        .iter()
                        .map(|d| format!("{}:{} !important", d.name, d.value))
                        .collect();
                    self.dark
                        .push(format!(".{cls}:hover{{{}}}", body.join(";")));
                }
                for d in &s.hover {
                    self.report.in_style.push(Entry {
                        element: el.into(),
                        class: d.class.clone(),
                        property: d.name.clone(),
                        value: d.value.clone(),
                        reason: ":hover".into(),
                    });
                }
            }
            let _ = write!(a, " class=\"{cls}\"");
        }
        if !s.base.is_empty() {
            let body: Vec<String> = s
                .base
                .iter()
                .map(|d| format!("{}:{}", d.name, d.value))
                .collect();
            let _ = write!(a, " style=\"{}\"", esc(&body.join(";")));
            for d in &s.base {
                self.report.inlined.push(Entry {
                    element: el.into(),
                    class: d.class.clone(),
                    property: d.name.clone(),
                    value: d.value.clone(),
                    reason: String::new(),
                });
            }
        }
        a
    }

    pub fn render(&mut self, file: &RafterFile) -> String {
        let by_id: HashMap<&str, &Block> = file.blocks.iter().map(|b| (b.id.as_str(), b)).collect();
        let roots: Vec<&Block> = file
            .blocks
            .iter()
            .filter(|b| b.parent_id.is_none())
            .collect();
        // The outer width: the first root container's max-width, else 600px.
        let width = roots
            .first()
            .map(|b| self.resolve(&class_of(b, "root"), "probe"))
            .and_then(|s| s.get("max-width").and_then(px))
            .unwrap_or(600.0);
        // The probe resolve wrote report entries; drop them.
        self.report = Report::default();
        let body = self.resolve(rafter::BODY, "body");
        let font = body.get("font-family").map(String::from);
        let bg = body.get("background-color").map(String::from);

        let ctx = Ctx {
            font: font.clone(),
            width,
            stretch: false,
        };
        let mut inner = String::new();
        for b in roots {
            inner.push_str(&self.block(b, &by_id, &ctx));
        }

        let body_attrs = self.attrs(&body, "body");
        // The <body> carries the same dark class, so its own background switches with the wrapper's.
        let body_class = body_attrs
            .split("style=")
            .next()
            .unwrap_or("")
            .trim_end()
            .to_string();
        let bgcolor = bg
            .as_deref()
            .map(|c| format!(" bgcolor=\"{c}\""))
            .unwrap_or_default();
        let w = width.round();
        let mut html = String::new();
        let _ = write!(
            html,
            "<!DOCTYPE html>\n<html lang=\"en\" xmlns:v=\"urn:schemas-microsoft-com:vml\" xmlns:o=\"urn:schemas-microsoft-com:office:office\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<meta name=\"color-scheme\" content=\"light dark\">\n<meta name=\"supported-color-schemes\" content=\"light dark\">\n<title>{}</title>\n<!--[if mso]><noscript><xml><o:OfficeDocumentSettings><o:PixelsPerInch>96</o:PixelsPerInch></o:OfficeDocumentSettings></xml></noscript><![endif]-->\n<style>\n:root{{color-scheme:light dark;supported-color-schemes:light dark}}\n{}\n</style>\n</head>\n<body{body_class} style=\"margin:0;padding:0;{}\">\n<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\"{bgcolor}><tr><td align=\"center\"{body_attrs}>\n<!--[if mso]><table role=\"presentation\" width=\"{w}\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\"><tr><td><![endif]-->\n{inner}<!--[if mso]></td></tr></table><![endif]-->\n</td></tr></table>\n</body>\n</html>\n",
            esc(&file.manifest.name),
            format!("{}\n@media (prefers-color-scheme: dark){{\n{}\n}}", self.rules.join("\n"), self.dark.join("\n")),
            bg.map(|c| format!("background-color:{c}")).unwrap_or_default(),
        );
        html
    }

    fn block(&mut self, b: &Block, by_id: &HashMap<&str, &Block>, ctx: &Ctx) -> String {
        let el = format!("{}#{}", b.kind, b.id);
        let kids: Vec<&Block> = b
            .children
            .iter()
            .filter_map(|c| by_id.get(c.as_str()).copied())
            .collect();
        match b.kind.as_str() {
            "container" => {
                let mut s = self.resolve(&class_of(b, "root"), &el);
                let gap = s
                    .get("row-gap")
                    .or(s.get("gap"))
                    .and_then(px)
                    .unwrap_or(0.0);
                let pad: Vec<String> = [
                    "padding-top",
                    "padding-right",
                    "padding-bottom",
                    "padding-left",
                ]
                .iter()
                .map(|p| s.get(p).unwrap_or("0").to_string())
                .collect();
                let centred = s.get("margin-left") == Some("auto");
                let column =
                    s.get("display") == Some("flex") && s.get("flex-direction") == Some("column");
                let kid_ctx = Ctx {
                    stretch: column
                        && s.get("align-items")
                            .is_none_or(|a| a == "stretch" || a == "normal"),
                    ..ctx.clone()
                };
                let ctx = &kid_ctx;
                s.take(&["display", "flex-direction", "gap", "row-gap", "column-gap", "width", "margin-left", "margin-right", "padding-top", "padding-right", "padding-bottom", "padding-left"], &el, "became table layout: a row per child, gap as padding-top, padding on each cell, auto margins as align=center", &mut self.report);
                let n = kids.len();
                let mut rows = String::new();
                for (i, k) in kids.iter().enumerate() {
                    let top = if i == 0 {
                        pad[0].clone()
                    } else {
                        format!("{gap}px")
                    };
                    let bottom = if i + 1 == n {
                        pad[2].clone()
                    } else {
                        "0".into()
                    };
                    let _ = writeln!(
                        rows,
                        "<tr><td style=\"padding:{top} {} {bottom} {}\">{}</td></tr>",
                        pad[1],
                        pad[3],
                        self.block(k, by_id, ctx)
                    );
                }
                let align = if centred { " align=\"center\"" } else { "" };
                let attrs = self.attrs(
                    &Styled {
                        base: with(&s.base, "width", "100%"),
                        ..s
                    },
                    &el,
                );
                format!("<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\"{align}{attrs}>\n{rows}</table>\n")
            }
            "grid" => {
                let mut s = self.resolve(&class_of(b, "root"), &el);
                let gap = s
                    .get("column-gap")
                    .or(s.get("gap"))
                    .and_then(px)
                    .unwrap_or(0.0);
                let cols = b
                    .meta_str("columns")
                    .and_then(|c| c.parse::<usize>().ok())
                    .unwrap_or(1)
                    .max(1);
                s.take(&["display", "grid-template-columns", "gap", "row-gap", "column-gap"], &el, "became table layout: one cell per column, the gap split as padding between cells", &mut self.report);
                let mut rows = String::new();
                for (r, chunk) in kids.chunks(cols).enumerate() {
                    rows.push_str("<tr>");
                    for (i, k) in chunk.iter().enumerate() {
                        let l = if i == 0 { 0.0 } else { gap / 2.0 };
                        let rr = if i + 1 == cols { 0.0 } else { gap / 2.0 };
                        let t = if r == 0 { 0.0 } else { gap };
                        let w = 100 / cols;
                        let _ = write!(rows, "<td width=\"{w}%\" valign=\"top\" style=\"width:{w}%;padding:{t}px {rr}px 0 {l}px\">{}</td>", self.block(k, by_id, ctx));
                    }
                    rows.push_str("</tr>\n");
                }
                let attrs = self.attrs(&s, &el);
                format!("<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\"{attrs}>\n{rows}</table>\n")
            }
            "heading" | "text" => {
                let tag = if b.kind == "text" {
                    "p".to_string()
                } else {
                    format!("h{}", b.meta_str("level").unwrap_or_else(|| "2".into()))
                };
                let mut s = self.resolve(&class_of(b, "root"), &el);
                // Preflight zeroes heading and paragraph margins; mail clients do not.
                s.base.insert(
                    0,
                    Decl {
                        class: "(preflight)".into(),
                        name: "margin".into(),
                        value: "0".into(),
                    },
                );
                inherit_font(&mut s, ctx);
                let attrs = self.attrs(&s, &el);
                format!("<{tag}{attrs}>{}</{tag}>", esc(&b.text()))
            }
            "image" => {
                let mut s = Styled::default();
                for part in ["root", "frame", "img"] {
                    let p = self.resolve(&class_of(b, part), &format!("{el}.{part}"));
                    s.base.extend(p.base);
                    s.dark.extend(p.dark);
                }
                s.take(
                    &["overflow", "margin-left", "margin-right"],
                    &el,
                    "the frame folds into the img: its radius goes on the img itself",
                    &mut self.report,
                );
                s.base.push(Decl {
                    class: "(email reset)".into(),
                    name: "border".into(),
                    value: "0".into(),
                });
                let w = b
                    .meta_str("width")
                    .and_then(|w| w.parse::<f32>().ok())
                    .unwrap_or(ctx.width)
                    .min(ctx.width);
                let src = b.meta_str("src").unwrap_or_default();
                let alt = b.meta_str("alt").unwrap_or_default();
                let attrs = self.attrs(&s, &el);
                format!(
                    "<img src=\"{}\" alt=\"{}\" width=\"{}\"{attrs}>",
                    esc(&src),
                    esc(&alt),
                    w.round()
                )
            }
            "button" => self.button(b, &el, ctx),
            "separator" => {
                let mut s = self.resolve(&class_of(b, "root"), &el);
                s.take(
                    &["width", "height"],
                    &el,
                    "the rule is a one-pixel table cell",
                    &mut self.report,
                );
                for (n, v) in [
                    ("height", "1px"),
                    ("line-height", "1px"),
                    ("font-size", "1px"),
                ] {
                    s.base.push(Decl {
                        class: "(email rule)".into(),
                        name: n.into(),
                        value: v.into(),
                    });
                }
                let attrs = self.attrs(&s, &el);
                format!("<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\"><tr><td height=\"1\"{attrs}>&nbsp;</td></tr></table>")
            }
            other => {
                self.report.dropped.push(Entry {
                    element: el,
                    class: String::new(),
                    property: String::new(),
                    value: String::new(),
                    reason: format!("block type {other} has no email rendering"),
                });
                String::new()
            }
        }
    }

    /// The button: a cell carrying the surface (so Outlook paints it, with
    /// mso-padding-alt for its padding) around an inline-block link carrying the padding.
    fn button(&mut self, b: &Block, el: &str, ctx: &Ctx) -> String {
        let mut s = self.resolve(&class_of(b, "root"), el);
        // A unitless line-height is a multiple of the font size.
        let fs = s.get("font-size").and_then(px);
        let lh = s
            .get("line-height")
            .and_then(|v| px(v).or_else(|| v.parse::<f32>().ok().zip(fs).map(|(m, f)| m * f)));
        let h = s.get("height").and_then(px);
        // A fixed height becomes vertical padding around one line of text.
        if let (Some(h), Some(lh)) = (h, lh) {
            let pad = ((h - lh) / 2.0).max(0.0);
            let has = s.get("padding-top").and_then(px).unwrap_or(0.0);
            if pad > has {
                for n in ["padding-top", "padding-bottom"] {
                    s.base.retain(|d| d.name != n);
                    s.base.push(Decl {
                        class: "(height)".into(),
                        name: n.into(),
                        value: format!("{pad}px"),
                    });
                }
            }
        }
        s.take(&["height", "align-items", "justify-content", "grid-auto-flow", "gap"], el, "one line of text in an inline-block link: height became vertical padding, alignment is text-align", &mut self.report);
        for d in &mut s.base {
            if d.name == "display" {
                // In a stretching flex column the site's button fills the row; inline-block alone would shrink it.
                let to = if ctx.stretch { "block" } else { "inline-block" };
                self.report.dropped.push(Entry {
                    element: el.into(),
                    class: d.class.clone(),
                    property: "display".into(),
                    value: d.value.clone(),
                    reason: format!("inline-grid became {to}"),
                });
                d.value = to.into();
            }
        }
        s.base.push(Decl {
            class: "(email link)".into(),
            name: "text-decoration".into(),
            value: "none".into(),
        });
        s.base.push(Decl {
            class: "(email link)".into(),
            name: "text-align".into(),
            value: "center".into(),
        });
        inherit_font(&mut s, ctx);

        let get = |n: &str| s.get(n).map(String::from);
        let bg = get("background-color");
        let radius = get("border-radius");
        let pads: Vec<String> = [
            "padding-top",
            "padding-right",
            "padding-bottom",
            "padding-left",
        ]
        .iter()
        .map(|p| get(p).unwrap_or_else(|| "0".into()))
        .collect();
        let mut cell = Styled::default();
        for (n, v) in [
            ("background-color", bg.clone()),
            ("border-radius", radius),
            ("mso-padding-alt", Some(pads.join(" "))),
        ] {
            if let Some(v) = v {
                cell.base.push(Decl {
                    class: "(button cell)".into(),
                    name: n.into(),
                    value: v,
                });
            }
        }
        cell.dark = s
            .dark
            .iter()
            .filter(|d| d.name == "background-color")
            .cloned()
            .collect();
        let cell_attrs = self.attrs(&cell, &format!("{el}.cell"));
        let link_attrs = self.attrs(&s, el);
        let href = b.meta_str("href").unwrap_or_default();
        let bgcolor = bg.map(|c| format!(" bgcolor=\"{c}\"")).unwrap_or_default();
        let width = if ctx.stretch { " width=\"100%\"" } else { "" };
        format!("<table role=\"presentation\"{width} cellpadding=\"0\" cellspacing=\"0\" border=\"0\"><tr><td{bgcolor}{cell_attrs}><a href=\"{}\" target=\"_blank\"{link_attrs}>{}</a></td></tr></table>", esc(&href), esc(&b.text()))
    }
}

#[derive(Clone)]
struct Ctx {
    font: Option<String>,
    width: f32,
    /// The parent is a flex column with the default `align-items: stretch`: its children fill the width.
    stretch: bool,
}

/// Outlook and some webmails do not carry font-family into nested tables, so text elements repeat it.
/// The size the design inherits is the browser default under preflight, 16px; Outlook's own default is not.
/// Then, for the Word engine: unitless line-height and em letter-spacing become px (caniemail:
/// Word renders em letter-spacing small and line-height oddly), with mso-line-height-rule:exactly.
fn inherit_font(s: &mut Styled, ctx: &Ctx) {
    if s.get("font-size").is_none() {
        s.base.push(Decl {
            class: "(browser default under preflight)".into(),
            name: "font-size".into(),
            value: "16px".into(),
        });
    }
    let fs = s.get("font-size").and_then(px).unwrap_or(16.0);
    let round = |v: f32| format!("{}px", (v * 100.0).round() / 100.0);
    for d in &mut s.base {
        if d.name == "line-height" {
            if let Ok(m) = d.value.parse::<f32>() {
                d.value = round(m * fs);
            }
        }
        if d.name == "letter-spacing" {
            if let Some(m) = d
                .value
                .strip_suffix("em")
                .and_then(|v| v.parse::<f32>().ok())
            {
                d.value = round(m * fs);
            }
        }
    }
    s.base.push(Decl {
        class: "(outlook)".into(),
        name: "mso-line-height-rule".into(),
        value: "exactly".into(),
    });
    if s.get("font-family").is_none() {
        if let Some(f) = &ctx.font {
            s.base.push(Decl {
                class: "(inherited from body)".into(),
                name: "font-family".into(),
                value: f.clone(),
            });
        }
    }
}

/// Inline style has one value per property: the last declaration wins, as in the cascade.
fn last_wins(ds: &[Decl]) -> Vec<Decl> {
    ds.iter()
        .enumerate()
        .filter(|(i, d)| !ds[i + 1..].iter().any(|e| e.name == d.name))
        .map(|(_, d)| d.clone())
        .collect()
}

fn with(base: &[Decl], name: &str, value: &str) -> Vec<Decl> {
    let mut v = base.to_vec();
    v.push(Decl {
        class: "(email table)".into(),
        name: name.into(),
        value: value.into(),
    });
    v
}

fn class_of(b: &Block, part: &str) -> String {
    rafter::classes(b)
        .into_iter()
        .find(|(p, _)| *p == part)
        .map(|(_, c)| c)
        .unwrap_or_default()
}

fn px(v: &str) -> Option<f32> {
    v.strip_suffix("px")
        .and_then(|n| n.parse().ok())
        .or_else(|| (v == "0").then_some(0.0))
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn variant_reason(r: &Rule) -> &'static str {
    if r.ctx.iter().any(|c| matches!(c, css::Ctx::Container(_))) {
        "container query: no mail client evaluates @container, so the base value stands"
    } else if r.suffix.starts_with(":focus") {
        "focus state: no keyboard focus in a static email"
    } else if r.suffix.starts_with(":active") {
        "press state: needs :active and transitions; left out"
    } else if r.suffix.starts_with(":disabled") || r.suffix.starts_with("[aria") {
        "state the email never has (disabled, busy)"
    } else if r.suffix.starts_with(":has") {
        "the button has no icon, and :has is not supported in mail"
    } else {
        "selector variant with no email equivalent"
    }
}

type Saved<'a, 'i> = Vec<(
    &'a str,
    Option<lightningcss::properties::custom::TokenList<'i>>,
)>;

/// Put a rule set's own custom properties over the variables; return what to restore.
fn overlay<'a, 'i>(
    vars: &mut HashMap<&'a str, lightningcss::properties::custom::TokenList<'i>>,
    rules: &[(&str, Rule<'a, 'i>)],
) -> Saved<'a, 'i> {
    let mut saved = Vec::new();
    for (_, r) in rules {
        for (p, _) in &r.decls {
            if let Some((k, v)) = css::custom(p) {
                saved.push((k, vars.insert(k, v.clone())));
            }
        }
    }
    saved
}

fn restore<'a, 'i>(
    vars: &mut HashMap<&'a str, lightningcss::properties::custom::TokenList<'i>>,
    saved: Saved<'a, 'i>,
) {
    for (k, old) in saved.into_iter().rev() {
        match old {
            Some(v) => {
                vars.insert(k, v);
            }
            None => {
                vars.remove(k);
            }
        }
    }
}
