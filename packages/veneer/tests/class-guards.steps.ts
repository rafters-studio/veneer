import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { Given, Then } from "@cucumber/cucumber";
import { INSTALL_TIMEOUT_MS, pkg } from "./support/docs-fixture.ts";
import { checked, copyOf, place, project, type PropsWorld } from "./rafters-props.steps.ts";

const PAGE_FILES = ["src/pages/fill.tsx", "src/pages/fill.astro"];
const CLASSY = "const classy = (...inputs: unknown[]) => inputs.join(' ');\n";

interface GuardsWorld extends PropsWorld {
  expectedBrackets?: string[];
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

Then(/^each finding sits on box in class=\{box\}$/, function (this: PropsWorld) {
  for (const { file, line, column, endColumn } of checked(this)) {
    const lines = readFileSync(join(copyOf(this), file), "utf8").split("\n");
    const start = String(lines[line - 1]).indexOf("class={box}") + "class={".length;
    assert.deepEqual([column, endColumn], [start + 1, start + 1 + "box".length], file);
  }
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
