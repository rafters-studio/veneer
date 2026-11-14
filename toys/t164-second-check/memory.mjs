// node --expose-gc --import ./modes.mjs memory.mjs <root> <opens> [unsavedFile]: P5, the heap after gc per check, and the ms per check.
import { readFileSync } from "node:fs";
import { check } from "@rafters/veneer";

const [root, count, file] = process.argv.slice(2);
const text = file ? readFileSync(`${root}/${file}`, "utf8") : "";
const heaps = [];
const times = [];
for (let open = 0; open < Number(count); open += 1) {
  const started = performance.now();
  const result = await check(root, file ? { unsaved: { [file]: `${text}\n<!-- ${open} -->\n` } } : {});
  times.push(performance.now() - started);
  if (!result.ok) throw new Error(JSON.stringify(result.errors));
  globalThis.gc();
  await new Promise((done) => setImmediate(done));
  globalThis.gc();
  heaps.push(process.memoryUsage().heapUsed / 1048576);
}
const tail = Math.floor(Number(count) / 2);
const perOpen = (heaps.at(-1) - heaps.at(-tail - 1)) / tail;
const median = (values) => values.toSorted((a, b) => a - b)[Math.floor(values.length / 2)];
console.log(
  JSON.stringify({
    mode: process.env.MODE ?? "none",
    first: Math.round(heaps[0]),
    last: Math.round(heaps.at(-1)),
    mbPerOpenSecondHalf: Number(perOpen.toFixed(2)),
    firstMs: Math.round(times[0]),
    laterMedianMs: Math.round(median(times.slice(1))),
  }),
);
