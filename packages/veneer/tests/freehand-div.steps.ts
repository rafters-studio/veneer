import assert from "node:assert/strict";
import { readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { Given, Then } from "@cucumber/cucumber";
import { commandOf } from "./check-command.steps.ts";
import { jsDoc } from "./custom-components.steps.ts";
import { INSTALL_TIMEOUT_MS } from "./support/docs-fixture.ts";
import { checked, copyOf, place, project, type PropsWorld } from "./rafters-props.steps.ts";

const PAGE_FILES = ["src/pages/fill.tsx", "src/pages/fill.astro"];
const TSX_IMPORT = "import { Surface } from '@/components/ui/surface';\n";
const ASTRO_IMPORT = "---\nimport Surface from '@/components/ui/surface-astro.astro';\n---\n";

// A div holding a rafters component, in each format: the page "a page with ..." builds.
const HOLDING: [string, string] = [
  `${TSX_IMPORT}export const Page = () => <div><Surface /></div>;\n`,
  `${ASTRO_IMPORT}<div><Surface /></div>\n`,
];

Given(
  /^a project file with a div and no component in scan$/,
  { timeout: INSTALL_TIMEOUT_MS },
  function (this: PropsWorld) {
    project(this, "export const Page = () => <div />;\n", "<div />\n");
  },
);

Given(
  /^a project whose only diagnostic is rafters\(freehand-div\)$/,
  { timeout: INSTALL_TIMEOUT_MS },
  function (this: PropsWorld) {
    project(this, ...HOLDING);
    // The lint fixture's other files hold findings of their own.
    for (const other of [
      "src/widgets",
      "src/pages/astro-cases.astro",
      "src/pages/brackets.astro",
      "src/pages/brackets-tsx.tsx",
      "src/pages/tsx-cases.tsx",
    ]) {
      rmSync(join(copyOf(this), other), { recursive: true, force: true });
    }
  },
);

Given(
  /^a file under componentsPath holding a div and using a component in scan$/,
  { timeout: INSTALL_TIMEOUT_MS },
  function (this: PropsWorld) {
    project(this, "export {};\n", "<div />\n");
    place(this, "src/components/ui/box.tsx", HOLDING[0]);
    place(this, "src/components/ui/box-astro.astro", HOLDING[1]);
    this.checkPaths = ["src/components/ui/box.tsx", "src/components/ui/box-astro.astro"];
  },
);

Given(
  /^a qualified component's own file holding a div and using a component in scan$/,
  { timeout: INSTALL_TIMEOUT_MS },
  function (this: PropsWorld) {
    project(this, "export {};\n", "<div />\n");
    const doc = jsDoc();
    place(
      this,
      "src/widgets/own.tsx",
      `${TSX_IMPORT}${doc}export function Own() {\n  return <div><Surface /></div>;\n}\n`,
    );
    place(
      this,
      "src/widgets/own-astro.astro",
      `---\nimport Surface from '@/components/ui/surface-astro.astro';\n${doc}---\n<div><Surface /></div>\n`,
    );
    for (const stem of ["src/widgets/own", "src/widgets/own-astro"]) {
      place(this, `${stem}.behavior.ts`, "export {};\n");
      place(this, `${stem}.classes.ts`, "export {};\n");
    }
    this.checkPaths = ["src/widgets/own.tsx", "src/widgets/own-astro.astro"];
  },
);

Then(
  /^it reports rafters\(freehand-div\) at severity warning on the div$/,
  function (this: PropsWorld) {
    for (const page of PAGE_FILES) {
      const here = checked(this).filter((found) => found.file === page);
      assert.equal(here.length, 1, `${page}: ${JSON.stringify(checked(this))}`);
      const [found] = here;
      assert.ok(found);
      assert.equal(found.code, "rafters(freehand-div)");
      assert.equal(found.severity, "warning");
      const line = readFileSync(join(copyOf(this), page), "utf8").split("\n")[found.line - 1];
      assert.equal(line?.slice(found.column - 1, found.endColumn - 1), "div");
    }
  },
);

Then(/^the help names Container and Grid$/, function (this: PropsWorld) {
  assert.ok(checked(this).length > 0, "nothing was reported");
  for (const { help } of checked(this)) assert.match(String(help), /Container or Grid/);
});

Then(/^it reports nothing for that file$/, function (this: PropsWorld) {
  assert.deepEqual(checked(this), []);
});

Then(
  /^the report names rafters\(freehand-div\)$/,
  function (this: Parameters<typeof commandOf>[0]) {
    assert.match(commandOf(this).stdout, / warning rafters\(freehand-div\) /);
  },
);
