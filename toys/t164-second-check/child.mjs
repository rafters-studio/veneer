// node --import ./modes.mjs child.mjs '<json steps>': one process. Runs steps in order; prints one JSON line per check call.
import {
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  statSync,
  utimesSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";
import { check } from "@rafters/veneer";

const steps = JSON.parse(process.argv[2]);
for (const step of steps) {
  if (step.write) {
    mkdirSync(path.dirname(step.write), { recursive: true });
    const kept = step.keepTime ? statSync(step.write) : undefined;
    writeFileSync(step.write, step.text);
    if (kept) utimesSync(step.write, kept.atime, kept.mtime);
  } else if (step.remove) {
    rmSync(step.remove, { force: true });
  } else {
    const started = performance.now();
    const result = await check(step.root, step.options);
    const docs = path.join(step.root, ".rafters/veneer/docs.jsonl");
    console.log(
      JSON.stringify({
        ms: Math.round(performance.now() - started),
        ok: result.ok,
        report: result.ok ? result.report : result.errors,
        diagnostics: result.ok ? result.diagnostics : undefined,
        notices: result.ok ? result.notices : undefined,
        docs: step.options?.save && existsSync(docs) ? readFileSync(docs, "utf8") : undefined,
      }),
    );
  }
}
