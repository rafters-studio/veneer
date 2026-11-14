//! Toy: build previews from the rafters REGISTRY instead of a directory walk.
//!
//! Reads registry JSON as `rafters.studio/registry/components/<name>.json`
//! serves it, and for each component takes two things it already publishes:
//!
//! - `files[].content` for `<name>.classes.ts`, the source of the classes
//!   function -- so the root part is resolved by the component's own composer.
//! - `facets.react.props`, the typed prop domains AND their declared defaults
//!   -- so the knob surface and its starting position are read, never derived
//!   from record insertion order.
//!
//! Nothing here walks a project, reconciles a config's installed list, or
//! guesses a default. Run:
//!
//!     cargo run -p veneer-adapters --example registry_toy -- <dir-of-json> <out.html>

use std::collections::BTreeMap;
use std::path::Path;

use veneer_adapters::read_class_parts_for;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().expect("usage: registry_toy <json-dir> <out.html>");
    let out = args.next().expect("usage: registry_toy <json-dir> <out.html>");

    let mut sections = String::new();
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .expect("registry dir must read")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    entries.sort();

    for path in entries {
        match section_for(&path) {
            Ok(section) => sections.push_str(&section),
            Err(reason) => {
                let name = path.file_stem().unwrap_or_default().to_string_lossy();
                sections.push_str(&format!(
                    "<section class=\"card\"><h2>{name}</h2><p class=\"skip\">{reason}</p></section>\n"
                ));
            }
        }
    }

    std::fs::write(&out, page(&sections)).expect("write html");
    println!("wrote {out}");
}

/// One component's panel, or the named reason it has none.
fn section_for(path: &Path) -> Result<String, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let entry: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    let name = entry["name"].as_str().unwrap_or("unknown").to_string();

    // The classes source, as the registry ships it.
    let classes_src = entry["files"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|f| {
            f["path"]
                .as_str()
                .is_some_and(|p| p.ends_with(".classes.ts"))
        })
        .and_then(|f| f["content"].as_str())
        .ok_or("registry entry ships no .classes.ts")?;

    // The knob surface, as the registry declares it: domains and defaults.
    let props = entry["facets"]["react"]["props"]
        .as_object()
        .ok_or("registry entry declares no react props")?;

    let mut knobs: Vec<(String, Vec<String>, String)> = Vec::new();
    for (prop, spec) in props {
        let Some(values) = spec["values"].as_array() else {
            continue;
        };
        let values: Vec<String> = values
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        if values.is_empty() {
            continue;
        }
        let default = spec["default"]
            .as_str()
            .unwrap_or(&values[0])
            .to_string();
        knobs.push((prop.clone(), values, default));
    }
    if knobs.is_empty() {
        return Err("no enum props to turn".into());
    }

    // Resolve the root once per combination the registry declares.
    let combos = combinations(&knobs);
    let mut resolved: Vec<(String, String)> = Vec::new();
    for combo in &combos {
        let config: BTreeMap<String, String> = combo.iter().cloned().collect();
        let Some(parts) = read_class_parts_for(&name, classes_src, &config) else {
            continue;
        };
        let Some(root) = parts.root() else { continue };
        let key = combo
            .iter()
            .map(|(_, v)| v.as_str())
            .collect::<Vec<_>>()
            .join("/");
        resolved.push((key, root.to_string()));
    }
    if resolved.is_empty() {
        return Err("classes function declared no root part".into());
    }

    let snippet = entry["facets"]["react"]["snippet"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let load = entry["intelligence"]["cognitiveLoad"]
        .as_u64()
        .map(|n| format!("cognitive load {n}/10"))
        .unwrap_or_default();

    let selects: String = knobs
        .iter()
        .map(|(prop, values, default)| {
            let options: String = values
                .iter()
                .map(|v| {
                    let sel = if v == default { " selected" } else { "" };
                    format!("<option{sel}>{v}</option>")
                })
                .collect();
            format!("<label>{prop}<select data-prop=\"{prop}\">{options}</select></label>")
        })
        .collect();

    let table: String = resolved
        .iter()
        .map(|(k, v)| format!("      \"{k}\": \"{v}\""))
        .collect::<Vec<_>>()
        .join(",\n");

    let start = knobs
        .iter()
        .map(|(_, _, d)| d.as_str())
        .collect::<Vec<_>>()
        .join("/");

    Ok(format!(
        r#"<section class="card" data-component="{name}">
  <header><h2>{name}</h2><span class="meta">{load} &middot; {count} configs</span></header>
  <div class="stage"><span class="preview" data-start="{start}">Save</span></div>
  <div class="knobs">{selects}</div>
  <code class="snippet">{snippet}</code>
  <script type="application/json" class="roots">{{
{table}
  }}</script>
</section>
"#,
        count = resolved.len(),
        snippet = escape_html(&snippet),
    ))
}

/// Every combination of the declared knob values, knob order preserved.
fn combinations(knobs: &[(String, Vec<String>, String)]) -> Vec<Vec<(String, String)>> {
    let mut out: Vec<Vec<(String, String)>> = vec![Vec::new()];
    for (prop, values, _) in knobs {
        let mut next = Vec::new();
        for base in &out {
            for value in values {
                let mut row = base.clone();
                row.push((prop.clone(), value.clone()));
                next.push(row);
            }
        }
        out = next;
    }
    out
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn page(sections: &str) -> String {
    format!(
        r#"<!doctype html>
<meta charset="utf-8">
<title>veneer toy - previews from the rafters registry</title>
<link rel="stylesheet" href="./sheet.css">
<style>
  body {{ font: 14px system-ui, sans-serif; margin: 0; padding: 2rem; background: #0b0b0c; color: #e7e7ea; }}
  h1 {{ font-size: 1.1rem; font-weight: 600; margin: 0 0 .25rem; }}
  .lede {{ color: #9a9aa2; margin: 0 0 2rem; max-width: 60ch; line-height: 1.5; }}
  .card {{ background: #141416; border: 1px solid #26262b; border-radius: 10px;
           padding: 1rem 1.25rem; margin-bottom: 1rem; }}
  .card header {{ display: flex; align-items: baseline; gap: .75rem; margin-bottom: .75rem; }}
  h2 {{ font-size: .95rem; margin: 0; }}
  .meta {{ color: #7c7c85; font-size: .8rem; }}
  .stage {{ background: #fff; border-radius: 8px; padding: 1.5rem; display: flex;
            justify-content: center; margin-bottom: .75rem; }}
  .knobs {{ display: flex; gap: 1rem; flex-wrap: wrap; margin-bottom: .5rem; }}
  label {{ display: flex; flex-direction: column; gap: .2rem; font-size: .75rem; color: #9a9aa2; }}
  select {{ font: inherit; padding: .2rem .3rem; }}
  .snippet {{ color: #7c7c85; font-size: .8rem; font-family: ui-monospace, monospace; }}
  .skip {{ color: #b06; font-size: .85rem; margin: 0; }}
  .roots {{ display: none; }}
</style>
<h1>previews built from the rafters registry</h1>
<p class="lede">Every class string below was produced by the component's own classes
function, resolved once per config. The knobs and their starting positions are the
prop domains the registry declares. Nothing here walked a project directory or
guessed a default.</p>
{sections}
<script>
for (const card of document.querySelectorAll('.card[data-component]')) {{
  const roots = JSON.parse(card.querySelector('.roots').textContent);
  const preview = card.querySelector('.preview');
  const selects = [...card.querySelectorAll('select')];
  const apply = () => {{
    const key = selects.map(s => s.value).join('/');
    preview.className = 'preview ' + (roots[key] ?? roots[preview.dataset.start] ?? '');
  }};
  selects.forEach(s => s.addEventListener('change', apply));
  apply();
}}
</script>
"#
    )
}
