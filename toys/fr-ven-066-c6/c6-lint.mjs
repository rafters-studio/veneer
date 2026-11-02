// FR-VEN-066-c6: node c6-lint.mjs <path to dist/index.mjs> <path to sites/rafters.studio>
import { readFileSync } from "node:fs";
import path from "node:path";

const [dist, site] = process.argv.slice(2);
const { openProject, lint, report } = await import(dist);
const project = await openProject(site);

const PAGE = "src/pages/kelex/index.astro";
const original = readFileSync(path.join(site, PAGE), "utf8");
const lines = original.split("\n");
const importAt = lines.findIndex((line) => line.includes("typography-p.astro"));
lines.splice(importAt + 1, 0, "import Button from '@/components/ui/button.astro';");
const frontmatterEnd = lines.indexOf("---", 1);
lines.splice(frontmatterEnd, 0, "const planted: number = 'x';");
const h1At = lines.findIndex((line) => line.trimStart().startsWith("<H1 size="));
lines.splice(
  h1At,
  0,
  '      <Button tone="x">Undeclared</Button>',
  '      <Button variant="nope">Bad value</Button>',
);
project.setUnsaved(PAGE, lines.join("\n"));

// Its own time is lint's whole time less what project.diagnostics takes.
const { diagnostics } = project;
let inDiagnostics = 0;
const timed = {
  ...project,
  program: () => project.program(),
  async diagnostics(file) {
    const start = performance.now();
    const found = await diagnostics.call(project, file);
    inDiagnostics += performance.now() - start;
    return found;
  },
};

const runs = [];
let found = [];
const all = [];
for (let run = 0; run < 5; run += 1) {
  inDiagnostics = 0;
  const start = performance.now();
  found = await lint(timed, { componentFolders: ["src/components/ui"] });
  const whole = performance.now() - start;
  runs.push({ wholeMs: +whole.toFixed(1), ownMs: +(whole - inDiagnostics).toFixed(2) });
}
for (const file of project.checker.getRootFileNames()) {
  if (!/\.(astro|tsx)$/.test(file)) continue;
  const relative = path.relative(project.root, file).split(path.sep).join("/");
  for (const d of await project.diagnostics(relative)) {
    all.push(`${d.file}:${d.line}:${d.column} ${d.severity} ${d.code}`);
  }
}
const unchanged = readFileSync(path.join(site, PAGE), "utf8") === original;
const planted = lines
  .map((text, index) => [index + 1, text])
  .filter(([, text]) => text.includes("<Button ") || text.includes("const planted"));
console.log(
  JSON.stringify(
    {
      planted,
      runs,
      everyDiagnostic: all,
      diskUnchanged: unchanged,
      found,
    },
    null,
    1,
  ),
);
console.log(report(found));
project.close();
process.exit(0);
