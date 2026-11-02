// FR-VEN-064-c4: node measure.mjs <path to dist/index.mjs> <path to sites/rafters.studio>
import { readFileSync } from "node:fs";
import path from "node:path";

const [dist, site] = process.argv.slice(2);
const median = (values) => [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)];

const loadStart = performance.now();
const { openProject, ProjectError } = await import(dist);
const moduleLoadMs = performance.now() - loadStart;

const coldStart = performance.now();
const project = await openProject(site);
const files = project.checker
  .getRootFileNames()
  .map((file) => path.relative(project.root, file).split(path.sep).join("/"));
const found = [];
for (const file of files) found.push(...(await project.diagnostics(file)));
const coldMs = performance.now() - coldStart;

const counts = {};
for (const { severity } of found) counts[severity] = (counts[severity] ?? 0) + 1;

const PAGE = "src/pages/kelex/index.astro";
const original = readFileSync(path.join(site, PAGE), "utf8");
const lines = original.split("\n");
const importAt = lines.findIndex((line) => line.includes("typography-p.astro"));
lines.splice(importAt + 1, 0, "import Button from '@/components/ui/button.astro';");
const h1At = lines.findIndex((line) => line.trimStart().startsWith("<H1 size="));
lines.splice(
  h1At,
  0,
  '      <Button tone="x">Undeclared</Button>',
  '      <Button variant="nope">Bad value</Button>',
);
const planted = lines.join("\n");

const warmMs = [];
let warm = [];
for (let run = 0; run < 5; run += 1) {
  const start = performance.now();
  project.setUnsaved(PAGE, planted);
  warm = await project.diagnostics(PAGE);
  warmMs.push(performance.now() - start);
  project.clearUnsaved(PAGE);
  await project.diagnostics(PAGE);
}
const unchanged = readFileSync(path.join(site, PAGE), "utf8") === original;

// The rejections no scenario names.
const codes = {};
for (const bad of ["/abs.astro", "a\\b.astro", "./src/x.astro", "src/../x.astro", "../x.astro"]) {
  await project.diagnostics(bad).catch((error) => {
    codes[bad] = error instanceof ProjectError ? error.code : String(error);
  });
}
await project.diagnostics("src/nope.astro").catch((error) => {
  codes["src/nope.astro"] = error.code;
});
try {
  project.setUnsaved("/abs.astro", "");
} catch (error) {
  codes["setUnsaved /abs.astro"] = error.code;
}
project.close();
try {
  project.close();
} catch (error) {
  codes["second close"] = error.code;
}
await openProject("/nonexistent-veneer-root").catch((error) => {
  codes["no tsconfig"] = `${error.code}: ${error.message}`;
});

console.log(
  JSON.stringify(
    {
      files: files.length,
      counts,
      diagnostics: found.map((d) => `${d.file} ${d.line}:${d.column} ${d.severity} ${d.code}`),
      moduleLoadMs: Math.round(moduleLoadMs),
      coldMs: Math.round(coldMs),
      warmMs: warmMs.map(Math.round),
      warmMedianMs: Math.round(median(warmMs)),
      warm: warm.map((d) => `${d.line}:${d.column}-${d.endLine}:${d.endColumn} ${d.code}`),
      diskUnchanged: unchanged,
      codes,
    },
    null,
    1,
  ),
);
