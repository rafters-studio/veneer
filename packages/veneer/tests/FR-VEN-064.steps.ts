import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { cpSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { After, Given, Then, When } from "@cucumber/cucumber";
import type { Diagnostic, Project } from "@rafters/veneer";

const pkg = fileURLToPath(new URL("../", import.meta.url));
const INSTALL_TIMEOUT_MS = 5 * 60 * 1000;
const STEP_TIMEOUT_MS = 60 * 1000;
const PAGE = "src/pages/index.astro";

interface Disk {
  bytes: Buffer;
  mtimeMs: number;
}

interface ReadWorld {
  folders?: string[];
  fixture?: string;
  noAstro?: string;
  project?: Project;
  disk?: Disk;
  sys?: Pick<typeof import("typescript").sys, "readFile" | "getModifiedTime" | "fileExists">;
  rejection?: unknown;
  script?: string;
  run?: { status: number | null; stdout: string; stderr: string };
}

// A dynamic import, so a checkout that has not built yet still loads every scenario file:
// FR-VEN-075's own scenario is the one that builds.
function veneer(): Promise<typeof import("@rafters/veneer")> {
  return import("@rafters/veneer");
}

function veneerRequire(id: string): unknown {
  return createRequire(join(pkg, "package.json"))(id);
}

function copyFixture(world: ReadWorld, name: string): string {
  const folder = mkdtempSync(join(tmpdir(), `veneer-${name}-`));
  (world.folders ??= []).push(folder);
  cpSync(join(pkg, "test/fixtures", name), folder, { recursive: true });
  return folder;
}

function install(folder: string, args: string[]): void {
  const { status, stderr, stdout } = spawnSync("pnpm", ["install", ...args], {
    cwd: folder,
    encoding: "utf8",
    timeout: INSTALL_TIMEOUT_MS,
    // pnpm fails an install that skips build scripts (esbuild, sharp); the read runs neither.
    env: { ...process.env, pnpm_config_strict_dep_builds: "false" },
  });
  assert.equal(status, 0, `${stdout}\n${stderr}`);
}

function installedCopy(world: ReadWorld, name: string): string {
  const folder = copyFixture(world, name);
  install(folder, ["--frozen-lockfile"]);
  return folder;
}

function fixture(world: ReadWorld): string {
  assert.ok(world.fixture, "no fixture copy in this scenario");
  return world.fixture;
}

function project(world: ReadWorld): Project {
  assert.ok(world.project, "no project is open in this scenario");
  return world.project;
}

function withFourthLine(world: ReadWorld, line: string): string {
  const lines = readFileSync(join(fixture(world), PAGE), "utf8").split("\n");
  lines[3] = line;
  return lines.join("\n");
}

function assertAt(found: Diagnostic | undefined, at: string): void {
  assert.ok(found, "the diagnostic is missing");
  const [line, column, endLine, endColumn] = (at.match(/\d+/g) ?? []).map(Number);
  assert.deepEqual(
    {
      line: found.line,
      column: found.column,
      endLine: found.endLine,
      endColumn: found.endColumn,
    },
    { line, column, endLine, endColumn },
  );
}

function noteDisk(path: string): Disk {
  return { bytes: readFileSync(path), mtimeMs: statSync(path).mtimeMs };
}

After(async function (this: ReadWorld) {
  const { ProjectError } = await veneer();
  try {
    this.project?.close();
  } catch (error) {
    if (!(error instanceof ProjectError && error.code === "closed")) throw error;
  }
  for (const folder of this.folders ?? []) rmSync(folder, { recursive: true, force: true });
});

Given(/^the read fixture$/, { timeout: INSTALL_TIMEOUT_MS }, function (this: ReadWorld) {
  this.fixture = installedCopy(this, "read");
});

Given(/^the no-Astro fixture$/, { timeout: INSTALL_TIMEOUT_MS }, function (this: ReadWorld) {
  this.fixture = installedCopy(this, "no-astro");
});

Given(
  /^the read fixture with "typescript": "7\.0\.2" added to its devDependencies, installed with pnpm install$/,
  { timeout: INSTALL_TIMEOUT_MS },
  function (this: ReadWorld) {
    const folder = copyFixture(this, "read");
    const file = join(folder, "package.json");
    const manifest: { devDependencies: Record<string, string> } = JSON.parse(
      readFileSync(file, "utf8"),
    );
    manifest.devDependencies.typescript = "7.0.2";
    writeFileSync(file, JSON.stringify(manifest));
    install(folder, []);
    this.fixture = folder;
  },
);

Given(
  /^the read fixture opened with openProject$/,
  { timeout: INSTALL_TIMEOUT_MS },
  async function (this: ReadWorld) {
    this.fixture = installedCopy(this, "read");
    const { openProject } = await veneer();
    this.project = await openProject(this.fixture);
  },
);

Given(
  /^the read fixture opened with openProject, with unsaved text set for src\/pages\/index\.astro$/,
  { timeout: INSTALL_TIMEOUT_MS },
  async function (this: ReadWorld) {
    this.fixture = installedCopy(this, "read");
    const { openProject } = await veneer();
    this.project = await openProject(this.fixture);
    this.project.setUnsaved(PAGE, withFourthLine(this, '<Card title="Unsaved" />'));
  },
);

Given(
  /^ts\.sys\.readFile, ts\.sys\.getModifiedTime and ts\.sys\.fileExists of require\("typescript"\) resolved from packages\/veneer, noted before openProject$/,
  function (this: ReadWorld) {
    const { sys } = veneerRequire("typescript") as typeof import("typescript");
    const { readFile, getModifiedTime, fileExists } = sys;
    this.sys = { readFile, getModifiedTime, fileExists };
  },
);

Given(
  /^the project is opened with openProject$/,
  { timeout: STEP_TIMEOUT_MS },
  async function (this: ReadWorld) {
    const { openProject } = await veneer();
    this.project = await openProject(fixture(this));
  },
);

Given(
  /^the bytes and modified time of src\/pages\/index\.astro are noted$/,
  function (this: ReadWorld) {
    this.disk = noteDisk(join(fixture(this), PAGE));
  },
);

When(
  /^setUnsaved\("src\/pages\/index\.astro", <the file's text with its fourth line replaced by `(.+)`>\) (?:is|has been) called$/,
  function (this: ReadWorld, line: string) {
    project(this).setUnsaved(PAGE, withFourthLine(this, line));
  },
);

When(/^clearUnsaved\("src\/pages\/index\.astro"\) is called$/, function (this: ReadWorld) {
  project(this).clearUnsaved(PAGE);
});

When(
  /^openProject is called on the no-Astro fixture before the first is closed$/,
  { timeout: INSTALL_TIMEOUT_MS },
  async function (this: ReadWorld) {
    this.noAstro = installedCopy(this, "no-astro");
    const { openProject } = await veneer();
    await openProject(this.noAstro).then(
      () => assert.fail("openProject resolved while a project was open"),
      (error: unknown) => {
        this.rejection = error;
      },
    );
  },
);

When(/^the project is closed$/, function (this: ReadWorld) {
  project(this).close();
});

Then(
  /^diagnostics\("src\/pages\/index\.astro"\) is one diagnostic with code "([^"]+)", severity "([^"]+)", (line \d+, column \d+, endLine \d+, endColumn \d+)$/,
  { timeout: STEP_TIMEOUT_MS },
  async function (this: ReadWorld, code: string, severity: string, at: string) {
    const found = await project(this).diagnostics(PAGE);
    assert.equal(found.length, 1, JSON.stringify(found));
    assert.equal(found[0]?.code, code);
    assert.equal(found[0]?.severity, severity);
    assertAt(found[0], at);
  },
);

Then(
  /^diagnostics\("src\/pages\/index\.astro"\) is empty$/,
  { timeout: STEP_TIMEOUT_MS },
  async function (this: ReadWorld) {
    assert.deepEqual(await project(this).diagnostics(PAGE), []);
  },
);

Then(
  /^src\/pages\/index\.astro has the noted bytes and modified time$/,
  function (this: ReadWorld) {
    assert.ok(this.disk, "the file was not noted");
    assert.deepEqual(noteDisk(join(fixture(this), PAGE)), this.disk);
  },
);

Then(/^project\.ts\.version is "([^"]+)"$/, function (this: ReadWorld, version: string) {
  assert.equal(project(this).ts.version, version);
});

Then(
  /^project\.ts is the module require\("typescript"\) gives from veneer's own package$/,
  function (this: ReadWorld) {
    assert.equal(project(this).ts, veneerRequire("typescript"));
  },
);

Given(
  /^a child script, written by the step to a temporary \.mjs file, that imports openProject from the absolute path of packages\/veneer\/dist\/index\.mjs, opens the copy, writes to stdout one line of JSON \{"hasButton": <program\(\)\.getSourceFile\(path\.join\(project\.root, "src\/components\/button\.tsx"\)\) !== undefined>\}, and closes the project$/,
  function (this: ReadWorld) {
    const folder = mkdtempSync(join(tmpdir(), "veneer-child-"));
    (this.folders ??= []).push(folder);
    this.script = join(folder, "child.mjs");
    writeFileSync(
      this.script,
      `import path from "node:path";
import { openProject } from ${JSON.stringify(join(pkg, "dist/index.mjs"))};
const project = await openProject(${JSON.stringify(fixture(this))});
const hasButton = project.program().getSourceFile(path.join(project.root, "src/components/button.tsx")) !== undefined;
process.stdout.write(JSON.stringify({ hasButton }) + "\\n");
project.close();
`,
    );
  },
);

When(/^node runs the child script$/, { timeout: STEP_TIMEOUT_MS }, function (this: ReadWorld) {
  assert.ok(this.script, "no child script");
  const { status, stdout, stderr } = spawnSync(process.execPath, [this.script], {
    encoding: "utf8",
    timeout: STEP_TIMEOUT_MS,
  });
  this.run = { status, stdout, stderr };
});

Then(/^its stdout is the line (\{.*\})$/, function (this: ReadWorld, line: string) {
  assert.equal(this.run?.stdout, `${line}\n`);
});

Then(/^its stderr is empty$/, function (this: ReadWorld) {
  assert.equal(this.run?.stderr, "");
});

Then(
  /^program\(\)\.getSourceFile\(path\.join\(project\.root, "src\/components\/ui\/card\.astro"\)\) and program\(\)\.getSourceFile\(path\.join\(project\.root, "src\/components\/button\.tsx"\)\) are both defined$/,
  function (this: ReadWorld) {
    const open = project(this);
    for (const file of ["src/components/ui/card.astro", "src/components/button.tsx"]) {
      assert.ok(
        open.program().getSourceFile(join(open.root, file)),
        `${file} is not in the program`,
      );
    }
  },
);

Then(
  /^diagnostics\("src\/components\/button\.tsx"\) is two diagnostics of code "([^"]+)" and severity "([^"]+)", the first at (line \d+, column \d+, endLine \d+, endColumn \d+), the second at (line \d+, column \d+, endLine \d+, endColumn \d+)$/,
  { timeout: STEP_TIMEOUT_MS },
  async function (this: ReadWorld, code: string, severity: string, first: string, second: string) {
    const found = await project(this).diagnostics("src/components/button.tsx");
    assert.equal(found.length, 2, JSON.stringify(found));
    for (const diagnostic of found) {
      assert.equal(diagnostic.code, code);
      assert.equal(diagnostic.severity, severity);
    }
    assertAt(found[0], first);
    assertAt(found[1], second);
  },
);

Then(
  /^it rejects with a ProjectError of code "([^"]+)" and the message "([^"]+)"$/,
  async function (this: ReadWorld, code: string, message: string) {
    const { ProjectError } = await veneer();
    assert.ok(this.rejection instanceof ProjectError, String(this.rejection));
    assert.equal(this.rejection.code, code);
    assert.equal(this.rejection.message, message);
  },
);

Then(
  /^after the first project is closed, openProject on the no-Astro fixture resolves$/,
  { timeout: STEP_TIMEOUT_MS },
  async function (this: ReadWorld) {
    assert.ok(this.noAstro, "no no-Astro copy");
    project(this).close();
    const { openProject } = await veneer();
    this.project = await openProject(this.noAstro);
  },
);

Then(
  /^ts\.sys\.readFile, getModifiedTime and fileExists are the noted functions$/,
  function (this: ReadWorld) {
    const { sys } = veneerRequire("typescript") as typeof import("typescript");
    assert.ok(this.sys, "ts.sys was not noted");
    assert.equal(sys.readFile, this.sys.readFile);
    assert.equal(sys.getModifiedTime, this.sys.getModifiedTime);
    assert.equal(sys.fileExists, this.sys.fileExists);
  },
);

Then(
  /^diagnostics\("src\/pages\/index\.astro"\) rejects with a ProjectError of code "([^"]+)" and the message "([^"]+)"$/,
  async function (this: ReadWorld, code: string, message: string) {
    const { ProjectError } = await veneer();
    await assert.rejects(project(this).diagnostics(PAGE), (error: unknown) => {
      assert.ok(error instanceof ProjectError);
      assert.equal(error.code, code);
      assert.equal(error.message, message);
      return true;
    });
  },
);
