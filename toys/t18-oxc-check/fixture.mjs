// node fixture.mjs <t18-oxc-check> <packages/veneer/test/fixtures/lint>
// Compares the Rust check's findings with lint.expected.jsonl's rafters(...) lines, on the keys the
// golden holds for each line. The ts(...) lines are rule one, which the toy does not port.
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";

const [bin, fixture] = process.argv.slice(2);
const parse = (text) => text.split("\n").filter(Boolean).map((line) => JSON.parse(line));
const expected = parse(readFileSync(path.join(fixture, "../lint.expected.jsonl"), "utf8"));
const golden = expected.filter(({ code }) => code.startsWith("rafters("));
const result = spawnSync(bin, [fixture, "--jsonl"], { encoding: "utf8" });
const actual = parse(result.stdout);

const key = (finding, keys) => JSON.stringify(keys.map((k) => finding[k]));
const unmatched = [...actual];
const missing = [];
for (const want of golden) {
  const keys = Object.keys(want);
  const at = unmatched.findIndex((got) => key(got, keys) === key(want, keys));
  if (at === -1) missing.push(want);
  else unmatched.splice(at, 1);
}
console.log(`golden: ${expected.length} lines, ${golden.length} rafters(...), ${expected.length - golden.length} ts(...) not ported`);
console.log(`rust: ${actual.length} findings; matched ${golden.length - missing.length}, missing ${missing.length}, extra ${unmatched.length}`);
for (const m of missing) console.log(`missing ${JSON.stringify(m)}`);
for (const u of unmatched) console.log(`extra   ${JSON.stringify(u)}`);
