// node matrix.mjs <scratch folder holding installed copies read/ and lint/> [mode ...]
// Each case's LAST call, in a process that made the earlier calls, against a fresh process that made only the last call.
import { spawnSync } from "node:child_process";
import { cpSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = fileURLToPath(new URL(".", import.meta.url));
const fixtures = path.join(here, "../../packages/veneer/test/fixtures");
const [scratch, ...modes] = process.argv.slice(2);
const R = path.join(scratch, "read");
const L = path.join(scratch, "lint");
const SHINGLE = "/Volumes/store/projects/rafters-studio/shingle/sites/rafters.studio";

const PAGE = "src/pages/index.astro";
const CARD = "src/components/ui/card.astro";
const BUTTON = "src/components/button.tsx";
const disk = (file) => readFileSync(path.join(fixtures, "read", file), "utf8");
const B = `\n<div class="p-[3px]"></div>\n`;
const B2 = `\n<div class="m-[7px]"></div>\n`;
const call = (options, root = R) => ({ root, options });
const unsaved = (file, text) => call({ unsaved: { [file]: text } });
const write = (file, text, root = R) => ({ write: path.join(root, file), text });
const remove = (file, root = R) => ({ remove: path.join(root, file) });
const FB = "src/components/FeatureBlock.astro";
const fb = readFileSync(path.join(SHINGLE, FB), "utf8");
const shingle = (file, text) => call({ unsaved: { [file]: text } }, SHINGLE);

const CASES = {
  "shingle unchanged-then-append (the issue)": [shingle(FB, fb), shingle(FB, `${fb}${B}`)],
  "shingle different file on the later call (the issue)": [
    shingle(FB, fb),
    shingle("src/components/Section.astro", `---\n---\n${B}`),
  ],
  "read: same file, new text (issue scenario)": [unsaved(PAGE, disk(PAGE)), unsaved(PAGE, `${disk(PAGE)}${B}`)],
  "read: reverse, finding then clean text": [unsaved(PAGE, `${disk(PAGE)}${B}`), unsaved(PAGE, disk(PAGE))],
  "read: reverse, finding then the disk text by paths": [
    unsaved(PAGE, `${disk(PAGE)}${B}`),
    call({ paths: [PAGE] }),
  ],
  "read: third call": [
    unsaved(PAGE, disk(PAGE)),
    unsaved(PAGE, `${disk(PAGE)}${B}`),
    unsaved(PAGE, `${disk(PAGE)}${B2}`),
  ],
  "read: phantom, later text has the same shape": [
    unsaved(PAGE, `---\n---\n<div class="p-[3px]"></div>\n`),
    unsaved(PAGE, `---\n---\n<div class="p-4xxxx"></div>\n`),
  ],
  "read: different file later": [
    unsaved(PAGE, disk(PAGE)),
    unsaved(BUTTON, `export const X = <div className="p-[3px]" />;\n`),
  ],
  "read: imported component's props change on disk (a type finding)": [
    call({ paths: [PAGE] }),
    write(CARD, disk(CARD).replace("title: string", "title: number")),
    call({ paths: [PAGE] }),
  ],
  "read: imported component's props change in unsaved": [
    call({ paths: [PAGE] }),
    call({
      paths: [PAGE],
      unsaved: { [CARD]: disk(CARD).replace("title: string", "title: number") },
    }),
  ],
  "read: new file later": [unsaved(PAGE, disk(PAGE)), unsaved("src/pages/new.astro", `---\n---\n${B}`)],
  "read: tsx new text": [
    unsaved(BUTTON, disk(BUTTON)),
    unsaved(BUTTON, `${disk(BUTTON)}\nexport const X = <div className="p-[3px]" />;\n`),
  ],
  "read: disk edit between calls, no unsaved": [
    call({ paths: [PAGE] }),
    write(PAGE, `${disk(PAGE)}${B}`),
    call({ paths: [PAGE] }),
  ],
  "read: disk edit that keeps the same mtime": [
    call({ paths: [PAGE] }),
    { ...write(PAGE, `${disk(PAGE)}${B}`), keepTime: true },
    call({ paths: [PAGE] }),
  ],
  "read: disk edit of an imported const": [
    write("src/box.ts", `export const box = "p-4";\n`),
    write("src/pages/uses.astro", `---\nimport { box } from "../box.ts";\n---\n<div class={box}></div>\n`),
    call({ paths: ["src/pages/uses.astro"] }),
    write("src/box.ts", `export const box = "p-[3px]";\n`),
    call({ paths: ["src/pages/uses.astro"] }),
  ],
  "read: tsconfig include narrowed between calls": [
    call({}),
    write("tsconfig.json", JSON.stringify({ ...JSON.parse(disk("tsconfig.json")), include: ["src/components/**/*.astro"] })),
    call({}),
  ],
  "read: imported file removed between calls": [call({}), remove(CARD), call({})],
  "read: save, disk edit, save (docs.jsonl compared too)": [
    call({ save: true }),
    write(CARD, disk(CARD).replace("title: string", "title: string; extra?: number")),
    call({ save: true }),
  ],
  "lint after read: a different root": [call({}), call({}, L)],
  "read after lint after read: back to the first root with new text": [
    unsaved(PAGE, disk(PAGE)),
    call({}, L),
    unsaved(PAGE, `${disk(PAGE)}${B}`),
  ],
};

function restore() {
  for (const item of ["src", "tsconfig.json"]) {
    rmSync(path.join(R, item), { recursive: true, force: true });
    cpSync(path.join(fixtures, "read", item), path.join(R, item), { recursive: true });
  }
  rmSync(path.join(R, ".rafters"), { recursive: true, force: true });
  mkdirSync(path.join(R, ".rafters"));
  writeFileSync(
    path.join(R, ".rafters/config.rafters.json"),
    JSON.stringify({ framework: "astro", componentsPath: "src/components/ui" }),
  );
}

function run(mode, steps) {
  restore();
  const { stdout, stderr, status } = spawnSync(
    process.execPath,
    ["--import", path.join(here, "modes.mjs"), path.join(here, "child.mjs"), JSON.stringify(steps)],
    { encoding: "utf8", env: { ...process.env, MODE: mode }, maxBuffer: Infinity, timeout: 180000 },
  );
  restore();
  const lines = stdout.trim().split("\n").filter(Boolean).map((line) => JSON.parse(line));
  return { lines, status, stderr: stderr.trim().split("\n").slice(-2).join(" ") };
}

const onlyLastCall = (steps) => {
  const last = steps.findLastIndex((step) => step.root);
  return steps.filter((step, at) => !step.root || at === last);
};
const short = (result) => {
  if (!result?.ok) return JSON.stringify(result?.report);
  const lines = String(result.report).trim().split("\n");
  const first = lines.length > 3 ? ` ... ${lines.length} lines` : "";
  return `${lines.slice(0, 2).join(" | ").slice(0, 160)}${first} | ${lines.at(-1)}`;
};
const same = (a, b) =>
  JSON.stringify([a?.ok, a?.report, a?.diagnostics, a?.notices, a?.docs]) ===
  JSON.stringify([b?.ok, b?.report, b?.diagnostics, b?.notices, b?.docs]);

for (const mode of modes) {
  console.log(`\n=== MODE ${mode}`);
  let wrong = 0;
  for (const [name, steps] of Object.entries(CASES)) {
    const later = run(mode, steps);
    const fresh = run("none", onlyLastCall(steps));
    const a = later.lines.at(-1);
    const b = fresh.lines.at(-1);
    const match = later.status === 0 && fresh.status === 0 && same(a, b);
    if (!match) wrong += 1;
    console.log(`${match ? "same " : "DIFF "} ${name}`);
    console.log(`      later ${a?.ms}ms: ${short(a)}`);
    console.log(`      fresh ${b?.ms}ms: ${short(b)}`);
    if (later.status !== 0) console.log(`      later exit ${later.status}: ${later.stderr}`);
    if (fresh.status !== 0) console.log(`      fresh exit ${fresh.status}: ${fresh.stderr}`);
  }
  console.log(`--- ${mode}: ${wrong} of ${Object.keys(CASES).length} differ from a fresh process`);
}
