//! Tailwind's compiled CSS, read with lightningcss: rules indexed by the class
//! that leads their selector, custom properties for light (`:root`) and dark
//! (`.dark`), and declarations resolved to concrete values a mail client takes.

use lightningcss::properties::custom::{CustomPropertyName, TokenList};
use lightningcss::properties::Property;
use lightningcss::rules::CssRule;
use lightningcss::stylesheet::{ParserOptions, PrinterOptions, StyleSheet};
use lightningcss::targets::{Browsers, Targets};
use lightningcss::traits::ToCss;
use lightningcss::values::color::CssColor;
use lightningcss::values::length::LengthValue;
use lightningcss::visit_types;
use lightningcss::visitor::{Visit, VisitTypes, Visitor};
use std::collections::HashMap;
use std::convert::Infallible;

/// Where a rule sits: the at-rules around it, outermost first.
#[derive(Debug, Clone, PartialEq)]
pub enum Ctx {
    Media(String),
    Container(String),
    Supports(String),
}

#[derive(Clone)]
pub struct Rule<'a, 'i> {
    pub order: usize,
    /// What follows the leading class in the selector: `""`, `":hover"`, `"[aria-busy=true]"`.
    pub suffix: String,
    pub ctx: Vec<Ctx>,
    pub decls: Vec<(&'a Property<'i>, bool)>,
}

impl Rule<'_, '_> {
    /// A rule whose declarations can go into `style`: no selector suffix, and no
    /// at-rule other than `@supports` (resolved here, since `style` cannot hold one).
    pub fn inlinable(&self) -> bool {
        self.suffix.is_empty() && self.ctx.iter().all(|c| matches!(c, Ctx::Supports(_)))
    }
}

pub struct Sheet<'a, 'i> {
    pub by_class: HashMap<String, Vec<Rule<'a, 'i>>>,
    pub light: HashMap<&'a str, TokenList<'i>>,
    pub dark: HashMap<&'a str, TokenList<'i>>,
    /// Rules the index read but no class owns (`:root`, `*`, element selectors), counted for the report.
    pub skipped: usize,
}

/// `@property` initial values, as a `:root` block to parse beside the sheet.
pub fn property_defaults(sheet: &StyleSheet) -> String {
    let mut out = String::from(":root{");
    collect_props(&sheet.rules.0, &mut out);
    out.push('}');
    out
}

fn collect_props(rules: &[CssRule], out: &mut String) {
    for r in rules {
        match r {
            CssRule::Property(p) => {
                if let Some(v) = &p.initial_value {
                    if let Ok(s) = v.to_css_string(PrinterOptions::default()) {
                        out.push_str(&format!("{}:{s};", p.name.0));
                    }
                }
            }
            CssRule::LayerBlock(l) => collect_props(&l.rules.0, out),
            _ => {}
        }
    }
}

pub fn parse(src: &str) -> Result<StyleSheet<'_>, String> {
    StyleSheet::parse(src, ParserOptions::default()).map_err(|e| e.to_string())
}

impl<'a, 'i> Sheet<'a, 'i> {
    pub fn index(sheet: &'a StyleSheet<'i>, defaults: &'a StyleSheet<'i>) -> Self {
        let mut s = Sheet { by_class: HashMap::new(), light: HashMap::new(), dark: HashMap::new(), skipped: 0 };
        let mut order = 0;
        s.walk(&defaults.rules.0, "", &[], &mut order);
        s.walk(&sheet.rules.0, "", &[], &mut order);
        // Dark is light with `.dark`'s overrides on top.
        let overrides = std::mem::take(&mut s.dark);
        s.dark = s.light.clone();
        s.dark.extend(overrides);
        s
    }

    fn walk(&mut self, rules: &'a [CssRule<'i>], parent: &str, ctx: &[Ctx], order: &mut usize) {
        for r in rules {
            match r {
                CssRule::LayerBlock(l) => self.walk(&l.rules.0, parent, ctx, order),
                CssRule::Media(m) => {
                    let q = m.query.to_css_string(PrinterOptions::default()).unwrap_or_default();
                    self.walk(&m.rules.0, parent, &push(ctx, Ctx::Media(q)), order);
                }
                CssRule::Container(c) => {
                    let q = c.condition.as_ref().and_then(|c| c.to_css_string(PrinterOptions::default()).ok()).unwrap_or_default();
                    self.walk(&c.rules.0, parent, &push(ctx, Ctx::Container(q)), order);
                }
                CssRule::Supports(sp) => {
                    let q = sp.condition.to_css_string(PrinterOptions::default()).unwrap_or_default();
                    self.walk(&sp.rules.0, parent, &push(ctx, Ctx::Supports(q)), order);
                }
                CssRule::NestedDeclarations(n) => {
                    self.add(parent, ctx, &n.declarations, order);
                }
                CssRule::Style(st) => {
                    let sel = st.selectors.to_css_string(PrinterOptions::default()).unwrap_or_default();
                    for one in split_selector_list(&sel) {
                        let full = if one.contains('&') { one.replace('&', parent) } else { one.clone() };
                        self.add(&full, ctx, &st.declarations, order);
                        self.walk(&st.rules.0, &full, ctx, order);
                    }
                }
                _ => {}
            }
        }
    }

    fn add(&mut self, selector: &str, ctx: &[Ctx], block: &'a lightningcss::declaration::DeclarationBlock<'i>, order: &mut usize) {
        *order += 1;
        let decls: Vec<(&'a Property<'i>, bool)> = block
            .declarations
            .iter()
            .map(|p| (p, false))
            .chain(block.important_declarations.iter().map(|p| (p, true)))
            .collect();
        if decls.is_empty() {
            return;
        }
        let in_media = ctx.iter().any(|c| !matches!(c, Ctx::Supports(_)));
        match selector.trim() {
            ":root" | ":host" if !in_media => {
                for (p, _) in &decls {
                    if let Some((k, v)) = custom(p) {
                        self.light.insert(k, v.clone());
                    }
                }
            }
            ".dark" if !in_media => {
                for (p, _) in &decls {
                    if let Some((k, v)) = custom(p) {
                        self.dark.insert(k, v.clone());
                    }
                }
            }
            sel => {
                if let Some((class, suffix)) = leading_class(sel) {
                    self.by_class.entry(class).or_default().push(Rule { order: *order, suffix, ctx: ctx.to_vec(), decls });
                } else {
                    self.skipped += 1;
                }
            }
        }
    }
}

fn push(ctx: &[Ctx], c: Ctx) -> Vec<Ctx> {
    let mut v = ctx.to_vec();
    v.push(c);
    v
}

pub fn custom<'a, 'i>(p: &'a Property<'i>) -> Option<(&'a str, &'a TokenList<'i>)> {
    if let Property::Custom(c) = p {
        if let CustomPropertyName::Custom(name) = &c.name {
            return Some((name.0.as_ref(), &c.value));
        }
    }
    None
}

/// Split a printed selector list on top-level commas.
fn split_selector_list(s: &str) -> Vec<String> {
    let (mut out, mut cur, mut depth, mut esc) = (Vec::new(), String::new(), 0i32, false);
    for ch in s.chars() {
        if esc {
            cur.push(ch);
            esc = false;
            continue;
        }
        match ch {
            '\\' => {
                esc = true;
                cur.push(ch);
            }
            '(' | '[' => {
                depth += 1;
                cur.push(ch);
            }
            ')' | ']' => {
                depth -= 1;
                cur.push(ch);
            }
            ',' if depth == 0 => out.push(std::mem::take(&mut cur).trim().to_string()),
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

/// `.hover\:bg-primary:hover` -> (`hover:bg-primary`, `:hover`). Only selectors that
/// start with a class count; the class is unescaped back to how it is written in HTML.
fn leading_class(sel: &str) -> Option<(String, String)> {
    let rest = sel.strip_prefix('.')?;
    let mut name = String::new();
    let mut chars = rest.char_indices().peekable();
    let mut end = rest.len();
    while let Some((i, ch)) = chars.next() {
        if ch == '\\' {
            // A hex escape (`\31 `) or a literal one (`\:`).
            let mut hex = String::new();
            while let Some(&(_, h)) = chars.peek() {
                if h.is_ascii_hexdigit() && hex.len() < 6 {
                    hex.push(h);
                    chars.next();
                } else {
                    break;
                }
            }
            if hex.is_empty() {
                if let Some((_, c)) = chars.next() {
                    name.push(c);
                }
            } else {
                if let Some(&(_, ' ')) = chars.peek() {
                    chars.next();
                }
                name.push(u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32).unwrap_or('?'));
            }
        } else if ch.is_alphanumeric() || ch == '-' || ch == '_' || !ch.is_ascii() {
            name.push(ch);
        } else {
            end = i;
            break;
        }
    }
    Some((name, rest[end..].to_string()))
}

/// Colours to sRGB and `rem` to `px` (16px root): the two lowerings every value gets.
struct Lower;
impl<'i> Visitor<'i> for Lower {
    type Error = Infallible;
    fn visit_types(&self) -> VisitTypes {
        visit_types!(COLORS | LENGTHS)
    }
    fn visit_color(&mut self, color: &mut CssColor) -> Result<(), Self::Error> {
        if let Ok(rgb) = color.to_rgb() {
            *color = rgb;
        }
        Ok(())
    }
    fn visit_length(&mut self, length: &mut LengthValue) -> Result<(), Self::Error> {
        if let LengthValue::Rem(v) = length {
            *length = LengthValue::Px(*v * 16.0);
        }
        Ok(())
    }
}

/// Printer targets with no hex-alpha, no modern colour syntax: IE 11 is the
/// floor lightningcss knows that is closest to a mail client.
pub fn printer() -> PrinterOptions<'static> {
    PrinterOptions {
        minify: true,
        targets: Targets::from(Browsers { ie: Some(11 << 16), ..Browsers::default() }),
        ..PrinterOptions::default()
    }
}

/// One declaration, its variables substituted from `vars`, lowered and printed.
/// `Err` carries why it could not be (a variable that resolves to nothing).
pub fn resolve(p: &Property, important: bool, vars: &HashMap<&str, TokenList>) -> Result<(String, String), String> {
    let mut prop: Property = match p {
        Property::Unparsed(u) => {
            let id = u.property_id.name().to_string();
            u.clone().substitute_variables(vars).map_err(|()| format!("{id}: a variable did not resolve to a valid value"))?
        }
        other => other.clone(),
    };
    let _ = prop.visit(&mut Lower);
    let printed = prop.to_css_string(important, printer()).map_err(|e| e.to_string())?;
    let (name, value) = printed.split_once(':').ok_or("unprintable")?;
    Ok((name.trim().to_string(), value.trim().to_string()))
}
