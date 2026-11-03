import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdtempSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Given, Then, When } from "@cucumber/cucumber";
import {
  CONFIG,
  INSTALL_TIMEOUT_MS,
  installOnce,
  pkg,
  type DocsWorld,
} from "./support/docs-fixture.ts";

const RUN_MS = 60 * 1000;
const BRACKET_VALUE = '<div class="p-[1px]">a</div>\n';
const CLEAN = "<div>a</div>\n";
const FINDING_LINE = /^(\S+):\d+:\d+ (?:error|warning|info|hint) \S+ .+$/;
const INDEX = join(pkg, "dist/index.mjs");
const VP = join(pkg, "node_modules/.bin/vp");
const manifest: { bin: { "rafters-check": string } } = JSON.parse(
  readFileSync(join(pkg, "package.json"), "utf8"),
);
const BIN = join(pkg, manifest.bin["rafters-check"]);

interface CommandWorld extends DocsWorld {
  // The lint fixture's Given sets this; the other Givens here do too.
  checkCopy?: string;
  before?: Map<string, string>;
  command?: { status: number | null; stdout: string; stderr: string };
}

function tempFolder(world: CommandWorld): string {
  const folder = mkdtempSync(join(tmpdir(), "veneer-command-"));
  (world.folders ??= []).push(folder);
  return folder;
}

function rootOf(world: CommandWorld): string {
  assert.ok(world.checkCopy, "no project is set up in this scenario");
  return world.checkCopy;
}

function commandOf(world: CommandWorld): NonNullable<CommandWorld["command"]> {
  assert.ok(world.command, "the command has not run");
  return world.command;
}

function run(
  world: CommandWorld,
  program: string,
  args: string[],
  cwd: string,
  input?: string,
): void {
  const { status, stdout, stderr } = spawnSync(program, args, {
    cwd,
    encoding: "utf8",
    timeout: RUN_MS,
    input,
  });
  world.command = { status, stdout, stderr };
}

// Every file outside node_modules, with its bytes.
function snapshot(root: string): Map<string, string> {
  const files = readdirSync(root, { recursive: true, withFileTypes: true })
    .filter((entry) => entry.isFile())
    .map((entry) => join(entry.parentPath, entry.name))
    .filter((file) => !file.includes("/node_modules/"));
  return new Map(files.map((file) => [file, readFileSync(file, "base64")]));
}

Given(
  /^a project with no findings$/,
  { timeout: INSTALL_TIMEOUT_MS },
  function (this: CommandWorld) {
    const folder = tempFolder(this);
    cpSync(join(pkg, "test/fixtures/read"), folder, { recursive: true });
    installOnce(this, folder);
    this.checkCopy = folder;
  },
);

Given(/^a folder with no rafters config$/, function (this: CommandWorld) {
  this.checkCopy = tempFolder(this);
});

Given(/^its componentsPath names a folder that does not exist$/, function (this: CommandWorld) {
  const file = join(rootOf(this), CONFIG);
  const config: object = JSON.parse(readFileSync(file, "utf8"));
  writeFileSync(file, JSON.stringify({ ...config, componentsPath: "src/nowhere" }));
});

Given(
  /^the lint fixture with the packed package installed$/,
  { timeout: INSTALL_TIMEOUT_MS },
  function (this: CommandWorld) {
    const tarballs = tempFolder(this);
    const env = { ...process.env, pnpm_config_strict_dep_builds: "false" };
    const pack = spawnSync("pnpm", ["pack", "--pack-destination", tarballs], {
      cwd: pkg,
      encoding: "utf8",
      env,
    });
    assert.equal(pack.status, 0, `${pack.stdout}\n${pack.stderr}`);
    const tarball = readdirSync(tarballs).find((file) => file.endsWith(".tgz"));
    assert.ok(tarball, "pnpm pack wrote no tarball");
    const folder = tempFolder(this);
    cpSync(join(pkg, "test/fixtures/lint"), folder, { recursive: true });
    const add = spawnSync("pnpm", ["add", "-D", `file:${join(tarballs, tarball)}`], {
      cwd: folder,
      encoding: "utf8",
      env,
    });
    assert.equal(add.status, 0, `${add.stdout}\n${add.stderr}`);
    this.checkCopy = folder;
  },
);

When(/^rafters-check runs in its root$/, { timeout: RUN_MS }, function (this: CommandWorld) {
  const root = rootOf(this);
  this.before = snapshot(root);
  run(this, BIN, [], root);
});

When(
  /^rafters-check runs elsewhere with its root as the argument$/,
  { timeout: RUN_MS },
  function (this: CommandWorld) {
    run(this, BIN, [rootOf(this)], tempFolder(this));
  },
);

When(/^rafters-check runs with the arguments (.+)$/, function (this: CommandWorld, args: string) {
  run(this, BIN, args.split(" "), tempFolder(this));
});

When(
  /^vp exec rafters-check runs in its root$/,
  { timeout: RUN_MS },
  function (this: CommandWorld) {
    run(this, VP, ["exec", "rafters-check"], rootOf(this));
  },
);

When(
  /^rafters-check runs with (none|--nope)$/,
  { timeout: RUN_MS },
  function (this: CommandWorld, args: string) {
    run(this, BIN, args === "none" ? [] : [args], rootOf(this));
  },
);

function runStdin(world: CommandWorld, file: string, input: string): void {
  const root = rootOf(world);
  world.before = snapshot(root);
  run(world, BIN, ["--stdin", file], root, input);
}

When(
  /^rafters-check runs with --stdin (\S+) and a bracket value on stdin$/,
  { timeout: RUN_MS },
  function (this: CommandWorld, file: string) {
    runStdin(this, file, BRACKET_VALUE);
  },
);

When(
  /^rafters-check runs with --stdin on a file with a finding, and clean content on stdin$/,
  { timeout: RUN_MS },
  function (this: CommandWorld) {
    runStdin(this, "src/pages/brackets.astro", CLEAN);
  },
);

When(
  /^rafters-check runs with --stdin (\.\.\/\S+)$/,
  { timeout: RUN_MS },
  function (this: CommandWorld, file: string) {
    runStdin(this, file, CLEAN);
  },
);

When(
  /^rafters-check runs with --stdin on a \.ts path the tsconfig takes$/,
  { timeout: RUN_MS },
  function (this: CommandWorld) {
    runStdin(this, "src/new.ts", "export const a = 1;\n");
  },
);

When(
  /^rafters-check runs elsewhere with its root and --stdin on the absolute path of (\S+)$/,
  { timeout: RUN_MS },
  function (this: CommandWorld, file: string) {
    const root = rootOf(this);
    run(this, BIN, [root, "--stdin", join(root, file)], tempFolder(this), BRACKET_VALUE);
  },
);

When(
  /^rafters-check runs in (\S+) with its root and --stdin (\S+)$/,
  { timeout: RUN_MS },
  function (this: CommandWorld, folder: string, file: string) {
    const root = rootOf(this);
    run(this, BIN, [root, "--stdin", file], join(root, folder), BRACKET_VALUE);
  },
);

const reportedFiles = (world: CommandWorld): string[] =>
  commandOf(world)
    .stdout.split("\n")
    .flatMap((line) => FINDING_LINE.exec(line)?.[1] ?? []);

Then(/^the report names (\S+\/\S+)$/, function (this: CommandWorld, file: string) {
  assert.ok(reportedFiles(this).includes(file), commandOf(this).stdout);
});

Then(/^the report names no other file$/, function (this: CommandWorld) {
  assert.equal(new Set(reportedFiles(this)).size, 1, commandOf(this).stdout);
});

Then(/^the report names no file$/, function (this: CommandWorld) {
  assert.deepEqual(reportedFiles(this), [], commandOf(this).stdout);
});

Then(/^(src\/\S+) does not exist$/, function (this: CommandWorld, file: string) {
  assert.equal(existsSync(join(rootOf(this), file)), false);
});

Then(
  /^a line of stdout matches file:line:column severity code message for (\S+)$/,
  function (this: CommandWorld, file: string) {
    assert.ok(reportedFiles(this).includes(file), commandOf(this).stdout);
  },
);

Then(/^the report is on stdout$/, { timeout: RUN_MS }, function (this: CommandWorld) {
  const script = `import { check } from ${JSON.stringify(INDEX)};
const result = await check(${JSON.stringify(rootOf(this))});
process.stdout.write(result.report);
`;
  const expected = spawnSync(process.execPath, ["--input-type=module", "--eval", script], {
    encoding: "utf8",
    timeout: RUN_MS,
  });
  assert.equal(expected.status, 0, expected.stderr);
  assert.ok(expected.stdout.length > 0);
  assert.equal(commandOf(this).stdout, expected.stdout);
});

Then(/^the exit code is (\d)$/, function (this: CommandWorld, code: string) {
  const { status, stderr } = commandOf(this);
  assert.equal(status, Number(code), stderr);
});

Then(/^stderr holds "(.*)"$/, function (this: CommandWorld, text: string) {
  assert.ok(commandOf(this).stderr.includes(text), commandOf(this).stderr);
});

Then(/^stdout is empty$/, function (this: CommandWorld) {
  assert.equal(commandOf(this).stdout, "");
});

Then(/^the project outside node_modules is unchanged$/, function (this: CommandWorld) {
  assert.ok(this.before, "the command has not run");
  assert.deepEqual(snapshot(rootOf(this)), this.before);
});
