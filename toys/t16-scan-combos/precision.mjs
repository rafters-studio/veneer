// Joins the hand verdicts to the combinations. Every row in a family was read, so these are a census
// of this corpus and not a sample. Writes results/precision.csv and prints the tables.
// Usage: node precision.mjs
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { CLASSES, GATES, QUALIFY, SITES, load, run, tokens } from "./lib.mjs";
import { DIV, SECTIONING, rawVerdict } from "./verdicts.mjs";

const here = import.meta.dirname;
const sites = SITES.map((s) => load(s, here)).filter(Boolean);
const missing = [];
const verdictOf = (row, site, family) => {
  if (family === "rawEquivalent") return rawVerdict(row, site);
  const table = family === "div" ? DIV : SECTIONING;
  const verdict = table[site]?.[row.file]?.[row.line];
  if (!verdict) missing.push(`${family} ${site}:${row.file}:${row.line} ${row.tag}`);
  return verdict ?? ["?", "unjudged"];
};

function measure(scan, gate, family, cls) {
  const element = family === "sectioning" ? "sectioningOnly" : family;
  const tally = { n: 0, R: 0, W: 0, O: 0, K: 0, files: new Set(), perSite: {} };
  for (const data of sites) {
    for (const row of run(data, scan, gate, element, cls)) {
      const [v] = verdictOf(row, data.site, family);
      tally.n += 1;
      tally[v] = (tally[v] ?? 0) + 1;
      tally.files.add(`${data.site}:${row.file}`);
      tally.perSite[data.site] = (tally.perSite[data.site] ?? 0) + 1;
    }
  }
  return tally;
}
const pct = (a, b) => (b === 0 ? "-" : `${Math.round((100 * a) / b)}%`);

const lines = ["scan,gate,family,classes,findings,files,R,W,O,K,strict,lenient,anyRafters,perSite"];
for (const scan of ["system", "format", "all"]) {
  for (const gate of Object.keys(GATES)) {
    for (const family of ["div", "sectioning", "rawEquivalent"]) {
      for (const cls of Object.keys(CLASSES)) {
        const t = measure(scan, gate, family, cls);
        lines.push([scan, gate, family, cls, t.n, t.files.size, t.R, t.W, t.O, t.K, pct(t.R, t.n), pct(t.R + t.W, t.n), pct(t.R + t.W + t.O, t.n), JSON.stringify(t.perSite).replaceAll(",", ";")].join(","));
      }
    }
  }
}
writeFileSync(join(here, "results/precision.csv"), `${lines.join("\n")}\n`);
console.log("unjudged rows:", [...new Set(missing)].length, [...new Set(missing)].slice(0, 20));

const show = (scan, gate, family, cls) => {
  const t = measure(scan, gate, family, cls);
  console.log(`${scan.padEnd(7)}${gate.padEnd(9)}${family.padEnd(14)}${cls.padEnd(20)} n=${String(t.n).padStart(3)} files=${String(t.files.size).padStart(3)} R=${String(t.R).padStart(2)} W=${String(t.W).padStart(2)} O=${String(t.O).padStart(2)} K=${String(t.K).padStart(3)} strict=${pct(t.R, t.n).padStart(4)} lenient=${pct(t.R + t.W, t.n).padStart(4)} anyRafters=${pct(t.R + t.W + t.O, t.n).padStart(4)}`);
};
console.log("\n-- divs by class filter and gate, format");
for (const gate of Object.keys(GATES)) for (const cls of ["every", "anyClass", "layout", "visual", "visualNoLayout", "noClass", "wrapsOne", "noClassOrWraps", "gridLike", "shellLike", "flexCol", "flexRow", "gridShellOrFlexCol", "textHolding"]) show("format", gate, "div", cls);
console.log("\n-- scan, div every");
for (const scan of ["system", "format", "all"]) for (const gate of Object.keys(GATES)) show(scan, gate, "div", "every");
console.log("\n-- sectioning");
for (const gate of Object.keys(GATES)) for (const cls of ["every", "noClass", "layout", "visual", "wrapsOne"]) show("format", gate, "sectioning", cls);
console.log("\n-- raw equivalents");
for (const gate of ["file", "every"]) show("format", gate, "rawEquivalent", "every");

console.log("\n-- raw by tag, every gate (R/W/K of n)");
const byTag = {};
for (const data of sites) for (const row of run(data, "format", "every", "rawEquivalent", "every")) {
  const [v] = rawVerdict(row, data.site);
  const entry = (byTag[row.tag] ??= { n: 0, R: 0, W: 0, O: 0, K: 0 });
  entry.n += 1;
  entry[v] += 1;
}
console.log(JSON.stringify(byTag));

console.log("\n-- verdict reasons for div, format, every gate");
const reasons = {};
for (const data of sites) for (const row of run(data, "format", "every", "div", "every")) {
  const [v, why] = verdictOf(row, data.site, "div");
  reasons[`${v} ${why}`] = (reasons[`${v} ${why}`] ?? 0) + 1;
}
for (const [why, n] of Object.entries(reasons).sort((a, b) => b[1] - a[1])) console.log(String(n).padStart(3), why);

console.log("\n-- the gate's blind side, divs in non-exempt files using no system or qualified component");
for (const data of sites) {
  const every = run(data, "format", "every", "div", "every");
  const gated = new Set(run(data, "format", "file", "div", "every").map((r) => r.id));
  const out = every.filter((r) => !gated.has(r.id));
  const files = {};
  for (const r of out) files[r.file] = (files[r.file] ?? 0) + 1;
  if (out.length) console.log(data.site, out.length, JSON.stringify(files));
}

console.log("\n-- any intrinsic element with a grid or container utility, outside exempt files");
for (const data of sites) {
  const found = run(data, "format", "every", "sectioning", "gridLike").length;
  const anyTag = data.rows.filter((r) => r.kind === "intrinsic" && !data.underSystem.get(r.file) && !(data.shapes.get(r.file) && QUALIFY.strict(data.shapes.get(r.file))) && tokens(r).some((t) => /^(grid|inline-grid)$/.test(t) || /^grid-cols-/.test(t)));
  console.log(data.site, "sectioning", found, "any tag", anyTag.map((r) => `${r.file}:${r.line} ${r.tag}`).join(" | "));
}

console.log("\n-- what format's exemption does: divs in qualified own files, and whether they use a system component");
for (const data of sites) {
  const own = data.meta.shapes.filter((s) => !data.underSystem.get(s.file) && QUALIFY.strict(s));
  if (!own.length) continue;
  const names = new Set(own.map((s) => s.file));
  const divs = data.rows.filter((r) => r.tag === "div" && names.has(r.file));
  const withSystem = new Set(data.rows.filter((r) => r.kind === "system" && names.has(r.file)).map((r) => r.file));
  console.log(data.site, `qualified ${own.length}, divs ${divs.length}, files using a system component ${withSystem.size}`, [...names].join(" "));
  const unq = data.meta.shapes.filter((s) => !data.underSystem.get(s.file) && !QUALIFY.strict(s) && data.rows.some((r) => r.kind === "own" && r.declaredIn === s.file));
  console.log(" own, used, not strict:", unq.map((s) => `${s.file} b=${s.behavior ? 1 : 0} c=${s.classes ? 1 : 0} tags=${s.bestBlock}`).join(" | "));
  const sys = data.meta.shapes.filter((s) => data.underSystem.get(s.file));
  console.log(` system files strict ${sys.filter(QUALIFY.strict).length} of ${sys.length}; missing behavior file ${sys.filter((s) => !s.behavior).length}; six tags ${sys.filter((s) => s.bestBlock === 6).length}`);
}

console.log("\n-- #158: existing rules by the kind of component they land on");
for (const data of sites) {
  const raw = {};
  for (const d of data.meta.raw) raw[`${d.severity} ${d.kind}`] = (raw[`${d.severity} ${d.kind}`] ?? 0) + 1;
  const fills = {};
  for (const r of data.rows) if (r.fillForm) {
    const q = r.kind === "own" ? (data.shapes.get(r.declaredIn) && QUALIFY.strict(data.shapes.get(r.declaredIn)) ? "own-qualified" : "own-unqualified") : r.kind;
    fills[q] = (fills[q] ?? 0) + 1;
  }
  const uses = {};
  for (const r of data.rows) if (r.kind === "system" || r.kind === "own") {
    const q = r.kind === "own" ? (data.shapes.get(r.declaredIn) && QUALIFY.strict(data.shapes.get(r.declaredIn)) ? "own-qualified" : "own-unqualified") : r.kind;
    uses[q] = (uses[q] ?? 0) + 1;
  }
  console.log(data.site, "raw", JSON.stringify(raw), "retiredFill", JSON.stringify(fills), "uses", JSON.stringify(uses), "baseline", data.meta.baseline.length);
}

console.log("\n-- timing");
for (const data of sites) {
  const { timing, rootFiles } = data.meta;
  console.log(data.site, rootFiles, JSON.stringify(timing), `elements ${data.rows.length}`);
}
