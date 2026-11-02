// node worker.mjs <dist/index.mjs> <project root> <component folder>. JSON lines on stdin and stdout.
import { createInterface } from "node:readline";
const [dist, root, folder] = process.argv.slice(2);
const { openProject, lint } = await import(dist);
const write = (message) => process.stdout.write(`${JSON.stringify(message)}\n`);
for (const k of ["log", "info", "warn"]) console[k] = () => {};
let project = await openProject(root);
const unsaved = new Map();
const isRoot = (abs) => project.checker.getRootFileNames().includes(abs);
write({ ready: true });
for await (const line of createInterface({ input: process.stdin })) {
  const { seq, file, text } = JSON.parse(line);
  const t0 = performance.now();
  let reopened = false;
  try {
    unsaved.set(file, text);
    if (!isRoot(`${project.root}/${file}`)) {
      project.close();
      project = await openProject(root, { unsaved: Object.fromEntries(unsaved) });
      reopened = true;
    }
    if (!isRoot(`${project.root}/${file}`)) {
      write({ seq, file, outside: true, workerMs: +(performance.now() - t0).toFixed(1), diagnostics: [] });
      continue;
    }
    project.setUnsaved(file, text);
    const diagnostics = await lint(project, { componentFolders: [folder], files: [file] });
    write({ seq, file, reopened, workerMs: +(performance.now() - t0).toFixed(1), diagnostics });
  } catch (error) {
    write({ seq, file, error: String(error), workerMs: +(performance.now() - t0).toFixed(1), diagnostics: [] });
  }
}
process.exit(0);
