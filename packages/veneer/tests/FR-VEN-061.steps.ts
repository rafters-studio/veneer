import assert from "node:assert/strict";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { Given, Then } from "@cucumber/cucumber";
import {
  CONFIG,
  DOCS,
  componentLines,
  docsCopy,
  docsLines,
  errorsOf,
  okResult,
  type DocsWorld,
} from "./support/docs-fixture.ts";

Given(
  /^the docs fixture, whose \.rafters\/config\.rafters\.json is (\{.*\})$/,
  function (this: DocsWorld, config: string) {
    const written = readFileSync(join(docsCopy(this), CONFIG), "utf8");
    assert.deepEqual(JSON.parse(written), JSON.parse(config));
  },
);

Given(
  /^the docs fixture with \.rafters\/config\.rafters\.json set to (\{.*\})$/,
  function (this: DocsWorld, config: string) {
    writeFileSync(join(docsCopy(this), CONFIG), config);
  },
);

Given(
  /^the docs fixture with \.rafters\/config\.rafters\.json deleted$/,
  function (this: DocsWorld) {
    rmSync(join(docsCopy(this), CONFIG));
  },
);

Given(
  /^the docs fixture with \.rafters\/config\.rafters\.json holding the text (.+)$/,
  function (this: DocsWorld, text: string) {
    writeFileSync(join(docsCopy(this), CONFIG), text);
  },
);

Given(
  /^the test has created the empty folder (\S+) in the copy$/,
  function (this: DocsWorld, folder: string) {
    mkdirSync(join(docsCopy(this), folder), { recursive: true });
  },
);

Then(
  /^line 1 of (?:\.rafters\/veneer\/)?docs\.jsonl is (\{.*\})$/,
  function (this: DocsWorld, line: string) {
    const [first] = readFileSync(join(docsCopy(this), DOCS), "utf8").split("\n");
    assert.equal(first, line);
  },
);

Then(
  /^it returns ok true with docs\.jsonl holding (\d+) component lines$/,
  function (this: DocsWorld, count: string) {
    okResult(this);
    assert.equal(docsLines(this)[0]?.kind, "project");
    assert.equal(componentLines(this).length, Number(count));
  },
);

Then(/^its notices are (\[.*\])$/, function (this: DocsWorld, notices: string) {
  assert.deepEqual(okResult(this).notices, JSON.parse(notices));
});

Then(
  /^it returns ok false with errors (\[.*\]) and each error has a message$/,
  function (this: DocsWorld, expected: string) {
    const errors = errorsOf(this);
    assert.deepEqual(
      errors.map(({ message: _message, ...rest }) => rest),
      JSON.parse(expected),
    );
    for (const { message } of errors) assert.ok(message.length > 0);
  },
);

Then(/^\.rafters\/veneer\/docs\.jsonl does not exist$/, function (this: DocsWorld) {
  assert.equal(existsSync(join(docsCopy(this), DOCS)), false);
});

Then(
  /^it returns ok false with one error whose code is "([^"]+)", whose field is (absent|"[^"]+"), and whose message contains "([^"]+)"$/,
  function (this: DocsWorld, code: string, field: string, contains: string) {
    const errors = errorsOf(this);
    assert.equal(errors.length, 1, JSON.stringify(errors));
    assert.equal(errors[0]?.code, code);
    assert.equal(errors[0]?.field, field === "absent" ? undefined : field.slice(1, -1));
    assert.ok(errors[0]?.message.includes(contains), errors[0]?.message);
  },
);

Then(
  /^it returns ok false with errors, in this order, of code "config-field" and field "componentsPath" with no value, field "intent" with value 7, and field "veneer\.mdxPath" with value 42$/,
  function (this: DocsWorld) {
    const errors = errorsOf(this);
    assert.deepEqual(
      errors.map(({ message: _message, ...rest }) => rest),
      [
        { code: "config-field", field: "componentsPath" },
        { code: "config-field", field: "intent", value: 7 },
        { code: "config-field", field: "veneer.mdxPath", value: 42 },
      ],
    );
    assert.equal("value" in (errors[0] ?? {}), false);
  },
);

Then(/^nothing exists at \.rafters\/veneer$/, function (this: DocsWorld) {
  assert.equal(existsSync(join(docsCopy(this), ".rafters/veneer")), false);
});

Then(
  /^it returns ok false with five errors of code "config-field", in this order: (.+)$/,
  function (this: DocsWorld, list: string) {
    const expected = list.split(/, (?=field ")/).map((item) => {
      const match = /^field "([^"]+)" value (.+)$/.exec(item);
      assert.ok(match, item);
      return { code: "config-field", field: match[1], value: JSON.parse(match[2] ?? "") };
    });
    assert.equal(expected.length, 5);
    assert.deepEqual(
      errorsOf(this).map(({ message: _message, ...rest }) => rest),
      expected,
    );
  },
);

Then(/^each error's message starts with its field and ": "$/, function (this: DocsWorld) {
  for (const { field, message } of errorsOf(this)) {
    assert.ok(message.startsWith(`${field}: `), message);
  }
});
