// node worker-run.mjs <installed copy of the read fixture> [mode ...]: P4. Drives the real dist/worker.mjs over its JSON lines.
import { spawn } from "node:child_process";
import { cpSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { createInterface } from "node:readline";
import { fileURLToPath } from "node:url";

const here = fileURLToPath(new URL(".", import.meta.url));
const pkg = path.join(here, "../../packages/veneer");
const fixture = path.join(pkg, "test/fixtures/read");
const [R, ...modes] = process.argv.slice(2);
const PAGE = "src/pages/index.astro";
const E = readFileSync(path.join(fixture, PAGE), "utf8");
const B = `\n<div class="p-[3px]"></div>\n`;
const N = "src/pages/n.astro";
const N2 = "src/pages/n2.astro";
const USES = "src/pages/uses.astro";
const BOX = "src/box.ts";
const USES_TEXT = `---\nimport { box } from "../box.ts";\n---\n<div class={box}></div>\n`;
const CLEAN = "---\n---\n<div class=\"p-4\"></div>\n";
const type = (file, text) => ({ file, text });
const close = (file) => ({ close: file });
const disk = (file, text) => ({ disk: file, text });
const withBox = [disk(BOX, `export const box = "p-4";\n`), disk(USES, USES_TEXT)];
const BOX_TSX = "src/boxx.tsx";
const USES_TSX = `---\nimport { box } from "../boxx.tsx";\n---\n<div class={box}></div>\n`;
const withTsxBox = [disk(BOX_TSX, `export const box = "p-4";\n`), disk(USES, USES_TSX)];

// before: disk writes before the worker starts. steps: what the editor does. fresh: the last answer's state, to a new worker.
const SEQUENCES = {
  "W1 plain typing in one root file, no reopen": {
    steps: [type(PAGE, E), type(PAGE, `${E}${B}`), type(PAGE, E), type(PAGE, `${E}${B}`)],
    fresh: [type(PAGE, `${E}${B}`)],
  },
  "W2 new file typed once, closed unsaved, opened again with a bracket": {
    steps: [type(N, CLEAN), close(N), type(N, `---\n---\n${B}`)],
    fresh: [type(N, `---\n---\n${B}`)],
  },
  "W2b as W2, typed twice before closing (version moves off 0)": {
    steps: [type(N, CLEAN), type(N, `${CLEAN}\n`), close(N), type(N, `---\n---\n${B}`)],
    fresh: [type(N, `---\n---\n${B}`)],
  },
  "W3 a disk edit of an imported const, then typing, no reopen": {
    before: withBox,
    steps: [type(USES, USES_TEXT), disk(BOX, `export const box = "p-[3px]";\n`), type(USES, `${USES_TEXT}\n`)],
    fresh: [type(USES, `${USES_TEXT}\n`)],
  },
  "W3b the same disk edit, then a new file (reopen), then typing": {
    before: withBox,
    steps: [
      type(USES, USES_TEXT),
      disk(BOX, `export const box = "p-[3px]";\n`),
      type(N, CLEAN),
      type(USES, `${USES_TEXT}\n`),
    ],
    fresh: [type(N, CLEAN), type(USES, `${USES_TEXT}\n`)],
  },
  "W4 an unsaved bracket in an imported const, a reopen, the edit discarded, a reopen": {
    before: withBox,
    steps: [
      type(BOX, `export const box = "p-[3px]";\n`),
      type(N, CLEAN),
      close(BOX),
      type(N2, CLEAN),
      type(USES, `${USES_TEXT}\n`),
    ],
    fresh: [type(USES, `${USES_TEXT}\n`)],
  },
  "W4b as W4, with the program rebuilt on the unsaved bracket before the first reopen": {
    before: withBox,
    steps: [
      type(BOX, `export const box = "p-[3px]";\n`),
      type(USES, USES_TEXT),
      type(N, CLEAN),
      close(BOX),
      type(N2, CLEAN),
      type(USES, `${USES_TEXT}\n`),
    ],
    fresh: [type(USES, `${USES_TEXT}\n`)],
  },
  // veneer-lsp sends only .astro and .tsx (main.rs:52), so W4b and W5 cannot reach the worker through it as a .ts buffer.
  "W4c as W4 with a .tsx const, which the server does send": {
    before: withTsxBox,
    steps: [
      type(BOX_TSX, `export const box = "p-[3px]";\n`),
      type(N, CLEAN),
      close(BOX_TSX),
      type(N2, CLEAN),
      type(USES, `${USES_TSX}\n`),
    ],
    fresh: [type(USES, `${USES_TSX}\n`)],
  },
  "W5c as W5 with a .tsx const, the buffer still open": {
    before: withTsxBox,
    steps: [type(BOX_TSX, `export const box = "p-[3px]";\n`), type(N, CLEAN), type(USES, `${USES_TSX}\n`)],
    fresh: [type(BOX_TSX, `export const box = "p-[3px]";\n`), type(USES, `${USES_TSX}\n`)],
  },
  "W5 an unsaved bracket in an imported const, no rebuild, a reopen, the buffer still open": {
    before: withBox,
    steps: [type(BOX, `export const box = "p-[3px]";\n`), type(N, CLEAN), type(USES, `${USES_TEXT}\n`)],
    fresh: [type(BOX, `export const box = "p-[3px]";\n`), type(USES, `${USES_TEXT}\n`)],
  },
};

function restore() {
  for (const item of ["src", "tsconfig.json"]) {
    rmSync(path.join(R, item), { recursive: true, force: true });
    cpSync(path.join(fixture, item), path.join(R, item), { recursive: true });
  }
  rmSync(path.join(R, ".rafters"), { recursive: true, force: true });
  mkdirSync(path.join(R, ".rafters"));
  writeFileSync(
    path.join(R, ".rafters/config.rafters.json"),
    JSON.stringify({ framework: "astro", componentsPath: "src/components/ui" }),
  );
}

async function drive(mode, before, steps) {
  restore();
  for (const { disk: file, text } of before ?? []) writeFileSync(path.join(R, file), text);
  const worker = spawn(
    process.execPath,
    ["--import", path.join(here, "modes.mjs"), path.join(pkg, "dist/worker.mjs"), R],
    { stdio: ["pipe", "pipe", "inherit"], env: { ...process.env, MODE: mode } },
  );
  const lines = createInterface({ input: worker.stdout })[Symbol.asyncIterator]();
  const next = async () => JSON.parse((await lines.next()).value);
  const ready = await next();
  if (!ready.ready) throw new Error(JSON.stringify(ready));
  const answers = [];
  let seq = 0;
  for (const step of steps) {
    if (step.disk) {
      writeFileSync(path.join(R, step.disk), step.text);
      continue;
    }
    seq += 1;
    worker.stdin.write(`${JSON.stringify({ seq, ...step })}\n`);
    const answer = await next();
    if (answer.error) answers.push(`error ${answer.error}`);
    else if (!step.close) {
      answers.push(`${step.file}: [${answer.diagnostics.map((d) => `${d.line}:${d.column} ${d.message}`).join("; ")}]`);
    }
  }
  worker.stdin.end();
  worker.kill();
  return answers;
}

for (const mode of modes) {
  console.log(`\n=== MODE ${mode}`);
  for (const [name, { before, steps, fresh }] of Object.entries(SEQUENCES)) {
    const live = await drive(mode, before, steps);
    const finalDisk = steps.filter((step) => step.disk);
    const baseline = await drive("none", [...(before ?? []), ...finalDisk], fresh);
    const match = live.at(-1) === baseline.at(-1);
    console.log(`${match ? "same " : "DIFF "} ${name}`);
    for (const answer of live) console.log(`      live  ${answer}`);
    console.log(`      fresh ${baseline.at(-1)}`);
  }
}
restore();
process.exit(0);
