import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { cpSync, mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Given, Then, When } from "@cucumber/cucumber";
import type { Diagnostic, LintDiagnostic } from "@rafters/veneer";
import { INSTALL_TIMEOUT_MS, installOnce, pkg, type DocsWorld } from "./support/docs-fixture.ts";

const STEP_TIMEOUT_MS = 60 * 1000;

// FR-VEN-064's After removes the folders.
interface LintWorld extends DocsWorld {
  lintCopy?: string;
  files?: string;
  found?: LintDiagnostic[];
  diagnostics?: LintDiagnostic[];
  reported?: string;
}

// A child process each time, as FR-VEN-074's scenarios do: an open leaves memory behind (#134),
// and every scenario running in this process exhausts the heap.
function withProject<Result>(world: LintWorld, expression: string): Result {
  assert.ok(world.lintCopy, "the lint fixture is not set up in this scenario");
  const script = `import { lint, openProject } from ${JSON.stringify(join(pkg, "dist/index.mjs"))};
const project = await openProject(${JSON.stringify(world.lintCopy)});
process.stdout.write(JSON.stringify(await ${expression}));
project.close();
`;
  const { status, stdout, stderr } = spawnSync(
    process.execPath,
    ["--input-type=module", "--eval", script],
    { encoding: "utf8", timeout: STEP_TIMEOUT_MS },
  );
  assert.equal(status, 0, stderr);
  return JSON.parse(stdout);
}

function runLint(world: LintWorld, folders: string, files: string): LintDiagnostic[] {
  return withProject(world, `lint(project, { componentFolders: ${folders}, files: ${files} })`);
}

function found(world: LintWorld): LintDiagnostic[] {
  assert.ok(world.found, "lint has not run");
  return world.found;
}

function reportLines(world: LintWorld): string[] {
  assert.ok(world.reported !== undefined, "report has not run");
  assert.ok(world.reported.endsWith("\n"));
  return world.reported.slice(0, -1).split("\n");
}

Given(
  /^the lint fixture, opened with openProject$/,
  { timeout: INSTALL_TIMEOUT_MS },
  function (this: LintWorld) {
    const folder = mkdtempSync(join(tmpdir(), "veneer-lint-"));
    (this.folders ??= []).push(folder);
    cpSync(join(pkg, "test/fixtures/lint"), folder, { recursive: true });
    installOnce(this, folder);
    this.lintCopy = folder;
  },
);

Given(
  /^project\.diagnostics\("([^"]+)"\) includes one of code "([^"]+)" at line (\d+)$/,
  { timeout: STEP_TIMEOUT_MS },
  function (this: LintWorld, file: string, code: string, line: string) {
    const diagnostics = withProject<Diagnostic[]>(
      this,
      `project.diagnostics(${JSON.stringify(file)})`,
    );
    assert.ok(
      diagnostics.some((found) => found.code === code && found.line === Number(line)),
      JSON.stringify(diagnostics),
    );
  },
);

Given(
  /^two diagnostics, the first with help "a, b" and the message "bad\\nmore"$/,
  function (this: LintWorld) {
    const at = { line: 1, column: 2, endLine: 1, endColumn: 3, severity: "error" } as const;
    this.diagnostics = [
      { file: "a.astro", ...at, code: "ts(2322)", message: "bad\nmore", help: "a, b" },
      { file: "b.astro", ...at, code: "ts(2741)", message: "worse" },
    ];
  },
);

When(
  /^lint runs with componentFolders (\[[^\]]*\])(?: and files (\[[^\]]*\]))?$/,
  { timeout: STEP_TIMEOUT_MS },
  function (this: LintWorld, folders: string, files?: string) {
    this.files = files ?? "undefined";
    this.found = runLint(this, folders, this.files);
  },
);

When(/^report runs on them$/, async function (this: LintWorld) {
  assert.ok(this.diagnostics, "no diagnostics in this scenario");
  const { report } = await import("@rafters/veneer");
  this.reported = report(this.diagnostics);
});

Then(
  /^its diagnostics, in order, equal the lines of lint\.expected\.jsonl on the fields those lines hold$/,
  function (this: LintWorld) {
    const expected: Record<string, unknown>[] = readFileSync(
      join(pkg, "test/fixtures/lint.expected.jsonl"),
      "utf8",
    )
      .trimEnd()
      .split("\n")
      .map((line) => JSON.parse(line));
    // help is kept whether or not the line holds it: a line with none means the diagnostic has none.
    const held = found(this).map((diagnostic, index) =>
      Object.fromEntries(
        Object.entries(diagnostic).filter(
          ([field]) => field === "help" || field in (expected[index] ?? {}),
        ),
      ),
    );
    assert.deepEqual(held, expected);
  },
);

Then(/^no reported diagnostic is at line (\d+)$/, function (this: LintWorld, line: string) {
  assert.deepEqual(
    found(this).filter((diagnostic) => diagnostic.line === Number(line)),
    [],
  );
});

Then(/^it reports nothing$/, function (this: LintWorld) {
  assert.deepEqual(found(this), []);
});

Then(
  /^when lint runs with componentFolders (\[[^\]]*\]) and the same files it reports one diagnostic of code "([^"]+)"$/,
  { timeout: STEP_TIMEOUT_MS },
  function (this: LintWorld, folders: string, code: string) {
    assert.ok(this.files, "lint has not run");
    const again = runLint(this, folders, this.files);
    assert.deepEqual(
      again.map((diagnostic) => diagnostic.code),
      [code],
    );
  },
);

Then(/^each is one line with the first line of its message$/, function (this: LintWorld) {
  const lines = reportLines(this);
  assert.equal(lines[0], "a.astro:1:2 error ts(2322) bad");
  assert.equal(lines[2], "b.astro:1:2 error ts(2741) worse");
});

Then(/^the first is followed by the line "(.+)"$/, function (this: LintWorld, line: string) {
  assert.equal(reportLines(this)[1], line);
});

Then(/^the last line is "(.+)"$/, function (this: LintWorld, line: string) {
  assert.equal(reportLines(this).at(-1), line);
});

Then(
  /^the diagnostic of code "([^"]+)" has help "(.+)"$/,
  function (this: LintWorld, code: string, help: string) {
    const diagnostic = found(this).find((candidate) => candidate.code === code);
    assert.ok(diagnostic, `no diagnostic of code ${code}`);
    assert.equal(diagnostic.help, help.replaceAll('\\"', '"'));
  },
);
