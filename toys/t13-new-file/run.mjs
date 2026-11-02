// node run.mjs <dist/index.mjs> <project> <relative new file> <componentFolder>
// Answers ts.sys for a file that is not on disk before the project opens, with TypeScript's own
// matcher deciding whether the tsconfig's include would take it.
import { createRequire } from "node:module";
import { existsSync } from "node:fs";
import { join } from "node:path";
const [dist, root, rel, folder] = process.argv.slice(2);
const ts = createRequire(dist)("typescript");
const { openProject, lint } = await import(dist);
const abs = join(root, rel);
const text = `---\nimport Button from '@/components/ui/button.astro';\n---\n<Button tone="x">a</Button>\n`;
const original = { readFile: ts.sys.readFile, fileExists: ts.sys.fileExists, getModifiedTime: ts.sys.getModifiedTime, readDirectory: ts.sys.readDirectory };
const walkTo = (target) => (dir) => {
  const prefix = dir.endsWith("/") ? dir : `${dir}/`;
  if (!target.startsWith(prefix)) return { files: [], directories: [] };
  const rest = target.slice(prefix.length).split("/");
  return rest.length === 1 ? { files: [rest[0]], directories: [] } : { files: [], directories: [rest[0]] };
};
const matches = (target, a) => ts.matchFiles(a[0], a[1], a[2], a[3], true, root, a[4], walkTo(target), (x) => x).length > 0;
let lastArgs;
ts.sys.readFile = (f, e) => (f === abs ? text : original.readFile.call(ts.sys, f, e));
ts.sys.fileExists = (f) => f === abs || original.fileExists.call(ts.sys, f);
ts.sys.getModifiedTime = (f) => (f === abs ? new Date(1) : original.getModifiedTime.call(ts.sys, f));
ts.sys.readDirectory = (...a) => { lastArgs = a; const found = original.readDirectory.call(ts.sys, ...a); return matches(abs, a) && !found.includes(abs) ? [...found, abs] : found; };
const t0 = performance.now();
const p = await openProject(root);
const opened = +(performance.now() - t0).toFixed(0);
const added = p.checker.getRootFileNames().includes(abs);
let diagnostics, linted, error;
try {
  diagnostics = (await p.diagnostics(rel)).map((d) => `${d.line}:${d.column} ${d.code}`);
  linted = (await lint(p, { componentFolders: [folder], files: [rel] })).map((d) => `${d.line}:${d.column} ${d.code} help=${d.help}`);
} catch (e) { error = `${e.code ?? ""} ${e.message}`; }
const roots = p.checker.getRootFileNames().length;
const out = { added, roots, diagnostics, linted, error, opened, onDisk: existsSync(abs), fitsPage: matches(abs, lastArgs), fitsOutside: matches(join(root, "elsewhere/x.astro"), lastArgs) };
Object.assign(ts.sys, original);
p.close();
console.log(JSON.stringify(out, null, 1));
process.exit(0);
