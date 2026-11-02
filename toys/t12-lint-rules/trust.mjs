// node trust.mjs <dist/index.mjs> <fixture> <trusted|untrusted>: prints whether project code ran, and diagnostics.
import { createRequire } from "node:module";
import { existsSync, rmSync } from "node:fs";
import { join } from "node:path";
const [dist, root, mode] = process.argv.slice(2);
const marker = join(process.cwd(), `marker-${mode}.txt`);
rmSync(marker, { force: true });
process.env.T12_MARKER = marker;
const logs = [];
for (const k of ["log", "info", "error", "warn"]) { const o = console[k]; console[k] = (...a) => { logs.push(`${k}: ${a.join(" ").slice(0, 110)}`); }; console[`_${k}`] = o; }
if (mode === "untrusted") {
  const req = createRequire(join(dist, ".."));
  const ls = createRequire(req.resolve("@astrojs/language-server/package.json"));
  ls("./dist/importPackage.js").setIsTrusted(false);
}
const { openProject } = await import(dist);
const p = await openProject(root);
const out = {};
for (const f of ["src/pages/uses.astro", "src/pages/imports-svelte.astro", "src/components/Foo.svelte"]) {
  try { out[f] = (await p.diagnostics(f)).map((d) => `${d.line}:${d.column} ${d.severity} ${d.code}`); } catch (e) { out[f] = `threw ${e.code}`; }
}
p.close();
console._log(JSON.stringify({ mode, projectCodeRan: existsSync(marker), logs, out }, null, 1));
rmSync(marker, { force: true });
process.exit(0);
