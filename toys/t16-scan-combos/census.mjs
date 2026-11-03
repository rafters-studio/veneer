// Prints every element of a family that is not in an exempt file, one line each, with what a human
// needs to judge it: classes, parent chain, children, and the opening source lines.
// Usage: node census.mjs <copies> <div|sectioningOnly|rawEquivalent|ownExemptDiv> [--sites a,b]
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { CLASSES, ELEMENTS, QUALIFY, SITES, load } from "./lib.mjs";

const here = import.meta.dirname;
const [copies, family] = process.argv.slice(2);
const wanted = (r, data) => {
  const exempt = data.underSystem.get(r.file) || (data.shapes.get(r.file) && QUALIFY.strict(data.shapes.get(r.file)));
  if (family === "ownExemptDiv") return r.tag === "div" && !data.underSystem.get(r.file) && exempt;
  return !exempt && ELEMENTS[family](r);
};
const labels = (r) => Object.keys(CLASSES).filter((c) => !["every", "anyClass", "visualNoLayout", "noClassOrWraps", "gridOrShell", "gridShellOrFlexCol"].includes(c) && CLASSES[c](r)).join(",");
for (const site of SITES) {
  const data = load(site, here);
  if (!data) continue;
  const texts = new Map();
  for (const r of data.rows) {
    if (r.kind !== "intrinsic" || !wanted(r, data)) continue;
    if (!texts.has(r.file)) texts.set(r.file, readFileSync(join(copies, site, r.file), "utf8").split("\n"));
    const lines = texts.get(r.file);
    const usesAny = data.rows.some((o) => o.file === r.file && (o.kind === "system" || (o.kind === "own" && data.shapes.get(o.declaredIn) && QUALIFY.strict(data.shapes.get(o.declaredIn)))));
    const usesOwn = data.rows.some((o) => o.file === r.file && o.kind === "own");
    const chain = r.ancestors.slice(0, 4).map((a) => a.tag + (a.classes ? `.${a.classes.split(/\s+/).slice(0, 3).join(".")}` : "")).join(" < ") || "ROOT";
    const kids = r.children.slice(0, 8).map((c) => (c.kind === "intrinsic" ? c.tag : `${c.tag}*${c.kind[0]}`)).join(" ");
    const desc = r.descendants.filter((d) => d.kind === "system").length;
    console.log(`#${site}:${r.file}:${r.line}:${r.column} ${r.tag} gate=${usesAny ? "S" : usesOwn ? "o" : "-"} [${labels(r)}] attrs=${r.attrs.filter((a) => a !== "class").join("|") || "-"}${r.hasSpread ? " SPREAD" : ""} txt=${r.text ? 1 : 0} expr=${r.expressions} map=${r.mapped ? 1 : 0} sysBelow=${desc}`);
    console.log(`  up: ${chain}`);
    console.log(`  kids(${r.children.length}): ${kids || "-"}`);
    console.log(`  src: ${lines.slice(r.line - 1, r.line + 1).map((l) => l.trim()).join(" // ").slice(0, 170)}`);
  }
}
