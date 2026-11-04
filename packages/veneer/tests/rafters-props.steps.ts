import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { cpSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { createInterface } from "node:readline";
import { Given, Then, When } from "@cucumber/cucumber";
import type { CheckResult, LintDiagnostic } from "@rafters/veneer";
import { INSTALL_TIMEOUT_MS, installOnce, pkg, type DocsWorld } from "./support/docs-fixture.ts";

const CHILD_MS = 60 * 1000;
const INDEX = join(pkg, "dist/index.mjs");
const WORKER = join(pkg, "dist/worker.mjs");
const TSX_PAGE = "src/pages/fill.tsx";
const ASTRO_PAGE = "src/pages/fill.astro";
const PAGES = [TSX_PAGE, ASTRO_PAGE];

// FR-VEN-064's After removes the folders.
export interface PropsWorld extends DocsWorld {
  propsCopy?: string;
  pagesBefore?: Buffer[];
  propsChecked?: LintDiagnostic[];
  // What FR-VEN-066's "it reports nothing" reads.
  found?: LintDiagnostic[];
  modulesLoaded?: string[];
  workerAnswer?: LintDiagnostic[];
  // The files veneer's check runs on, when a scenario names others than the two pages.
  checkPaths?: string[];
  dist?: string;
  assertion?: { status: number | null; stderr: string };
}

export function copyOf(world: PropsWorld): string {
  assert.ok(world.propsCopy, "the project is not set up in this scenario");
  return world.propsCopy;
}

export const place = (world: PropsWorld, file: string, text: string): void => {
  const at = join(copyOf(world), file);
  mkdirSync(dirname(at), { recursive: true });
  writeFileSync(at, text);
};

export function project(world: PropsWorld, tsxPage: string, astroPage: string): void {
  const folder = mkdtempSync(join(tmpdir(), "veneer-props-"));
  (world.folders ??= []).push(folder);
  cpSync(join(pkg, "test/fixtures/lint"), folder, { recursive: true });
  cpSync(join(pkg, "test/fixtures/fill-tokens"), join(folder, ".rafters/tokens"), {
    recursive: true,
  });
  installOnce(world, folder);
  world.propsCopy = folder;
  place(
    world,
    "src/components/ui/surface.tsx",
    "export function Surface(props: { fill?: string; surface?: string }) {\n  return <div>{props.fill}</div>;\n}\n",
  );
  place(
    world,
    "src/components/ui/surface-astro.astro",
    "---\ninterface Props { fill?: string; surface?: string }\n---\n<div><slot /></div>\n",
  );
  place(world, TSX_PAGE, tsxPage);
  place(world, ASTRO_PAGE, astroPage);
  world.pagesBefore = PAGES.map((page) => readFileSync(join(folder, page)));
}

const tsxImport = "import { Surface } from '@/components/ui/surface';\n";
const astroImport = "---\nimport Surface from '@/components/ui/surface-astro.astro';\n---\n";

const raftersPages = (world: PropsWorld, uses: string): void => {
  project(
    world,
    `${tsxImport}export const Page = () => <main>${uses}</main>;\n`,
    `${astroImport}<main>${uses}</main>\n`,
  );
};

Given(
  /^a project whose source has fill="(.*)" on a rafters component(?:, a .+)?$/,
  { timeout: INSTALL_TIMEOUT_MS },
  function (this: PropsWorld, value: string) {
    raftersPages(this, `<Surface fill="${value}" />`);
  },
);

Given(
  /^a project whose source has fill="(.*)" and fill="(.*)" on rafters components$/,
  { timeout: INSTALL_TIMEOUT_MS },
  function (this: PropsWorld, first: string, second: string) {
    raftersPages(this, `<Surface fill="${first}" /><Surface fill="${second}" />`);
  },
);

Given(
  /^a project whose source has fill="(.*)" on its own component$/,
  { timeout: INSTALL_TIMEOUT_MS },
  function (this: PropsWorld, value: string) {
    const own = "function Own(props: { fill?: string }) {\n  return <div>{props.fill}</div>;\n}\n";
    project(
      this,
      `${own}export const Page = () => <Own fill="${value}" />;\n`,
      `---\nimport Own from '../widgets/own.astro';\n---\n<Own fill="${value}" />\n`,
    );
    place(this, "src/widgets/own.astro", "---\ninterface Props { fill?: string }\n---\n<div />\n");
  },
);

Given(
  /^the project's tokens hold surface-foreground in the namespace semantic$/,
  function (this: PropsWorld) {
    const semantic = JSON.parse(
      readFileSync(join(copyOf(this), ".rafters/tokens/semantic.rafters.json"), "utf8"),
    );
    assert.equal(semantic.tokens[0].name, "surface-foreground");
    assert.equal(semantic.tokens[0].namespace, "semantic");
  },
);

Given(/^the project's tokens cannot be loaded$/, function (this: PropsWorld) {
  place(this, ".rafters/tokens/fill.rafters.json", "{ not json");
});

When(/^veneer's check runs$/, { timeout: CHILD_MS }, function (this: PropsWorld) {
  // The hook goes in before veneer is imported, so it sees every module veneer loads.
  const script = `import { registerHooks } from "node:module";
const loaded = [];
registerHooks({
  load(url, context, nextLoad) {
    loaded.push(url);
    return nextLoad(url, context);
  },
});
const { check } = await import(${JSON.stringify(INDEX)});
const result = await check(${JSON.stringify(copyOf(this))}, { paths: ${JSON.stringify(this.checkPaths ?? PAGES)} });
process.stdout.write(JSON.stringify({ result, loaded }));
`;
  const { status, stdout, stderr } = spawnSync(
    process.execPath,
    ["--input-type=module", "--eval", script],
    { encoding: "utf8", timeout: CHILD_MS },
  );
  assert.equal(status, 0, stderr);
  const { result, loaded }: { result: CheckResult; loaded: string[] } = JSON.parse(stdout);
  assert.ok(result.ok, stdout);
  this.propsChecked = result.diagnostics;
  this.found = result.diagnostics;
  this.modulesLoaded = loaded;
});

Then(/^the tokens code (was|was not) loaded$/, function (this: PropsWorld, answer: string) {
  assert.ok(this.modulesLoaded, "check has not run");
  const tokens = this.modulesLoaded.filter((url) => /\/dist\/tokens-[^/]+\.mjs$/.test(url));
  assert.equal(tokens.length > 0, answer === "was", JSON.stringify(tokens));
});

export function checked(world: PropsWorld): LintDiagnostic[] {
  assert.ok(world.propsChecked, "check has not run");
  return world.propsChecked;
}

Then(/^it flags fill="(.*)" with (.+)$/, function (this: PropsWorld, value: string, help: string) {
  for (const page of PAGES) {
    const flagged = checked(this).filter(
      (found) => found.file === page && found.message === `fill="${value}" is a retired fill value`,
    );
    assert.equal(flagged.length, 1, `${page}: ${JSON.stringify(checked(this))}`);
    const [found] = flagged;
    assert.equal(found?.code, "rafters(retired-fill)");
    assert.equal(found?.severity, "error");
    assert.equal(found?.help, help);
  }
});

Then(/^the source file is unchanged$/, function (this: PropsWorld) {
  assert.deepEqual(
    PAGES.map((page) => readFileSync(join(copyOf(this), page))),
    this.pagesBefore,
  );
});

Then(/^it reports nothing for that value$/, function (this: PropsWorld) {
  assert.deepEqual(checked(this), []);
});

Then(
  /^fill="(.*)" says the tokens could not be loaded and names no replacement$/,
  function (this: PropsWorld, value: string) {
    const flagged = checked(this).filter((found) => found.message.includes(`fill="${value}"`));
    assert.equal(flagged.length, PAGES.length, JSON.stringify(checked(this)));
    for (const found of flagged) {
      assert.match(String(found.help), /tokens could not be loaded/);
      assert.doesNotMatch(String(found.help), /surface=|bg-fill-/);
    }
  },
);

When(
  /^the language server's worker reads (the same page|\S+)$/,
  { timeout: CHILD_MS },
  async function (this: PropsWorld, named: string) {
    const file = named === "the same page" ? TSX_PAGE : named;
    const worker = spawn(process.execPath, [WORKER, copyOf(this)], {
      stdio: ["pipe", "pipe", "inherit"],
    });
    try {
      const text = readFileSync(join(copyOf(this), file), "utf8");
      for await (const raw of createInterface({ input: worker.stdout })) {
        const line: { ready?: boolean; seq?: number; diagnostics?: LintDiagnostic[] } =
          JSON.parse(raw);
        if (line.ready === true) {
          worker.stdin.write(`${JSON.stringify({ seq: 1, file, text })}\n`);
        } else {
          assert.equal(line.seq, 1, raw);
          this.workerAnswer = line.diagnostics;
          break;
        }
      }
    } finally {
      worker.kill();
    }
  },
);

Then(/^the worker's diagnostics equal check's$/, function (this: PropsWorld) {
  const fromCheck = checked(this).filter((found) => found.file === TSX_PAGE);
  assert.equal(fromCheck.length, 1, JSON.stringify(fromCheck));
  assert.deepEqual(this.workerAnswer, fromCheck);
});

const distFiles = (extension: string): string[] =>
  readdirSync(join(pkg, "dist")).filter((file) => file.endsWith(extension));

Then(
  /^veneer's dist imports no rafters source and no token schema package$/,
  function (this: PropsWorld) {
    const specifier =
      /(?:from|import)\s*\(?\s*"(rafters-monorepo|@rafters\/[^"]*|zod|colorjs\.io|apca-w3|comment-parser)[/"]/;
    for (const file of distFiles(".mjs")) {
      assert.doesNotMatch(readFileSync(join(pkg, "dist", file), "utf8"), specifier, file);
    }
  },
);

Then(/^veneer's published types do not name TokenRegistry$/, function (this: PropsWorld) {
  for (const file of distFiles(".d.mts")) {
    assert.doesNotMatch(readFileSync(join(pkg, "dist", file), "utf8"), /TokenRegistry/, file);
  }
});

Given(/^a dist folder holding the text (\S+)$/, function (this: PropsWorld, name: string) {
  const folder = mkdtempSync(join(tmpdir(), "veneer-dist-"));
  (this.folders ??= []).push(folder);
  mkdirSync(join(folder, "nested"));
  writeFileSync(join(folder, "nested/chunk.mjs"), `export const held = "${name}";\n`);
  this.dist = folder;
});

When(/^the build's dist assertion runs on it$/, function (this: PropsWorld) {
  assert.ok(this.dist, "no dist folder");
  const { status, stderr } = spawnSync(
    process.execPath,
    [join(pkg, "scripts/assert-dist.ts"), this.dist],
    { encoding: "utf8" },
  );
  this.assertion = { status, stderr };
});

Then(/^it fails naming (\S+)$/, function (this: PropsWorld, name: string) {
  assert.equal(this.assertion?.status, 1, this.assertion?.stderr);
  assert.ok(this.assertion?.stderr.includes(name), this.assertion?.stderr);
});
