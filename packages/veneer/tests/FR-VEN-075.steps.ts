import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { After, Before, Given, Then, When } from "@cucumber/cucumber";

const root = fileURLToPath(new URL("../../../", import.meta.url));
const pkg = join(root, "packages/veneer");
const COMMAND_TIMEOUT_MS = 5 * 60 * 1000;

interface Run {
  status: number | null;
  stdout: string;
  stderr: string;
}

interface ScenarioWorld {
  run?: Run;
  folder?: string;
  members?: string[];
}

const IN_PROBE = "VENEER_UNTAGGED_PROBE";

function run(command: string, args: string[], env: NodeJS.ProcessEnv = process.env): Run {
  const { status, stdout, stderr } = spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
    env,
    timeout: COMMAND_TIMEOUT_MS,
  });
  return { status, stdout, stderr };
}

function lastRun(world: ScenarioWorld): Run {
  assert.ok(world.run, "no command has run in this scenario");
  return world.run;
}

function memberNames(metadata: unknown): string[] {
  assert.ok(metadata && typeof metadata === "object" && "packages" in metadata);
  const { packages } = metadata;
  assert.ok(Array.isArray(packages));
  return packages.map((entry: unknown) => {
    assert.ok(entry && typeof entry === "object" && "name" in entry);
    assert.equal(typeof entry.name, "string");
    return String(entry.name);
  });
}

// Cucumber adds cucumber.mjs's paths and imports to the command line's, so the probe's child run
// also holds every real scenario, this one included. There, only the untagged probe may run.
Before(function ({ pickle }) {
  const tagged = pickle.tags.some((tag) => tag.name.startsWith("@criterion-"));
  return process.env[IN_PROBE] && tagged ? "skipped" : undefined;
});

After(function (this: ScenarioWorld) {
  if (this.folder) rmSync(this.folder, { recursive: true, force: true });
});

Given(
  /^the workspace after pnpm install --frozen-lockfile$/,
  { timeout: COMMAND_TIMEOUT_MS },
  function () {
    // An earlier build leaves dist behind; only the build under test may satisfy the scenario.
    rmSync(join(pkg, "dist"), { recursive: true, force: true });
    const install = run("pnpm", ["install", "--frozen-lockfile"]);
    assert.equal(install.status, 0, install.stderr);
  },
);

When(/^pnpm -r build runs$/, { timeout: COMMAND_TIMEOUT_MS }, function (this: ScenarioWorld) {
  this.run = run("pnpm", ["-r", "build"]);
});

Then(/^it exits with code 0$/, function (this: ScenarioWorld) {
  const { status, stdout, stderr } = lastRun(this);
  assert.equal(status, 0, `${stdout}\n${stderr}`);
});

Then(/^packages\/veneer\/dist\/index\.mjs and packages\/veneer\/dist\/index\.d\.mts exist$/, () => {
  assert.ok(existsSync(join(pkg, "dist/index.mjs")), "dist/index.mjs is missing");
  assert.ok(existsSync(join(pkg, "dist/index.d.mts")), "dist/index.d.mts is missing");
});

Then(/^packages\/veneer holds no \.node file$/, () => {
  const found = readdirSync(pkg, { recursive: true, encoding: "utf8" }).filter(
    (name) => name.endsWith(".node") && !name.startsWith("node_modules"),
  );
  assert.deepEqual(found, []);
});

Given(
  /^a temporary folder holding untagged\.feature, one scenario named "untagged probe" with no tags and the single step "Given a step that passes", and pass\.steps\.ts defining Given\("a step that passes", \(\) => \{\}\)$/,
  function (this: ScenarioWorld) {
    this.folder = mkdtempSync(join(tmpdir(), "veneer-untagged-"));
    writeFileSync(
      join(this.folder, "untagged.feature"),
      "Feature: probe\n\n  Scenario: untagged probe\n    Given a step that passes\n",
    );
    // The folder sits outside the workspace, so a bare specifier would not resolve from it.
    const cucumber = import.meta.resolve("@cucumber/cucumber");
    writeFileSync(
      join(this.folder, "pass.steps.ts"),
      `import { Given } from ${JSON.stringify(cucumber)};\nGiven("a step that passes", () => {});\n`,
    );
  },
);

When(
  /^pnpm exec cucumber-js <folder>\/untagged\.feature --import packages\/veneer\/tests\/support\/criterion-tag\.steps\.ts --import <folder>\/pass\.steps\.ts --strict runs from the repository root$/,
  { timeout: COMMAND_TIMEOUT_MS },
  function (this: ScenarioWorld) {
    assert.ok(this.folder, "no temporary folder");
    this.run = run(
      "pnpm",
      [
        "exec",
        "cucumber-js",
        join(this.folder, "untagged.feature"),
        "--import",
        "packages/veneer/tests/support/criterion-tag.steps.ts",
        "--import",
        join(this.folder, "pass.steps.ts"),
        "--strict",
      ],
      { ...process.env, [IN_PROBE]: "1" },
    );
  },
);

Then(/^it exits with a non-zero code$/, function (this: ScenarioWorld) {
  const { status } = lastRun(this);
  assert.notEqual(status, null, "the command did not exit");
  assert.notEqual(status, 0);
});

Then(/^its stdout names "untagged probe"$/, function (this: ScenarioWorld) {
  assert.match(lastRun(this).stdout, /untagged probe/);
});

Given(/^the repository at the build's head$/, () => {
  assert.ok(existsSync(join(root, "Cargo.toml")));
});

When(
  /^cargo metadata --no-deps --format-version 1 runs at the root$/,
  { timeout: COMMAND_TIMEOUT_MS },
  function (this: ScenarioWorld) {
    const metadata = run("cargo", ["metadata", "--no-deps", "--format-version", "1"]);
    assert.equal(metadata.status, 0, metadata.stderr);
    this.members = memberNames(JSON.parse(metadata.stdout)).sort();
  },
);

Then(
  /^its workspace members are exactly veneer-check and veneer-lsp$/,
  function (this: ScenarioWorld) {
    assert.deepEqual(this.members, ["veneer-check", "veneer-lsp"]);
  },
);

Then(
  /^crates\/veneer, crates\/veneer-adapters, crates\/veneer-node and install\.sh do not exist$/,
  () => {
    for (const path of [
      "crates/veneer",
      "crates/veneer-adapters",
      "crates/veneer-node",
      "install.sh",
    ]) {
      assert.ok(!existsSync(join(root, path)), `${path} still exists`);
    }
  },
);

Then(/^no file under \.github\/workflows names install\.sh$/, () => {
  const workflows = join(root, ".github/workflows");
  for (const name of readdirSync(workflows)) {
    const text = readFileSync(join(workflows, name), "utf8");
    assert.ok(!text.includes("install.sh"), `${name} names install.sh`);
  }
});
