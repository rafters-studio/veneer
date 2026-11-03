// Every combination over every site: findings and files touched. Writes results/combos.csv and
// results/qualify.json, and prints the summary. Usage: node combos.mjs
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { CLASSES, ELEMENTS, GATES, QUALIFY, SCANS, SITES, inScan, load, run } from "./lib.mjs";

const here = import.meta.dirname;
const sites = SITES.map((s) => load(s, here)).filter(Boolean);

const lines = ["scan,gate,elements,classes,site,findings,files"];
for (const scan of Object.keys(SCANS)) {
  for (const gate of Object.keys(GATES)) {
    for (const element of Object.keys(ELEMENTS)) {
      for (const cls of Object.keys(CLASSES)) {
        let total = 0;
        const totalFiles = new Set();
        for (const data of sites) {
          const found = run(data, scan, gate, element, cls);
          const files = new Set(found.map((r) => r.file));
          lines.push([scan, gate, element, cls, data.site, found.length, files.size].join(","));
          total += found.length;
          for (const f of files) totalFiles.add(`${data.site}:${f}`);
        }
        lines.push([scan, gate, element, cls, "ALL", total, totalFiles.size].join(","));
      }
    }
  }
}
writeFileSync(join(here, "results/combos.csv"), `${lines.join("\n")}\n`);
console.log(`combos.csv: ${lines.length - 1} rows`);

// ---- What qualifies, per definition, split by where the component lives.
const qualify = {};
for (const data of sites) {
  const used = new Set(data.rows.filter((r) => r.kind === "own" || r.kind === "system").map((r) => r.declaredIn));
  const entry = {};
  for (const [name, test] of Object.entries(QUALIFY)) {
    const matched = data.meta.shapes.filter(test);
    const own = matched.filter((s) => !data.underSystem.get(s.file) && !s.file.includes("components/ui/"));
    const system = matched.filter((s) => data.underSystem.get(s.file) || s.file.includes("components/ui/"));
    entry[name] = {
      ownFiles: own.length,
      ownUsed: own.filter((s) => used.has(s.file)).length,
      systemFiles: system.length,
      ownNames: own.map((s) => s.file),
    };
  }
  qualify[data.site] = entry;
  const totalShapes = data.meta.shapes;
  const sysUsed = data.rows.filter((r) => r.kind === "system");
  console.log(`\n${data.site}: root files ${data.meta.rootFiles}, shapes ${totalShapes.length}, system uses ${sysUsed.length}`);
  for (const [name, e] of Object.entries(entry)) {
    console.log(`  ${name.padEnd(16)} own ${String(e.ownFiles).padStart(3)} (used ${String(e.ownUsed).padStart(3)}) system ${String(e.systemFiles).padStart(3)}`);
  }
}
writeFileSync(join(here, "results/qualify.json"), `${JSON.stringify(qualify, null, 1)}\n`);

// ---- The main table, for the eye: total per scan and gate.
const get = (scan, gate, el, cls, site = "ALL") => lines.find((l) => l.startsWith(`${scan},${gate},${el},${cls},${site},`)).split(",").slice(5);
console.log("\nscan x gate, div, every class (findings/files)");
for (const scan of ["system", "format", "all"]) {
  console.log(`  ${scan.padEnd(8)}`, Object.keys(GATES).map((g) => `${g} ${get(scan, g, "div", "every").join("/")}`).join("  "));
}
console.log("\nformat x class filter, div, (findings/files) by gate");
for (const cls of Object.keys(CLASSES)) {
  console.log(`  ${cls.padEnd(20)}`, Object.keys(GATES).map((g) => `${g} ${get("format", g, "div", cls).join("/")}`).join("  "));
}
console.log("\nformat, file gate, per site: div every / sectioningOnly every / rawEquivalent every");
for (const data of sites) {
  console.log(`  ${data.site.padEnd(18)}`, ["div", "sectioningOnly", "rawEquivalent"].map((e) => get("format", "file", e, "every", data.site).join("/")).join("   "));
}

// ---- Raw elements by tag, format gate and every-file gate.
console.log("\nraw elements by tag (format, file gate / every gate), all sites");
const tagCount = {};
for (const data of sites) {
  for (const r of run(data, "format", "file", "sectioning", "every").concat(run(data, "format", "file", "rawEquivalent", "every"))) {
    tagCount[r.tag] = tagCount[r.tag] ?? [0, 0];
    tagCount[r.tag][0] += 1;
  }
  for (const r of run(data, "format", "every", "sectioning", "every").concat(run(data, "format", "every", "rawEquivalent", "every"))) {
    tagCount[r.tag] = tagCount[r.tag] ?? [0, 0];
    tagCount[r.tag][1] += 1;
  }
}
console.log(Object.entries(tagCount).sort((a, b) => b[1][1] - a[1][1]).map(([t, [a, b]]) => `${t} ${a}/${b}`).join(", "));

// ---- Coverage: what share of files with a div are gated in by each scan.
console.log("\nfiles holding a div (non exempt) reached by the file gate, per scan");
for (const data of sites) {
  const parts = ["system", "format", "all"].map((scan) => {
    const every = new Set(run(data, scan, "every", "div", "every").map((r) => r.file));
    const gated = new Set(run(data, scan, "file", "div", "every").map((r) => r.file));
    return `${scan} ${gated.size}/${every.size}`;
  });
  console.log(`  ${data.site.padEnd(18)} ${parts.join("  ")}`);
}
void inScan;
