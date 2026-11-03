// Sanity over the facts: element kinds, shapes, and the program's div count against a grep count.
// The grep numbers below are line-based text matches, and are labelled so.
// Usage: node sanity.mjs <folder of site copies>
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const here = import.meta.dirname;
const copies = process.argv[2];
const six = 6;
for (const site of ["gitpress.app", "huttspawn", "rafters.studio", "runlegion.dev", "sean.silvius.me", "smugglr.dev"]) {
  const rowsPath = join(here, "results/facts", `${site}.rows.jsonl`);
  if (!existsSync(rowsPath)) continue;
  const rows = readFileSync(rowsPath, "utf8").trim().split("\n").map((l) => JSON.parse(l));
  const meta = JSON.parse(readFileSync(join(here, "results/facts", `${site}.meta.json`), "utf8"));
  const kinds = {};
  for (const r of rows) kinds[r.kind] = (kinds[r.kind] ?? 0) + 1;
  const div = rows.filter((r) => r.tag === "div");
  const divOutside = div.filter((r) => !meta.files.find((f) => f.file === r.file)?.underSystem);
  let grepOpen = 0;
  for (const f of meta.files) {
    const text = readFileSync(join(copies, site, f.file), "utf8");
    if (!f.underSystem) grepOpen += (text.match(/<div[\s>]/g) ?? []).length;
  }
  let mdxFiles = 0;
  let mdxDiv = 0;
  const walk = (d) => {
    for (const e of readdirSync(d, { withFileTypes: true })) {
      const p = join(d, e.name);
      if (e.isDirectory()) walk(p);
      else if (e.name.endsWith(".mdx")) {
        mdxFiles += 1;
        mdxDiv += (readFileSync(p, "utf8").match(/<div[\s>]/g) ?? []).length;
      }
    }
  };
  walk(join(copies, site, "src"));
  const own = rows.filter((r) => r.kind === "own");
  const ownFiles = new Set(own.map((r) => r.declaredIn));
  const sh = meta.shapes;
  const strict = sh.filter((s) => s.behavior && s.classes && s.bestBlock === six);
  console.log(`== ${site}: rootFiles ${meta.rootFiles}, elements ${rows.length} ${JSON.stringify(kinds)}`);
  console.log(`   div outside componentsPath: program ${divOutside.length}, grep(<div) ${grepOpen}; mdx files ${mdxFiles}, grep(<div) in mdx ${mdxDiv}`);
  console.log(`   own components used: ${ownFiles.size}; strict shapes among all files: ${strict.length}`);
  console.log(`   raw: ${JSON.stringify(meta.raw.map((r) => `${r.file}:${r.line} ${r.code} ${r.severity} ${r.kind} ${r.message.slice(0, 70)}`))}`);
  console.log(`   baseline: ${JSON.stringify(meta.baseline.map((r) => `${r.file}:${r.line} ${r.code} ${r.severity}`))}`);
}
