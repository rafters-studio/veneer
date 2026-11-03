// Three small questions the tables leave open. Reads only: the recorded facts and the site copies.
// The "line grep" numbers are per-line text matches, kept to show what that method answers.
// Usage: node extra.mjs <copies>
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { SCANS, SITES, load, run } from "./lib.mjs";

const here = import.meta.dirname;
const copies = process.argv[2];
const sites = SITES.map((s) => load(s, here)).filter(Boolean);

console.log("-- div findings per qualification definition (file gate / every gate), and own components in scan");
for (const scan of Object.keys(SCANS)) {
  let file = 0;
  let every = 0;
  let uses = 0;
  for (const data of sites) {
    file += run(data, scan, "file", "div", "every").length;
    every += run(data, scan, "every", "div", "every").length;
    uses += data.rows.filter((r) => r.kind === "own" && data.shapes.get(r.declaredIn) && SCANS[scan].own(data.shapes.get(r.declaredIn))).length;
  }
  console.log(`  ${scan.padEnd(24)} file ${String(file).padStart(3)}  every ${String(every).padStart(3)}  own uses in scan ${uses}`);
}

console.log("\n-- layout divs: program against a per-line grep, non-exempt files");
const LINE = /<div\b[^>]*\bclass(Name)?=["'{][^>]*\b(flex|inline-flex|grid|inline-grid|container|mx-auto|gap-|justify-|items-)/;
for (const data of sites) {
  const program = run(data, "format", "every", "div", "layout");
  const programAt = new Set(program.map((r) => `${r.file}:${r.line}`));
  const every = new Set(run(data, "format", "every", "div", "every").map((r) => r.file));
  const files = data.meta.files.filter((f) => every.has(f.file));
  const grepAt = new Set();
  for (const f of files) {
    readFileSync(join(copies, data.site, f.file), "utf8").split("\n").forEach((line, i) => {
      if (LINE.test(line)) grepAt.add(`${f.file}:${i + 1}`);
    });
  }
  const missed = [...programAt].filter((k) => !grepAt.has(k));
  const extra = [...grepAt].filter((k) => !programAt.has(k));
  console.log(`  ${data.site.padEnd(18)} program ${program.length}  grep ${grepAt.size}  grep missed ${JSON.stringify(missed)}  grep extra ${JSON.stringify(extra)}`);
}

console.log("\n-- local bindings rendered as tags, which #158's all could count as declared in source");
for (const data of sites) {
  const dynamic = data.rows.filter((r) => r.kind === "dynamic");
  if (dynamic.length) {
    const names = {};
    for (const r of dynamic) names[r.tag] = (names[r.tag] ?? 0) + 1;
    console.log(`  ${data.site}: ${dynamic.length} ${JSON.stringify(names)}`);
  }
}

console.log("\n-- near misses on huttspawn: uses of own components that do not qualify");
const hutt = sites.find((d) => d.site === "huttspawn");
if (hutt) {
  const counts = {};
  for (const r of hutt.rows) if (r.kind === "own" && !SCANS.format.own(hutt.shapes.get(r.declaredIn))) counts[r.declaredIn] = (counts[r.declaredIn] ?? 0) + 1;
  console.log(" ", JSON.stringify(counts));
}
