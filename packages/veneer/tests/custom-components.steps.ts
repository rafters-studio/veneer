import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { Given, Then, When } from "@cucumber/cucumber";
import type { CheckResult } from "@rafters/veneer";
import { CONFIG, INSTALL_TIMEOUT_MS, pkg } from "./support/docs-fixture.ts";
import { checked, copyOf, place, project, type PropsWorld } from "./rafters-props.steps.ts";

const CHILD_MS = 60 * 1000;
const TSX_PAGE = "src/pages/fill.tsx";
const ASTRO_PAGE = "src/pages/fill.astro";
const TAGS = [
  "cognitive-load 3/10",
  "attention-economics Secondary",
  "trust-building Low",
  "accessibility Keyboard",
  "semantic-meaning Custom",
  "usage-patterns DO: use it",
];
const OWN_FILES = ["src/widgets/custom.tsx", "src/widgets/custom-astro.astro"];

interface CustomWorld extends PropsWorld {
  checkResult?: CheckResult;
}

const block = (tags: string[]): string => `/**\n${tags.map((tag) => ` * @${tag}\n`).join("")} */\n`;

export function jsDoc(missing?: string): string {
  if (missing === "the @usage-patterns tag") return block(TAGS.slice(0, -1));
  if (missing === "one block holding the tags") {
    return block(TAGS.slice(0, 3)) + block(TAGS.slice(3));
  }
  return block(TAGS);
}

// One component in each format, a custom one for each page to use.
function widget(world: CustomWorld, name: string, missing?: string): void {
  const doc = jsDoc(missing);
  const tag = name[0]?.toUpperCase() + name.slice(1);
  const stems = [`src/widgets/${name}`, `src/widgets/${name}-astro`];
  place(
    world,
    `${stems[0]}.tsx`,
    `${doc}export function ${tag}(props: { fill?: string }) {\n  return <div className="w-[3px]">{props.fill}</div>;\n}\n`,
  );
  place(
    world,
    `${stems[1]}.astro`,
    `---\n${doc}interface Props { fill?: string }\n---\n<div class="w-[3px]"><slot /></div>\n`,
  );
  for (const stem of stems) {
    if (missing !== "its behavior file") place(world, `${stem}.behavior.ts`, "export {};\n");
    if (missing !== "its classes file") place(world, `${stem}.classes.ts`, "export {};\n");
  }
}

const IMPORTS: Record<string, [string, string]> = {
  Surface: [
    "import { Surface } from '@/components/ui/surface';",
    "import Surface from '@/components/ui/surface-astro.astro';",
  ],
  Custom: [
    "import { Custom } from '@/widgets/custom';",
    "import Custom from '@/widgets/custom-astro.astro';",
  ],
  Plain: [
    "import { Plain } from '@/widgets/plain';",
    "import Plain from '@/widgets/plain-astro.astro';",
  ],
  Button: ["import { Button } from 'third-party-ui';", "import { Button } from 'third-party-ui';"],
};

function pages(world: CustomWorld, uses: [string, string][]): void {
  const imported = (format: 0 | 1): string =>
    uses.map(([tag]) => IMPORTS[tag]?.[format]).join("\n");
  const body = uses.map(([tag, attribute]) => `    <${tag} ${attribute} />\n`).join("");
  place(
    world,
    TSX_PAGE,
    `${imported(0)}\nexport const Page = () => (\n  <main>\n${body}  </main>\n);\n`,
  );
  place(world, ASTRO_PAGE, `---\n${imported(1)}\n---\n<main>\n${body}</main>\n`);
}

Given(
  /^a project with a custom component outside componentsPath, (?:with all six tags, a behavior file and a classes file|missing (.+))$/,
  { timeout: INSTALL_TIMEOUT_MS },
  function (this: CustomWorld, missing?: string) {
    project(this, "", "");
    widget(this, "custom", missing);
  },
);

Given(
  /^a project with a system component, a qualified custom component and an unqualified custom component$/,
  { timeout: INSTALL_TIMEOUT_MS },
  function (this: CustomWorld) {
    project(this, "", "");
    widget(this, "custom");
    widget(this, "plain", "the @usage-patterns tag");
  },
);

Given(/^a page passing it a prop its type refuses$/, function (this: CustomWorld) {
  pages(this, [["Custom", 'wrong="x"']]);
});

Given(/^a page passing it fill="(.*)"$/, function (this: CustomWorld, value: string) {
  pages(this, [["Custom", `fill="${value}"`]]);
});

Given(/^a page passing each a prop its type refuses$/, function (this: CustomWorld) {
  pages(this, [
    ["Surface", 'wrong="x"'],
    ["Custom", 'wrong="x"'],
    ["Plain", 'wrong="x"'],
  ]);
});

Given(
  /^a page passing a prop its type refuses to a component from node_modules$/,
  function (this: CustomWorld) {
    pages(this, [["Button", 'wrong="x"']]);
  },
);

Given(/^the veneer block sets scan to (\S+)$/, function (this: CustomWorld, scan: string) {
  const file = join(copyOf(this), CONFIG);
  const config = JSON.parse(readFileSync(file, "utf8"));
  writeFileSync(file, JSON.stringify({ ...config, veneer: { scan } }));
});

When(
  /^veneer's check runs on the component's own files$/,
  { timeout: CHILD_MS },
  function (this: CustomWorld) {
    const script = `import { check } from ${JSON.stringify(join(pkg, "dist/index.mjs"))};
process.stdout.write(JSON.stringify(await check(${JSON.stringify(copyOf(this))}, { paths: ${JSON.stringify(OWN_FILES)} })));
`;
    const { status, stdout, stderr } = spawnSync(
      process.execPath,
      ["--input-type=module", "--eval", script],
      { encoding: "utf8", timeout: CHILD_MS },
    );
    assert.equal(status, 0, stderr);
    this.checkResult = JSON.parse(stdout);
  },
);

// The components reported on one page, by the tag on each diagnostic's line.
function tagsReported(world: CustomWorld, page: string): string[] {
  const lines = readFileSync(join(copyOf(world), page), "utf8").split("\n");
  return checked(world)
    .filter((found) => found.file === page)
    .map((found) => {
      assert.equal(found.severity, "error", JSON.stringify(found));
      assert.match(found.code, /^ts\(/, JSON.stringify(found));
      return /<(\w+)/.exec(lines[found.line - 1] ?? "")?.[1] ?? "";
    });
}

const REPORTED: Record<string, string[]> = {
  "the system component only": ["Surface"],
  "the system and qualified components": ["Surface", "Custom"],
  "all three": ["Surface", "Custom", "Plain"],
};

function assertReported(world: CustomWorld, expected: string[]): void {
  for (const page of [TSX_PAGE, ASTRO_PAGE]) {
    assert.deepEqual(tagsReported(world, page), expected, page);
  }
}

Then(/^it reports the type error at severity error$/, function (this: CustomWorld) {
  assertReported(this, ["Custom"]);
});

Then(/^it reports the type error on (.+)$/, function (this: CustomWorld, reported: string) {
  const expected = REPORTED[reported];
  assert.ok(expected, reported);
  assertReported(this, expected);
});

Then(/^it reports nothing for that use$/, function (this: CustomWorld) {
  assert.deepEqual(checked(this), []);
});

Then(/^it reports (\d+) bracket values$/, function (this: CustomWorld, count: string) {
  assert.ok(this.checkResult?.ok, JSON.stringify(this.checkResult));
  const brackets = this.checkResult.diagnostics.filter(
    (found) => found.code === "rafters(arbitrary-value)",
  );
  assert.deepEqual(
    brackets.map((found) => found.file).sort(),
    Number(count) ? OWN_FILES.toSorted() : [],
  );
});

Then(/^no file of the component was loaded as a module$/, function (this: CustomWorld) {
  assert.ok(this.modulesLoaded, "check has not run");
  assert.deepEqual(
    this.modulesLoaded.filter((url) => url.includes("/src/widgets/")),
    [],
  );
});

Then(/^the check fails with a config error on (\S+)$/, function (this: CustomWorld, field: string) {
  assert.ok(this.checkResult && !this.checkResult.ok, JSON.stringify(this.checkResult));
  assert.deepEqual(
    this.checkResult.errors.map((error) => [error.code, error.field]),
    [["config-field", field]],
  );
});
