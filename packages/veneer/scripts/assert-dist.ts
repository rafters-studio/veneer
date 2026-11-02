// node scripts/assert-dist.ts <dist folder>: fails when write code or css-tree reached the build.
import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";

const FORBIDDEN = ["css-tree", "saveRegistryToDir", "writeFileAtomicSync"];

const dist = process.argv[2] ?? "";
const entries = readdirSync(dist, { recursive: true, withFileTypes: true });
const found = entries
  .filter((entry) => entry.isFile())
  .flatMap((entry) => {
    const file = path.join(entry.parentPath, entry.name);
    const text = readFileSync(file, "utf8");
    return FORBIDDEN.filter((name) => text.includes(name)).map((name) => `${file}: ${name}`);
  });

if (found.length > 0) {
  console.error(`dist holds what it must not:\n${found.join("\n")}`);
  process.exit(1);
}
