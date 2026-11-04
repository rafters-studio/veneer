import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { createInterface } from "node:readline";
import { Given, Then, When } from "@cucumber/cucumber";
import type { CheckResult } from "@rafters/veneer";
import type { CommandWorld } from "./check-command.steps.ts";
import { DOCS, INSTALL_TIMEOUT_MS, installOnce, pkg } from "./support/docs-fixture.ts";

const CHILD_MS = 60 * 1000;
const INDEX = join(pkg, "dist/index.mjs");
const WORKER = join(pkg, "dist/worker.mjs");

const CLEAN = '<div class="p-4"></div>\n';
const BRACKET = '<div class="p-[3px]"></div>\n';
const SAME_SHAPE = '<div class="p-12345"></div>\n';
const PAGE = "src/pages/second.astro";
const OTHER = "src/pages/other.astro";
const DISK = "src/pages/disk.astro";
const USES = "src/pages/uses-box.astro";
const BOX = "src/lib/box.ts";
const BOX_TSX = "src/lib/boxx.tsx";
const PROPS_PAGE = "src/pages/props.astro";
const CARD = "src/components/ui/card.astro";
const CARD_TEXT = readFileSync(join(pkg, "test/fixtures/read", CARD), "utf8");
const CARD_EDITED = CARD_TEXT.replace("title: string", "title: number");
const NEW = "src/pages/n.astro";
const NEW2 = "src/pages/n2.astro";

interface Call {
  options: { save?: boolean; paths?: string[]; unsaved?: Record<string, string> };
  // Disk writes made before the call.
  disk?: Record<string, string>;
}

interface SecondWorld extends CommandWorld {
  second?: CheckResult;
  fresh?: CheckResult;
  docsAfterSecond?: string;
  docsFresh?: string;
  lastAnswer?: unknown;
  freshAnswer?: unknown;
}

const project = (world: SecondWorld): string => {
  assert.ok(world.checkCopy, "the project is not set up in this scenario");
  return world.checkCopy;
};

function place(root: string, files: Record<string, string>): void {
  for (const [file, text] of Object.entries(files)) {
    mkdirSync(dirname(join(root, file)), { recursive: true });
    writeFileSync(join(root, file), text);
  }
}

// The files the scenarios change, none with a finding, added to the read fixture.
function addFiles(root: string): void {
  place(root, {
    [PAGE]: CLEAN,
    [OTHER]: CLEAN,
    [DISK]: CLEAN,
    [BOX]: 'export const box = "p-4";\n',
    [USES]: '---\nimport { box } from "../lib/box.ts";\n---\n<div class={box}></div>\n',
    [PROPS_PAGE]: '---\nimport Card from "../components/ui/card.astro";\n---\n<Card title="x" />\n',
  });
}

// All calls run in this one process, as the language server's worker would run them.
function runCalls(root: string, calls: Call[]): CheckResult[] {
  const script = `import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { check } from ${JSON.stringify(INDEX)};
const root = ${JSON.stringify(root)};
const results = [];
for (const call of ${JSON.stringify(calls)}) {
  for (const [file, text] of Object.entries(call.disk ?? {})) writeFileSync(join(root, file), text);
  results.push(await check(root, call.options));
}
process.stdout.write(JSON.stringify(results));
`;
  const { status, stdout, stderr } = spawnSync(
    process.execPath,
    ["--input-type=module", "--eval", script],
    { encoding: "utf8", timeout: CHILD_MS },
  );
  assert.equal(status, 0, stderr);
  return JSON.parse(stdout);
}

function lastOf(results: CheckResult[]): CheckResult {
  const last = results.at(-1);
  assert.ok(last, "no call ran");
  return last;
}

const CHANGES: Record<string, { first: Call; second: Call }> = {
  "a bracket value added through unsaved text": {
    first: { options: { unsaved: { [PAGE]: CLEAN } } },
    second: { options: { unsaved: { [PAGE]: `${CLEAN}${BRACKET}` } } },
  },
  "a bracket value in a different unsaved file": {
    first: { options: { unsaved: { [PAGE]: CLEAN } } },
    second: { options: { unsaved: { [OTHER]: `${CLEAN}${BRACKET}` } } },
  },
  "a bracket value written to the page on disk": {
    first: { options: { paths: [DISK] } },
    second: { options: { paths: [DISK] }, disk: { [DISK]: `${CLEAN}${BRACKET}` } },
  },
  "an imported const edited on disk": {
    first: { options: { paths: [USES] } },
    second: { options: { paths: [USES] }, disk: { [BOX]: 'export const box = "p-[3px]";\n' } },
  },
  "a component's props edited on disk": {
    first: { options: { paths: [PROPS_PAGE] } },
    second: { options: { paths: [PROPS_PAGE] }, disk: { [CARD]: CARD_EDITED } },
  },
};

When(
  /^one process runs check, then (.+), then check again$/,
  { timeout: INSTALL_TIMEOUT_MS },
  function (this: SecondWorld, change: string) {
    const { first, second } = CHANGES[change] ?? assert.fail(`no change named ${change}`);
    const root = project(this);
    addFiles(root);
    this.second = lastOf(runCalls(root, [first, second]));
    // The disk now holds what the change wrote, so a fresh process sees the same state.
    this.fresh = lastOf(runCalls(root, [{ options: second.options }]));
  },
);

Then(/^the second report equals the report of a fresh process$/, function (this: SecondWorld) {
  assert.ok(this.second?.ok && this.fresh?.ok, JSON.stringify([this.second, this.fresh]));
  assert.ok(
    this.fresh.diagnostics.length > 0,
    "a fresh process reports nothing, so nothing is compared",
  );
  assert.deepEqual(this.second, this.fresh);
});

When(
  /^one process runs check with p-\[3px\] on a page, then with clean text of the same shape$/,
  { timeout: INSTALL_TIMEOUT_MS },
  function (this: SecondWorld) {
    const root = project(this);
    addFiles(root);
    this.second = lastOf(
      runCalls(root, [
        { options: { unsaved: { [PAGE]: BRACKET } } },
        { options: { unsaved: { [PAGE]: SAME_SHAPE } } },
      ]),
    );
  },
);

Then(/^the second report has no finding$/, function (this: SecondWorld) {
  assert.ok(this.second?.ok, JSON.stringify(this.second));
  assert.deepEqual(this.second.diagnostics, []);
});

When(
  /^one process runs check with save, edits a component's props on disk, and runs check with save again$/,
  { timeout: INSTALL_TIMEOUT_MS },
  function (this: SecondWorld) {
    const root = project(this);
    addFiles(root);
    const docs = join(root, DOCS);
    runCalls(root, [
      { options: { save: true } },
      { options: { save: true }, disk: { [CARD]: CARD_EDITED } },
    ]);
    this.docsAfterSecond = readFileSync(docs, "utf8");
    rmSync(docs);
    runCalls(root, [{ options: { save: true } }]);
    this.docsFresh = readFileSync(docs, "utf8");
  },
);

Then(/^docs\.jsonl equals what a fresh process writes$/, function (this: SecondWorld) {
  assert.match(this.docsFresh ?? "", /number/);
  assert.equal(this.docsAfterSecond, this.docsFresh);
});

type Step = { type: string; text: string } | { close: string } | { disk: string; text: string };

const type = (file: string, text: string): Step => ({ type: file, text });
const close = (file: string): Step => ({ close: file });
const disk = (file: string, text: string): Step => ({ disk: file, text });
const NEW_CLEAN = `---\n---\n${CLEAN}`;
const NEW_BRACKET = `---\n---\n${BRACKET}`;
const boxText = (value: string): string => `export const box = "${value}";\n`;
const importer = (from: string): string =>
  `---\nimport { box } from "${from}";\n---\n<div class={box}></div>\n`;
const USES_TEXT = importer("../lib/box.ts");
const USES_TSX_TEXT = importer("../lib/boxx.tsx");

const SEQUENCES: Record<string, { before?: Step[]; steps: Step[]; fresh: Step[]; finds: boolean }> =
  {
    "a new file typed, closed, and typed again with a bracket": {
      steps: [type(NEW, NEW_CLEAN), close(NEW), type(NEW, NEW_BRACKET)],
      fresh: [type(NEW, NEW_BRACKET)],
      finds: true,
    },
    "an imported const changed on disk and a new file opened": {
      steps: [
        type(USES, USES_TEXT),
        disk(BOX, boxText("p-[3px]")),
        type(NEW, NEW_CLEAN),
        type(USES, `${USES_TEXT}\n`),
      ],
      fresh: [type(NEW, NEW_CLEAN), type(USES, `${USES_TEXT}\n`)],
      finds: true,
    },
    "a tsx const typed, closed, and a reopen before its importer is typed": {
      before: [disk(BOX_TSX, boxText("p-4")), disk(USES, USES_TSX_TEXT)],
      steps: [
        type(BOX_TSX, boxText("p-[3px]")),
        type(NEW, NEW_CLEAN),
        close(BOX_TSX),
        type(NEW2, NEW_CLEAN),
        type(USES, `${USES_TSX_TEXT}\n`),
      ],
      fresh: [type(USES, `${USES_TSX_TEXT}\n`)],
      finds: false,
    },
  };

// The worker's answer for the last file it was sent, over its JSON lines.
async function drive(root: string, steps: Step[]): Promise<unknown> {
  const worker = spawn(process.execPath, [WORKER, root], { stdio: ["pipe", "pipe", "inherit"] });
  const lines = createInterface({ input: worker.stdout })[Symbol.asyncIterator]();
  const next = async (): Promise<Record<string, unknown>> => {
    const { value } = await lines.next();
    assert.equal(typeof value, "string", "the worker closed its output");
    return JSON.parse(value);
  };
  try {
    assert.ok((await next()).ready, "the worker is not ready");
    let answer: unknown;
    let seq = 0;
    for (const step of steps) {
      if ("disk" in step) {
        place(root, { [step.disk]: step.text });
        continue;
      }
      seq += 1;
      const message =
        "close" in step ? { seq, close: step.close } : { seq, file: step.type, text: step.text };
      worker.stdin.write(`${JSON.stringify(message)}\n`);
      const reply = await next();
      assert.equal(reply.error, undefined, JSON.stringify(reply));
      if (!("close" in step)) answer = reply.diagnostics;
    }
    return answer;
  } finally {
    worker.kill();
  }
}

Given(
  /^a worker open on a project$/,
  { timeout: INSTALL_TIMEOUT_MS },
  function (this: SecondWorld) {
    const folder = mkdtempSync(join(tmpdir(), "veneer-second-"));
    (this.folders ??= []).push(folder);
    cpSync(join(pkg, "test/fixtures/read"), folder, { recursive: true });
    installOnce(this, folder);
    addFiles(folder);
    this.checkCopy = folder;
  },
);

When(
  /^the worker is driven through (.+)$/,
  { timeout: INSTALL_TIMEOUT_MS },
  async function (this: SecondWorld, name: string) {
    const {
      before = [],
      steps,
      fresh,
      finds,
    } = SEQUENCES[name] ?? assert.fail(`no sequence ${name}`);
    const root = project(this);
    for (const step of before) if ("disk" in step) place(root, { [step.disk]: step.text });
    this.lastAnswer = await drive(root, steps);
    // The disk now holds the sequence's writes, so a fresh worker sees the same state.
    this.freshAnswer = await drive(root, fresh);
    // A sequence that expects a finding must not pass by both answers being empty.
    if (finds) assert.notDeepEqual(this.freshAnswer, [], "a fresh worker reports nothing");
  },
);

Then(
  /^the worker's answer for the last file equals a fresh worker's$/,
  function (this: SecondWorld) {
    assert.deepEqual(this.lastAnswer, this.freshAnswer);
  },
);
