//! The `.rafter` file (the rafters composite format) and the classes each
//! block's installed huttspawn component puts on it.
//!
//! The class strings are transcribed from huttspawn's `src/components/ui/*.classes.ts`
//! for the variants the samples use. A real build would run those files; the
//! toy holds the literals so the question stays on the CSS.

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct RafterFile {
    pub manifest: Manifest,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Block {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub children: Vec<String>,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub content: Option<serde_json::Value>,
    #[serde(default)]
    pub meta: HashMap<String, serde_json::Value>,
}

impl Block {
    pub fn meta_str(&self, key: &str) -> Option<String> {
        match self.meta.get(key)? {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            _ => None,
        }
    }
    pub fn text(&self) -> String {
        match &self.content {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(v) => v.to_string(),
            None => String::new(),
        }
    }
}

/// The wrapper the page sits on: huttspawn's `body { @apply bg-background text-foreground }`
/// plus preflight's `html { font-family: --default-font-family }`, as classes.
pub const BODY: &str = "bg-background text-foreground font-sans";

/// Classes for each part of a block, in the component's own order.
pub fn classes(block: &Block) -> Vec<(&'static str, String)> {
    let m = |k: &str| block.meta_str(k);
    match block.kind.as_str() {
        // container.classes.ts: query, size + mx-auto, padding or auto edge padding, stack gap.
        "container" => {
            let mut p = vec!["@container w-full".to_string()];
            if let Some(size) = m("size") {
                p.push(format!("max-w-{size} mx-auto"));
                p.push("px-4 @md:px-6 @lg:px-8".into());
            }
            if let Some(gap) = m("gap") {
                p.push(format!("flex flex-col gap-{gap}"));
            }
            vec![("root", p.join(" "))]
        }
        // grid.classes.ts: an explicit gap replaces the auto spacing.
        "grid" => {
            let gap = m("gap").map_or_else(
                || "gap-3 p-3 @md:gap-4 @md:p-4 @lg:gap-6 @lg:p-6 @xl:gap-8 @xl:p-8".to_string(),
                |g| format!("gap-{g}"),
            );
            let cols = m("columns").unwrap_or_else(|| "1".into());
            vec![("root", format!("grid {gap} grid-cols-{cols}"))]
        }
        // typography.classes.ts, resolveTypography(h1 | h2 | h3).
        "heading" => {
            let c = match m("level").as_deref() {
                Some("1") => "scroll-m-20 text-4xl font-bold tracking-tight text-foreground @lg:text-5xl",
                Some("3") => "scroll-m-20 text-2xl font-semibold tracking-tight text-foreground",
                _ => "scroll-m-20 text-3xl font-semibold tracking-tight text-foreground",
            };
            vec![("root", c.into())]
        }
        // typography.classes.ts, resolveTypography(p | muted).
        "text" => {
            let c = if m("variant").as_deref() == Some("muted") {
                "text-sm text-muted-foreground"
            } else {
                "leading-7 text-foreground"
            };
            vec![("root", c.into())]
        }
        // image.classes.ts, alignment center, radius from meta (default lg), status loaded.
        "image" => {
            let radius = m("radius").unwrap_or_else(|| "lg".into());
            vec![
                ("root", "relative mx-auto".into()),
                ("frame", format!("relative overflow-hidden rounded-{radius}")),
                ("img", "block w-full h-auto animate-fade-in-moderate-enter".into()),
            ]
        }
        // button.classes.ts, buttonVariants({ variant, size }).
        "button" => {
            let variant = m("variant").unwrap_or_else(|| "default".into());
            let size = m("size").unwrap_or_else(|| "default".into());
            vec![("root", button(&variant, &size))]
        }
        // separator.classes.ts, horizontal.
        "separator" => vec![("root", "shrink-0 bg-border h-px w-full".into())],
        _ => vec![],
    }
}

fn button(variant: &str, size: &str) -> String {
    let base = "inline-grid grid-flow-col items-center justify-center gap-2 rounded-md text-label-large ts-label-large cursor-pointer \
        focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 \
        transition-[color,background-color,border-color,box-shadow,scale] duration-fast ease-standard \
        scale-100 extent-press \
        active:scale-(--rafters-consumed-extent) active:duration-micro active:ease-spring-snappy \
        disabled:opacity-50 disabled:cursor-not-allowed \
        aria-disabled:opacity-50 aria-disabled:cursor-not-allowed \
        aria-busy:cursor-progress";
    let surface = match variant {
        "secondary" => "bg-secondary text-secondary-foreground hover:bg-secondary-hover active:bg-secondary-active",
        _ => "bg-primary text-primary-foreground hover:bg-primary-hover active:bg-primary-active",
    };
    let ring = match variant {
        "secondary" => "focus-visible:ring-2 focus-visible:ring-secondary-ring",
        _ => "focus-visible:ring-2 focus-visible:ring-primary-ring",
    };
    let sizing = match size {
        "lg" => "h-12 px-6 text-label-large ts-label-large has-data-[icon=inline-start]:ps-4 has-data-[icon=inline-end]:pe-4",
        _ => "h-11 px-4 py-2.5 @md:h-10 @md:py-2 has-data-[icon=inline-start]:ps-3 has-data-[icon=inline-end]:pe-3",
    };
    format!("{base} {surface} {ring} {sizing}")
}

/// Every class the file's blocks use, plus the body wrapper's.
pub fn all_classes(file: &RafterFile) -> Vec<String> {
    let mut out: Vec<String> = BODY.split_whitespace().map(String::from).collect();
    for b in &file.blocks {
        for (_, c) in classes(b) {
            out.extend(c.split_whitespace().map(String::from));
        }
    }
    out.sort();
    out.dedup();
    out
}
