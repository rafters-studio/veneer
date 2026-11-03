// The divs in .mdx files, which sit outside the program veneer opens. These are TEXT matches, not
// program facts: a <div> in a fenced code sample counts too, and is marked by the fence tracking.
// Usage: node mdx-divs.mjs <copies>
import { readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";
import { SITES } from "./lib.mjs";

const copies = process.argv[2];
for (const site of SITES) {
  const root = join(copies, site);
  const walk = (d) => {
    for (const e of readdirSync(d, { withFileTypes: true })) {
      const p = join(d, e.name);
      if (e.isDirectory()) walk(p);
      else if (e.name.endsWith(".mdx")) {
        let fenced = false;
        readFileSync(p, "utf8").split("\n").forEach((line, i) => {
          if (/^\s*```/.test(line)) fenced = !fenced;
          if (/<div[\s>]/.test(line) || /<div$/.test(line)) {
            console.log(`${site}:${relative(root, p)}:${i + 1} ${fenced ? "FENCED" : "live"} ${line.trim().slice(0, 130)}`);
          }
        });
      }
    }
  };
  try {
    walk(join(root, "src"));
  } catch (error) {
    console.log(`${site}: ${error.message}`);
  }
}
