// The combination vocabulary, shared by combos.mjs and census.mjs. It only queries the rows facts.ts
// wrote from the program; nothing here reads source text.
import { readFileSync, existsSync } from "node:fs";
import { join } from "node:path";

export const SITES = ["gitpress.app", "huttspawn", "rafters.studio", "runlegion.dev", "sean.silvius.me", "smugglr.dev"];

// The utility after the last colon outside brackets, as lint.ts's utilityOf.
export function utilityOf(className) {
  let depth = 0;
  let start = 0;
  for (let at = 0; at < className.length; at += 1) {
    const c = className[at];
    if (c === "[" || c === "(") depth += 1;
    else if (c === "]" || c === ")") depth -= 1;
    else if (c === ":" && depth === 0) start = at + 1;
  }
  return className.slice(start);
}
export const tokens = (row) => row.classes.split(/\s+/).filter(Boolean).map(utilityOf);

const TEXT_NOT_COLOUR = /^text-(xs|sm|base|lg|[0-9]*xl|left|right|center|justify|start|end|wrap|nowrap|balance|pretty|ellipsis|clip)$/;
const isLayout = (t) => /^(flex|inline-flex|grid|inline-grid|container|mx-auto)$/.test(t) || /^(gap|justify|items)-/.test(t);
const isVisual = (t) => /^(bg|border|rounded|shadow|ring)(-|$)/.test(t) || (t.startsWith("text-") && !TEXT_NOT_COLOUR.test(t));
const hasClass = (r) => r.classes !== "" || r.classDynamic;
const noClassAttr = (r) => !hasClass(r) && !r.hasSpread;

export function load(site, here) {
  const dir = join(here, "results/facts");
  const rowsPath = join(dir, `${site}.rows.jsonl`);
  if (!existsSync(rowsPath)) return undefined;
  const rows = readFileSync(rowsPath, "utf8").trim().split("\n").map((l) => JSON.parse(l));
  const meta = JSON.parse(readFileSync(join(dir, `${site}.meta.json`), "utf8"));
  const byId = new Map(rows.map((r) => [r.id, r]));
  for (const r of rows) r.children = [];
  for (const r of rows) if (r.parent !== null) byId.get(r.parent).children.push(r);
  for (const r of rows) {
    r.ancestors = [];
    for (let p = r.parent; p !== null; p = byId.get(p).parent) r.ancestors.push(byId.get(p));
    r.descendants = [];
  }
  for (const r of rows) for (const a of r.ancestors) a.descendants.push(r);
  const shapes = new Map(meta.shapes.map((s) => [s.file, s]));
  const underSystem = new Map(meta.files.map((f) => [f.file, f.underSystem]));
  return { site, rows, meta, shapes, underSystem };
}

// Which components qualify as format. strict is #158's criterion one.
export const QUALIFY = {
  strict: (s) => s.behavior && s.classes && s.bestBlock === 6,
  strictUnion: (s) => s.behavior && s.classes && s.union.length === 6,
  filesAnyTag: (s) => s.behavior && s.classes && s.union.length >= 1,
  filesOnly: (s) => s.behavior && s.classes,
  classesAnyTag: (s) => s.classes && s.union.length >= 1,
  classesOnly: (s) => s.classes,
  sixTagsNoFiles: (s) => s.bestBlock === 6,
  anyTag: (s) => s.union.length >= 1,
};

export const SCANS = {
  system: { own: () => false, q: "strict" },
  format: { own: QUALIFY.strict, q: "strict" },
  "format:strictUnion": { own: QUALIFY.strictUnion, q: "strictUnion" },
  "format:filesAnyTag": { own: QUALIFY.filesAnyTag, q: "filesAnyTag" },
  "format:filesOnly": { own: QUALIFY.filesOnly, q: "filesOnly" },
  "format:classesAnyTag": { own: QUALIFY.classesAnyTag, q: "classesAnyTag" },
  "format:classesOnly": { own: QUALIFY.classesOnly, q: "classesOnly" },
  "format:sixTagsNoFiles": { own: QUALIFY.sixTagsNoFiles, q: "sixTagsNoFiles" },
  "format:anyTag": { own: QUALIFY.anyTag, q: "anyTag" },
  all: { own: () => true, q: "strict" },
};

export function inScan(row, scan, shapes) {
  if (row.kind === "system") return true;
  if (row.kind !== "own") return false;
  const shape = shapes.get(row.declaredIn);
  return Boolean(shape && scan.own(shape));
}

export function isExempt(file, scan, data) {
  if (data.underSystem.get(file)) return true;
  const shape = data.shapes.get(file);
  return Boolean(shape && QUALIFY[scan.q](shape));
}

export const GATES = {
  // The file holds at least one use in scan. #159 as written.
  file: (row, inScanRow, fileHasUse) => fileHasUse.has(row.file),
  // A use in scan is the row's nearest element child.
  parent: (row, inScanRow) => row.children.some(inScanRow),
  // A use in scan sits anywhere below the row.
  ancestor: (row, inScanRow) => row.descendants.some(inScanRow),
  // Any file that is not exempt.
  every: () => true,
};

const SECTIONING = new Set(["div", "section", "main", "article", "aside", "header", "footer", "nav"]);
const RAW_EQUIVALENT = new Set(["button", "input", "select", "textarea", "h1", "h2", "h3", "h4", "h5", "h6", "p", "a", "table", "img"]);
export const ELEMENTS = {
  div: (r) => r.tag === "div",
  sectioning: (r) => SECTIONING.has(r.tag),
  sectioningOnly: (r) => SECTIONING.has(r.tag) && r.tag !== "div",
  rawEquivalent: (r) => RAW_EQUIVALENT.has(r.tag),
};

const has = (r, test) => tokens(r).some(test);
export const CLASSES = {
  every: () => true,
  anyClass: hasClass,
  layout: (r) => has(r, isLayout),
  visual: (r) => has(r, isVisual),
  visualNoLayout: (r) => has(r, isVisual) && !has(r, isLayout),
  noClass: noClassAttr,
  wrapsOne: (r) => r.children.length === 1 && !r.text && r.expressions === 0,
  wrapsOneNoClass: (r) => noClassAttr(r) && r.children.length === 1 && !r.text && r.expressions === 0,
  noClassOrWraps: (r) => noClassAttr(r) || (r.children.length === 1 && !r.text && r.expressions === 0),
  // Splits of layout, added because Container and Grid replace some of these and not others.
  gridLike: (r) => has(r, (t) => /^(grid|inline-grid)$/.test(t) || /^grid-cols-/.test(t)),
  shellLike: (r) => has(r, (t) => /^(container|mx-auto)$/.test(t) || /^max-w-/.test(t)),
  flexCol: (r) => has(r, (t) => /^(flex|inline-flex)$/.test(t)) && has(r, (t) => t === "flex-col"),
  flexRow: (r) => has(r, (t) => /^(flex|inline-flex)$/.test(t)) && !has(r, (t) => t === "flex-col"),
  // Added after reading the census: a div that directly holds text.
  textHolding: (r) => r.text || (r.expressions > 0 && r.children.length === 0),
  // The rule as recommended. Written after reading this corpus, so its precision here is in-sample.
  // Hint: an unprefixed flex, flex-col and gap, or an unprefixed max-w size. max-w-none and max-w-prose
  // are excluded: the first is no width, and the second is a text measure, which Container has no size for.
  recommendedWarn: (r) => has(r, (t) => /^(grid|inline-grid|container|mx-auto)$/.test(t) || /^grid-cols-/.test(t)),
  recommendedHint: (r) => {
    const written = r.classes.split(/\s+/).filter(Boolean);
    const stack = written.includes("flex") && written.includes("flex-col") && written.some((c) => c.startsWith("gap-"));
    return stack || written.some((c) => /^max-w-/.test(c) && c !== "max-w-none" && c !== "max-w-prose");
  },
  gridOrShell: (r) => CLASSES.gridLike(r) || CLASSES.shellLike(r),
  gridShellOrFlexCol: (r) => CLASSES.gridLike(r) || CLASSES.shellLike(r) || CLASSES.flexCol(r),
};

// Runs one combination over one site.
export function run(data, scanName, gateName, elementName, className) {
  const scan = SCANS[scanName];
  const inScanRow = (r) => inScan(r, scan, data.shapes);
  const fileHasUse = new Set(data.rows.filter(inScanRow).map((r) => r.file));
  const gate = GATES[gateName];
  const element = ELEMENTS[elementName];
  const classes = CLASSES[className];
  return data.rows.filter(
    (r) => r.kind === "intrinsic" && element(r) && classes(r) && !isExempt(r.file, scan, data) && gate(r, inScanRow, fileHasUse),
  );
}
