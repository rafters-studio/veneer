import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import fsPromises from "node:fs/promises";
import { syncBuiltinESMExports } from "node:module";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { After, Given, Then, When } from "@cucumber/cucumber";
import {
  DOCS,
  GOLDEN,
  INSTALL_TIMEOUT_MS,
  componentLines,
  copyDocsFixture,
  docsCopy,
  docsLines,
  errorsOf,
  installOnce,
  okResult,
  pkg,
  write,
  type DocsWorld,
} from "./support/docs-fixture.ts";

const CHILD_MS = 60 * 1000;
const EXIT_WITHIN_MS = 30 * 1000;

function compareCodePoints(a: string, b: string): number {
  const [left, right] = [[...a], [...b]];
  for (let i = 0; i < Math.min(left.length, right.length); i += 1) {
    const difference = (left[i]?.codePointAt(0) ?? 0) - (right[i]?.codePointAt(0) ?? 0);
    if (difference !== 0) return difference;
  }
  return left.length - right.length;
}

// A closed Project leaves memory behind, and one process running every scenario runs out of
// heap. So only the scenario that wraps rename, which must share a process with writeDocs,
// runs in this one.
async function runDocs(world: DocsWorld): Promise<void> {
  const folder = docsCopy(world);
  installOnce(world, folder);
  if (world.renames) {
    const { writeDocs } = await import("@rafters/veneer");
    world.result = await writeDocs(folder);
    return;
  }
  const script = `import { writeDocs } from ${JSON.stringify(join(pkg, "dist/index.mjs"))};
process.stdout.write(JSON.stringify(await writeDocs(${JSON.stringify(folder)})));
`;
  const { status, stdout, stderr } = spawnSync(
    process.execPath,
    ["--input-type=module", "--eval", script],
    { encoding: "utf8", timeout: CHILD_MS },
  );
  assert.equal(status, 0, stderr);
  world.result = JSON.parse(stdout);
}

function outsideFolder(world: DocsWorld): string {
  const folder = mkdtempSync(join(tmpdir(), "veneer-outside-"));
  (world.folders ??= []).push(folder);
  return folder;
}

function runChild(folder: string): Promise<NonNullable<DocsWorld["children"]>[number]> {
  const script = `import { writeDocs } from ${JSON.stringify(join(pkg, "dist/index.mjs"))};
await writeDocs(${JSON.stringify(folder)});
process.stdout.write("resolved\\n");
`;
  return new Promise((done, fail) => {
    const child = spawn(process.execPath, ["--input-type=module", "--eval", script], {
      stdio: ["ignore", "pipe", "inherit"],
    });
    const killer = setTimeout(() => child.kill("SIGKILL"), CHILD_MS);
    let output = "";
    let resolvedAt: number | undefined;
    child.stdout.on("data", (chunk: Buffer) => {
      output += chunk.toString();
      if (output.includes("resolved\n")) resolvedAt ??= Date.now();
    });
    child.on("error", fail);
    child.on("close", (status) => {
      clearTimeout(killer);
      done({
        status,
        exitedAfterMs: Date.now() - (resolvedAt ?? Date.now()),
        sawResolved: resolvedAt !== undefined,
      });
    });
  });
}

After(function (this: DocsWorld) {
  this.restoreRename?.();
});

Given(/^the docs fixture$/, function (this: DocsWorld) {
  docsCopy(this);
});

Given(/^the docs fixture, whose config's intent is "efficient"$/, function (this: DocsWorld) {
  docsCopy(this);
});

Given(
  /^the docs fixture with an earlier \.rafters\/veneer\/docs\.jsonl holding the single line (\{.*\})$/,
  function (this: DocsWorld, line: string) {
    write(docsCopy(this), DOCS, `${line}\n`);
  },
);

Given(
  /^the test has wrapped rename on node:fs\/promises to record its calls and called module\.syncBuiltinESMExports\(\)$/,
  function (this: DocsWorld) {
    const { rename } = fsPromises;
    const renames: [string, string][] = [];
    this.renames = renames;
    fsPromises.rename = (from, to) => {
      renames.push([String(from), String(to)]);
      return rename(from, to);
    };
    syncBuiltinESMExports();
    this.restoreRename = () => {
      fsPromises.rename = rename;
      syncBuiltinESMExports();
    };
  },
);

Given(
  /^the docs fixture with an earlier docs\.jsonl, and its config's veneer block set to (\{.*\})$/,
  function (this: DocsWorld, block: string) {
    const folder = docsCopy(this);
    this.earlier = Buffer.from('{"kind":"project","intent":"old"}\n');
    write(folder, DOCS, this.earlier.toString());
    const file = join(folder, ".rafters/config.rafters.json");
    const config: Record<string, unknown> = JSON.parse(readFileSync(file, "utf8"));
    writeFileSync(file, JSON.stringify({ ...config, veneer: JSON.parse(block) }));
  },
);

Given(
  /^a second copy of it whose tsconfig\.json has "include": \[\] and a files list naming every \.ts, \.tsx and \.astro file under src, each written as src\/components\/<name>, in reverse code point order$/,
  function (this: DocsWorld) {
    const folder = copyDocsFixture(this);
    this.secondCopy = folder;
    const file = join(folder, "tsconfig.json");
    const tsconfig: Record<string, unknown> = JSON.parse(readFileSync(file, "utf8"));
    const files = readdirSync(join(folder, "src/components"))
      .map((name) => `src/components/${name}`)
      .sort(compareCodePoints)
      .reverse();
    writeFileSync(file, JSON.stringify({ ...tsconfig, include: [], files }));
  },
);

Given(
  /^the docs fixture with its tsconfig\.json include changed to (\[.*\])$/,
  function (this: DocsWorld, include: string) {
    const file = join(docsCopy(this), "tsconfig.json");
    const tsconfig: Record<string, unknown> = JSON.parse(readFileSync(file, "utf8"));
    writeFileSync(file, JSON.stringify({ ...tsconfig, include: JSON.parse(include) }));
  },
);

Given(/^the docs fixture with its tsconfig\.json deleted$/, function (this: DocsWorld) {
  rmSync(join(docsCopy(this), "tsconfig.json"));
});

Given(
  /^the docs fixture, with \.rafters\/veneer replaced by a symbolic link to a folder outside the copy$/,
  function (this: DocsWorld) {
    const link = join(docsCopy(this), ".rafters/veneer");
    rmSync(link, { recursive: true, force: true });
    this.outside = outsideFolder(this);
    symlinkSync(this.outside, link);
  },
);

Given(
  /^the docs fixture, with \.rafters\/veneer\/docs\.jsonl\.tmp a symbolic link to a file outside the copy holding the text "([^"]+)"$/,
  function (this: DocsWorld, text: string) {
    const folder = docsCopy(this);
    this.outside = join(outsideFolder(this), "outside.txt");
    writeFileSync(this.outside, text);
    mkdirSync(join(folder, ".rafters/veneer"), { recursive: true });
    symlinkSync(this.outside, join(folder, `${DOCS}.tmp`));
  },
);

When(
  /^(?:writeDocs runs on it(?:, called with the copy's absolute path)?|veneer writes the docs)$/,
  { timeout: INSTALL_TIMEOUT_MS },
  async function (this: DocsWorld) {
    await runDocs(this);
  },
);

When(
  /^writeDocs runs on each copy, each in its own fresh child process$/,
  { timeout: INSTALL_TIMEOUT_MS },
  async function (this: DocsWorld) {
    assert.ok(this.secondCopy, "no second copy");
    this.children = [];
    for (const folder of [docsCopy(this), this.secondCopy]) {
      installOnce(this, folder);
      this.children.push(await runChild(folder));
    }
  },
);

Then(/^docs\.jsonl has (\d+) component lines$/, function (this: DocsWorld, count: string) {
  assert.equal(componentLines(this).length, Number(count));
});

Then(
  /^the lines for src\/components\/wrapped\.tsx come last, Memo before Ref$/,
  function (this: DocsWorld) {
    const lines = docsLines(this).slice(-2);
    assert.deepEqual(
      lines.map(({ name, file }) => [name, file]),
      [
        ["Memo", "src/components/wrapped.tsx"],
        ["Ref", "src/components/wrapped.tsx"],
      ],
    );
  },
);

Then(
  /^the recorded calls hold one rename from <the copy's path as passed to writeDocs>\/\.rafters\/veneer\/docs\.jsonl\.tmp to <the copy's path as passed to writeDocs>\/\.rafters\/veneer\/docs\.jsonl$/,
  function (this: DocsWorld) {
    const target = join(docsCopy(this), DOCS);
    assert.deepEqual(this.renames, [[`${target}.tmp`, target]]);
  },
);

Then(
  /^docs\.jsonl equals packages\/veneer\/test\/fixtures\/docs\.expected\.jsonl byte for byte$/,
  function (this: DocsWorld) {
    assert.deepEqual(readFileSync(join(docsCopy(this), DOCS)), readFileSync(GOLDEN));
  },
);

Then(/^no docs\.jsonl\.tmp is left in \.rafters\/veneer$/, function (this: DocsWorld) {
  assert.equal(existsSync(join(docsCopy(this), `${DOCS}.tmp`)), false);
});

Then(
  /^it returns \{"ok": true, "docsPath": path\.resolve\(<the copy's absolute path>, "\.rafters\/veneer\/docs\.jsonl"\), "notices": \[\]\}$/,
  function (this: DocsWorld) {
    assert.deepEqual(this.result, {
      ok: true,
      docsPath: resolve(docsCopy(this), DOCS),
      notices: [],
    });
  },
);

Then(
  /^it returns ok false with the error code "config-field", field "veneer\.mdxPath" and value 42$/,
  function (this: DocsWorld) {
    const errors = errorsOf(this);
    assert.equal(errors.length, 1);
    assert.equal(errors[0]?.code, "config-field");
    assert.equal(errors[0]?.field, "veneer.mdxPath");
    assert.equal(errors[0]?.value, 42);
  },
);

Then(/^docs\.jsonl has the same bytes as before$/, function (this: DocsWorld) {
  assert.deepEqual(readFileSync(join(docsCopy(this), DOCS)), this.earlier);
});

Then(
  /^both docs\.jsonl files equal packages\/veneer\/test\/fixtures\/docs\.expected\.jsonl byte for byte$/,
  function (this: DocsWorld) {
    assert.ok(this.secondCopy, "no second copy");
    for (const folder of [docsCopy(this), this.secondCopy]) {
      assert.deepEqual(readFileSync(join(folder, DOCS)), readFileSync(GOLDEN));
    }
  },
);

Then(
  /^each child process writes resolved to stdout, then exits on its own with code 0 within 30 seconds of writing it$/,
  function (this: DocsWorld) {
    assert.equal(this.children?.length, 2);
    for (const { status, exitedAfterMs, sawResolved } of this.children ?? []) {
      assert.ok(sawResolved, "the child did not write resolved");
      assert.equal(status, 0);
      assert.ok(exitedAfterMs < EXIT_WITHIN_MS, `exited after ${exitedAfterMs} ms`);
    }
  },
);

Then(/^docs\.jsonl has no line whose file ends in \.astro$/, function (this: DocsWorld) {
  assert.deepEqual(
    componentLines(this).filter(({ file }) => file?.endsWith(".astro")),
    [],
  );
});

Then(/^notices is exactly (\[.*\])$/, function (this: DocsWorld, notices: string) {
  assert.deepEqual(okResult(this).notices, JSON.parse(notices));
});

Then(
  /^it returns ok false with errors \[\{"code":"tsconfig-missing","message":"tsconfig\.json not found at <the copy's absolute path>"\}\]$/,
  function (this: DocsWorld) {
    assert.deepEqual(errorsOf(this), [
      { code: "tsconfig-missing", message: `tsconfig.json not found at ${docsCopy(this)}` },
    ]);
  },
);

Then(
  /^it returns ok false with one error of code "([^"]+)"$/,
  function (this: DocsWorld, code: string) {
    assert.deepEqual(
      errorsOf(this).map((error) => error.code),
      [code],
    );
  },
);

Then(/^that outside folder is empty$/, function (this: DocsWorld) {
  assert.ok(this.outside, "no outside folder");
  assert.deepEqual(readdirSync(this.outside), []);
});

Then(/^it returns ok true$/, function (this: DocsWorld) {
  okResult(this);
});

Then(/^that outside file holds the text "([^"]+)"$/, function (this: DocsWorld, text: string) {
  assert.ok(this.outside, "no outside file");
  assert.equal(readFileSync(this.outside, "utf8"), text);
});
