import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { Given, Then, When } from "@cucumber/cucumber";
import { BIN, commandOf, run, type CommandWorld } from "./check-command.steps.ts";
import { INSTALL_TIMEOUT_MS, pkg } from "./support/docs-fixture.ts";
import { checked, copyOf, place, project, type PropsWorld } from "./rafters-props.steps.ts";

const PAGE_FILES = ["src/pages/fill.tsx", "src/pages/fill.astro"];
const CLASSY = "const classy = (...inputs: unknown[]) => inputs.join(' ');\n";

const RUN_MS = 60 * 1000;
const CLASSES_TEXT =
  'export const box = {\n  text: "text-[0.625rem]",\n  current: "aria-[current=page]:text-primary",\n};\n';

interface GuardsWorld extends PropsWorld, CommandWorld {
  expectedBrackets?: string[];
  classesFile?: string;
}

interface Page {
  tsx: string;
  astro: string;
  brackets: string[];
  // Other files the page reads, by path.
  files?: Record<string, string>;
}

const element = (attribute: string, script = ""): Pick<Page, "tsx" | "astro"> => ({
  tsx: `${script}export const Page = () => <div ${attribute} />;\n`,
  astro: `---\n${script}---\n<div ${attribute} />\n`,
});

const PAGES: Record<string, Page> = {
  'class="p-[3px]"': { ...element('class="p-[3px]"'), brackets: ["p-[3px]"] },
  'class={classy("p-[3px]", cond && "m-[1px]")}': {
    ...element(
      'class={classy("p-[3px]", cond && "m-[1px]")}',
      `${CLASSY}const cond = Date.now() > 0;\n`,
    ),
    brackets: ["p-[3px]", "m-[1px]"],
  },
  'const box = classy("p-[3px]") outside JSX': {
    ...element('class="p-2"', `${CLASSY}const box = classy("p-[3px]");\n`),
    brackets: ["p-[3px]"],
  },
  'const box = "p-[3px]" used in class={box}': {
    ...element("class={box}", 'const box = "p-[3px]";\n'),
    brackets: ["p-[3px]"],
  },
  "const box imported from the own source, used in class={box}": {
    ...element("class={box}", "import { box } from '../lib/box';\n"),
    brackets: ["p-[3px]"],
    files: { "src/lib/box.ts": 'export const box = "p-[3px]";\n' },
  },
  "const box imported from a dependency, used in class={box}": {
    ...element("class={box}", "import { box } from 'outside-const';\n"),
    brackets: [],
    files: { "node_modules/outside-const/index.d.ts": 'export declare const box = "p-[3px]";\n' },
  },
  'const box = { text: "text-[0.625rem]" } and classy(box.text)': {
    ...element("class={classy(box.text)}", `${CLASSY}const box = { text: "text-[0.625rem]" };\n`),
    brackets: ["text-[0.625rem]"],
  },
  'const box = { text: "p-[3px]" } used in class={box.text}': {
    ...element("class={box.text}", 'const box = { text: "p-[3px]" };\n'),
    brackets: ["p-[3px]"],
  },
  'const box = { text: "p-[3px]" } as const used in class={box.text}': {
    ...element("class={box.text}", 'const box = { text: "p-[3px]" } as const;\n'),
    brackets: ["p-[3px]"],
  },
  "a const object imported from the own source, used in class={box.text}": {
    ...element("class={box.text}", "import { box } from '../lib/box';\n"),
    brackets: ["p-[3px]"],
    files: { "src/lib/box.ts": 'export const box = { text: "p-[3px]" };\n' },
  },
  "a const object imported from a dependency, used in class={box.text}": {
    ...element("class={box.text}", "import { box } from 'outside-object';\n"),
    brackets: [],
    files: {
      "node_modules/outside-object/index.ts": 'export const box = { text: "p-[3px]" };\n',
    },
  },
  'let box = { text: "p-[3px]" } used in class={box.text}': {
    ...element("class={box.text}", 'let box = { text: "p-[3px]" };\n'),
    brackets: [],
  },
  'a function returning { text: "p-[3px]" }, used in class={box.text}': {
    ...element(
      "class={box.text}",
      'const make = () => ({ text: "p-[3px]" });\nconst box = make();\n',
    ),
    brackets: [],
  },
  'class="data-[state=open]:p-2"': {
    ...element('class="data-[state=open]:p-2"'),
    brackets: [],
  },
};

const raftersUse = (world: PropsWorld, use: string): void => {
  project(
    world,
    `import { Surface } from '@/components/ui/surface';\nexport const Page = () => <main><Surface ${use} /></main>;\n`,
    `---\nimport Surface from '@/components/ui/surface-astro.astro';\n---\n<main><Surface ${use} /></main>\n`,
  );
};

Given(
  /^a page with (.+)$/,
  { timeout: INSTALL_TIMEOUT_MS },
  function (this: GuardsWorld, written: string) {
    const page = PAGES[written];
    assert.ok(page, written);
    project(this, page.tsx, page.astro);
    for (const [file, text] of Object.entries(page.files ?? {})) place(this, file, text);
    this.expectedBrackets = page.brackets;
  },
);

Given(
  /^a project whose tokens hold surface-card in the namespace fill$/,
  function (this: PropsWorld) {
    const fill: { tokens: { name: string; namespace: string }[] } = JSON.parse(
      readFileSync(join(pkg, "test/fixtures/fill-tokens/fill.rafters.json"), "utf8"),
    );
    assert.ok(
      fill.tokens.some(({ name, namespace }) => name === "surface-card" && namespace === "fill"),
    );
  },
);

Given(
  /^(?:its source|a project whose source) has surface="(.*)" on a rafters component$/,
  { timeout: INSTALL_TIMEOUT_MS },
  function (this: PropsWorld, value: string) {
    raftersUse(this, `surface="${value}"`);
  },
);

Then(
  /^it reports (rafters\([a-z-]+\))(?: at severity (\w+))?$/,
  function (this: GuardsWorld, code: string, severity?: string) {
    const found = checked(this);
    for (const page of PAGE_FILES) {
      const here = found.filter((diagnostic) => diagnostic.file === page);
      assert.ok(here.length > 0, `${page}: ${JSON.stringify(found)}`);
      for (const diagnostic of here) {
        assert.equal(diagnostic.code, code);
        assert.equal(diagnostic.severity, severity ?? "error");
      }
      if (code === "rafters(arbitrary-value)") {
        assert.deepEqual(
          here.map(({ message }) => message),
          this.expectedBrackets?.map((bracket) => `${bracket} is an arbitrary value`),
        );
      }
    }
  },
);

Then(/^each finding sits on (\S+) in class=\{\1\}$/, function (this: PropsWorld, use: string) {
  for (const { file, line, column, endColumn } of checked(this)) {
    const lines = readFileSync(join(copyOf(this), file), "utf8").split("\n");
    const start = String(lines[line - 1]).indexOf(`class={${use}}`) + "class={".length;
    assert.deepEqual([column, endColumn], [start + 1, start + 1 + use.length], file);
  }
});

const classesProject = (world: GuardsWorld, file: string): void => {
  project(world, "export {};\n", "<div />\n");
  place(world, file, CLASSES_TEXT);
  world.classesFile = file;
};

Given(
  /^a project with (\S+\.classes\.ts) holding text-\[0\.625rem\] and aria-\[current=page\]:text-primary$/,
  { timeout: INSTALL_TIMEOUT_MS },
  function (this: GuardsWorld, file: string) {
    classesProject(this, file);
  },
);

Given(
  /^a project with a bracket value in a \.classes\.ts file under componentsPath$/,
  { timeout: INSTALL_TIMEOUT_MS },
  function (this: GuardsWorld) {
    classesProject(this, "src/components/ui/x.classes.ts");
  },
);

Given(
  /^a project whose tsconfig takes a \.classes\.ts file with a bracket value from a dependency$/,
  { timeout: INSTALL_TIMEOUT_MS },
  function (this: GuardsWorld) {
    const file = "node_modules/outside-object/x.classes.ts";
    classesProject(this, file);
    const tsconfig: { include: string[] } = JSON.parse(
      readFileSync(join(copyOf(this), "tsconfig.json"), "utf8"),
    );
    place(
      this,
      "tsconfig.json",
      JSON.stringify({ ...tsconfig, include: [...tsconfig.include, file] }),
    );
  },
);

When(/^rafters-check runs$/, { timeout: RUN_MS }, function (this: GuardsWorld) {
  run(this, BIN, [], copyOf(this));
});

Then(
  /^the report names (\S+) at its line in (\S+)$/,
  function (this: GuardsWorld, value: string, file: string) {
    const lines = readFileSync(join(copyOf(this), file), "utf8").split("\n");
    const line = lines.findIndex((text) => text.includes(value)) + 1;
    const { stdout } = commandOf(this);
    const finding = stdout
      .split("\n")
      .find((entry) => entry.startsWith(`${file}:${line}:`) && entry.includes(value));
    assert.ok(line > 0 && finding, stdout);
    assert.match(finding, / error rafters\(arbitrary-value\) /);
  },
);

Then(/^the report does not name (\S+\[\S+)$/, function (this: GuardsWorld, value: string) {
  assert.ok(!commandOf(this).stdout.includes(value), commandOf(this).stdout);
});

Then(/^the report does not name that file$/, function (this: GuardsWorld) {
  assert.ok(this.classesFile, "no classes file in this scenario");
  assert.ok(!commandOf(this).stdout.includes(this.classesFile), commandOf(this).stdout);
});

Then(
  /^the report names (\S+) on the page at the classy call$/,
  function (this: GuardsWorld, value: string) {
    const { stdout } = commandOf(this);
    for (const page of PAGE_FILES) {
      const lines = readFileSync(join(copyOf(this), page), "utf8").split("\n");
      const line = lines.findIndex((text) => text.includes("classy(box.text)"));
      const column = String(lines[line]).indexOf("box.text") + 1;
      const entry = `${page}:${line + 1}:${column} error rafters(arbitrary-value) ${value} is an arbitrary value`;
      assert.ok(stdout.includes(entry), `${entry}\n${stdout}`);
    }
  },
);

Then(/^the worker's answer on the classes file is the report's$/, function (this: GuardsWorld) {
  const reported = commandOf(this)
    .stdout.split("\n")
    .filter((entry) => entry.startsWith(`${this.classesFile}:`));
  assert.equal(reported.length, 1, commandOf(this).stdout);
  assert.deepEqual(
    this.workerAnswer?.map(
      ({ file, line, column, severity, code, message }) =>
        `${file}:${line}:${column} ${severity} ${code} ${message}`,
    ),
    reported,
  );
});

Then(
  /^each finding says the tokens could not be loaded and names no surface$/,
  function (this: PropsWorld) {
    const found = checked(this);
    assert.equal(found.length, PAGE_FILES.length, JSON.stringify(found));
    for (const { help } of found) {
      assert.match(String(help), /tokens could not be loaded/);
      assert.doesNotMatch(String(help), /surface=/);
    }
  },
);
