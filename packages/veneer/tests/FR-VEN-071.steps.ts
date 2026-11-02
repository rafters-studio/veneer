import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { Given, Then, When } from "@cucumber/cucumber";
import type { CheckResult, LintDiagnostic } from "@rafters/veneer";
import {
  DOCS,
  INSTALL_TIMEOUT_MS,
  installOnce,
  pkg,
  type DocsWorld,
} from "./support/docs-fixture.ts";

const CHILD_MS = 60 * 1000;
const OUTPUT = ".rafters/veneer";
const INDEX = join(pkg, "dist/index.mjs");

// FR-VEN-064's After removes the folders.
interface CheckWorld extends DocsWorld {
  checkCopy?: string;
  // FR-VEN-064's "does not exist on disk" reads this.
  fixture?: string;
  unsavedFile?: string;
  bytesBefore?: Buffer;
  checked?: CheckResult;
  // FR-VEN-064's "node runs the child script" runs this and sets run.
  script?: string;
  run?: { stdout: string; stderr: string };
}

function copyOf(world: CheckWorld): string {
  assert.ok(world.checkCopy, "the lint fixture is not set up in this scenario");
  return world.checkCopy;
}

// A child process each time, as FR-VEN-066's scenarios do: an open leaves memory behind (#134).
function runCheck(world: CheckWorld, options: object): void {
  const script = `import { check } from ${JSON.stringify(INDEX)};
const result = await check(${JSON.stringify(copyOf(world))}, ${JSON.stringify(options)});
process.stdout.write(JSON.stringify(result));
`;
  const { status, stdout, stderr } = spawnSync(
    process.execPath,
    ["--input-type=module", "--eval", script],
    { encoding: "utf8", timeout: CHILD_MS },
  );
  assert.equal(status, 0, stderr);
  world.checked = JSON.parse(stdout);
}

function okResult(world: CheckWorld): Extract<CheckResult, { ok: true }> {
  assert.ok(world.checked, "check has not run");
  assert.ok(world.checked.ok, JSON.stringify(world.checked));
  return world.checked;
}

Given(/^the lint fixture$/, { timeout: INSTALL_TIMEOUT_MS }, function (this: CheckWorld) {
  const folder = mkdtempSync(join(tmpdir(), "veneer-check-"));
  (this.folders ??= []).push(folder);
  cpSync(join(pkg, "test/fixtures/lint"), folder, { recursive: true });
  installOnce(this, folder);
  this.checkCopy = folder;
  this.fixture = folder;
});

Given(
  /^a child script, written by the step to a temporary \.mjs file, that imports check from the absolute path of packages\/veneer\/dist\/index\.mjs, calls it with save true and writes the line resolved to stdout, and never calls process\.exit$/,
  function (this: CheckWorld) {
    const folder = mkdtempSync(join(tmpdir(), "veneer-check-child-"));
    (this.folders ??= []).push(folder);
    this.script = join(folder, "child.mjs");
    writeFileSync(
      this.script,
      `import { check } from ${JSON.stringify(INDEX)};
await check(${JSON.stringify(copyOf(this))}, { save: true });
process.stdout.write("resolved\\n");
`,
    );
  },
);

When(/^check runs with save true$/, { timeout: CHILD_MS }, function (this: CheckWorld) {
  runCheck(this, { save: true });
});

When(
  /^check runs with save true and unsaved holding (\S+) with the text "(.*)"$/,
  { timeout: CHILD_MS },
  function (this: CheckWorld, file: string, text: string) {
    this.unsavedFile = file;
    this.bytesBefore = readFileSync(join(copyOf(this), file));
    runCheck(this, { save: true, unsaved: { [file]: text.replaceAll('\\"', '"') } });
  },
);

When(
  /^check runs with unsaved holding (\S+) with the text "(.*)"$/,
  { timeout: CHILD_MS },
  function (this: CheckWorld, file: string, text: string) {
    const written = text.replaceAll("\\n", "\n").replaceAll('\\"', '"');
    runCheck(this, { unsaved: { [file]: written } });
  },
);

When(
  /^check runs with paths (\[.*\])$/,
  { timeout: CHILD_MS },
  function (this: CheckWorld, paths: string) {
    runCheck(this, { paths: JSON.parse(paths) });
  },
);

Then(
  /^its diagnostics have as many entries as lint\.expected\.jsonl has lines$/,
  function (this: CheckWorld) {
    const expected = readFileSync(join(pkg, "test/fixtures/lint.expected.jsonl"), "utf8")
      .trimEnd()
      .split("\n");
    assert.equal(okResult(this).diagnostics.length, expected.length);
  },
);

Then(
  /^docsPath is \.rafters\/veneer\/docs\.jsonl under the copy, and that file exists$/,
  function (this: CheckWorld) {
    const { docsPath } = okResult(this);
    assert.equal(docsPath, resolve(copyOf(this), DOCS));
    assert.ok(existsSync(docsPath), `${docsPath} does not exist`);
  },
);

Then(
  /^its report names (\S+) at (\S+:\d+:\d+)$/,
  function (this: CheckWorld, className: string, at: string) {
    const line = okResult(this)
      .report.split("\n")
      .find((entry) => entry.startsWith(`${at} `));
    assert.ok(line, okResult(this).report);
    assert.ok(line.includes(className), line);
  },
);

Then(/^\.rafters\/veneer does not exist in the copy$/, function (this: CheckWorld) {
  assert.ok(!existsSync(join(copyOf(this), OUTPUT)));
  assert.equal(okResult(this).docsPath, undefined);
});

Then(/^(\S+) on disk is unchanged$/, function (this: CheckWorld, file: string) {
  assert.equal(file, this.unsavedFile);
  assert.ok(this.bytesBefore, "no bytes were kept");
  assert.deepEqual(readFileSync(join(copyOf(this), file)), this.bytesBefore);
});

Then(/^every diagnostic's file is (\S+)$/, function (this: CheckWorld, file: string) {
  const diagnostics: LintDiagnostic[] = okResult(this).diagnostics;
  assert.ok(diagnostics.length > 0, "check reported nothing");
  assert.deepEqual(
    diagnostics.filter((diagnostic) => diagnostic.file !== file),
    [],
  );
});

Then(
  /^its one diagnostic is at (\S+) (\d+):(\d+) with code "([^"]+)" and help "([^"]+)"$/,
  function (
    this: CheckWorld,
    file: string,
    line: string,
    column: string,
    code: string,
    help: string,
  ) {
    const { diagnostics } = okResult(this);
    assert.equal(diagnostics.length, 1, JSON.stringify(diagnostics));
    const [found] = diagnostics;
    assert.deepEqual(
      [found?.file, found?.line, found?.column, found?.code, found?.help],
      [file, Number(line), Number(column), code, help],
    );
  },
);

Then(/^it returns ok true with no diagnostics$/, function (this: CheckWorld) {
  assert.deepEqual(okResult(this).diagnostics, []);
});

Then(
  /^its notices hold one of code "([^"]+)" naming (\S+)$/,
  function (this: CheckWorld, code: string, file: string) {
    const held = okResult(this).notices.filter((notice) => notice.code === code);
    assert.equal(held.length, 1, JSON.stringify(okResult(this).notices));
    assert.ok(held[0]?.message.includes(file), held[0]?.message);
  },
);

Then(/^its stdout is the line resolved$/, function (this: CheckWorld) {
  assert.ok(this.run, "the child script has not run");
  assert.equal(this.run.stdout, "resolved\n", this.run.stderr);
});
