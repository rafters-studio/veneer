// node one-case.mjs <project root> <project-relative .astro file> <case>
// Prints each check() result, one line per pass, in this one process. Nothing is written.
import { readFileSync } from "node:fs";
import { check } from "@rafters/veneer";

const [root, file, how] = process.argv.slice(2);
const existing = readFileSync(`${root}/${file}`, "utf8");
const bracket = `<div class="p-[3px]"></div>\n`;
const appended = `${existing}\n${bracket}`;
const cases = {
  append: [appended],
  replace: [`---\n---\n${bracket}`],
  "append-twice": [appended, appended],
  "unchanged-then-append": [existing, appended],
};
for (const text of cases[how]) {
  const started = performance.now();
  const result = await check(root, { unsaved: { [file]: text } });
  const ms = Math.round(performance.now() - started);
  console.log(how, `${ms}ms`, JSON.stringify(result.ok ? result.report : result.errors));
}
