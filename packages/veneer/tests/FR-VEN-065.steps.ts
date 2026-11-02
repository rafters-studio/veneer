import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { Given, Then } from "@cucumber/cucumber";
import {
  DOCS,
  GOLDEN,
  componentNamed,
  docsCopy,
  type DocsLine,
  type DocsWorld,
} from "./support/docs-fixture.ts";

function propOf(line: DocsLine, name: string) {
  const prop = line.props?.find((candidate) => candidate.name === name);
  assert.ok(prop, `${line.name} has no prop ${name}`);
  return prop;
}

Given(
  /^the docs fixture, where (src\/components\/\S+) .+$/,
  function (this: DocsWorld, file: string) {
    assert.ok(existsSync(join(docsCopy(this), file)), `${file} is not in the fixture`);
  },
);

Given(
  /^the docs fixture, packages\/veneer\/test\/fixtures\/docs, copied into a fresh temporary folder and installed with pnpm install --frozen-lockfile$/,
  function (this: DocsWorld) {
    docsCopy(this);
  },
);

Given(
  /^src\/components\/astro-default\.astro destructures title = 'Hello' from Astro\.props$/,
  function (this: DocsWorld) {
    const text = readFileSync(join(docsCopy(this), "src/components/astro-default.astro"), "utf8");
    assert.ok(text.includes("title = 'Hello'"));
  },
);

Then(
  /^(\w+)'s (\w+) has the type (\[.*\]) and the values (\[.*\])$/,
  function (this: DocsWorld, component: string, name: string, type: string, values: string) {
    const prop = propOf(componentNamed(this, component), name);
    assert.deepEqual(prop.type, JSON.parse(type));
    assert.deepEqual(prop.values, JSON.parse(values));
  },
);

Then(
  /^(\w+)'s (\w+) has the default "(.+)"$/,
  function (this: DocsWorld, component: string, name: string, fallback: string) {
    assert.equal(propOf(componentNamed(this, component), name).default, fallback);
  },
);

Then(/^(\w+) has no own props$/, function (this: DocsWorld, component: string) {
  assert.deepEqual(componentNamed(this, component).props, []);
});

Then(/^its inherited list holds (\{.*\})$/, function (this: DocsWorld, entry: string) {
  assert.ok(
    componentNamed(this, "Inherits").inherited?.some((found) => {
      try {
        assert.deepEqual(found, JSON.parse(entry));
        return true;
      } catch {
        return false;
      }
    }),
    `the inherited list has no ${entry}`,
  );
});

Then(/^its inheritedDefaults is (\{.*\})$/, function (this: DocsWorld, defaults: string) {
  assert.deepEqual(componentNamed(this, "Inherits").inheritedDefaults, JSON.parse(defaults));
});

Then(
  /^(\w+)'s tags hold (\{.*\}) and then (\{.*\})$/,
  function (this: DocsWorld, component: string, first: string, second: string) {
    const tags = componentNamed(this, component).tags ?? [];
    assert.deepEqual(tags.slice(0, 2), [JSON.parse(first), JSON.parse(second)]);
  },
);

Then(
  /^its \.rafters\/veneer\/docs\.jsonl equals packages\/veneer\/test\/fixtures\/docs\.expected\.jsonl byte for byte$/,
  function (this: DocsWorld) {
    assert.deepEqual(readFileSync(join(docsCopy(this), DOCS)), readFileSync(GOLDEN));
  },
);
