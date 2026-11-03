// Copies each site's node_modules as a directory of symlinks, one per entry, so that anything a
// tool writes into node_modules (astro's cache dir, for one) lands in the copy and not the source.
// Usage: node prep.mjs <folder holding the site copies> <site>=<original site root> ...
import { existsSync, mkdirSync, readdirSync, statSync, symlinkSync } from "node:fs";
import { join } from "node:path";

const [copies, ...pairs] = process.argv.slice(2);
for (const pair of pairs) {
  const [site, original] = pair.split("=");
  const from = join(original, "node_modules");
  const to = join(copies, site, "node_modules");
  if (!existsSync(join(copies, site, "tsconfig.json")) || !existsSync(from)) {
    console.log(`${site}: skipped (no tsconfig in the copy, or no node_modules in the original)`);
    continue;
  }
  mkdirSync(to, { recursive: true });
  let linked = 0;
  for (const entry of readdirSync(from)) {
    // .bin and .pnpm stay out: a package manager run in the copy writes through those links.
    if ([".astro", ".vite", ".cache", ".bin", ".pnpm"].includes(entry)) continue;
    const source = join(from, entry);
    if (entry.startsWith("@") && statSync(source).isDirectory()) {
      mkdirSync(join(to, entry), { recursive: true });
      for (const scoped of readdirSync(source)) {
        if (!existsSync(join(to, entry, scoped))) symlinkSync(join(source, scoped), join(to, entry, scoped));
        linked += 1;
      }
    } else if (!existsSync(join(to, entry))) {
      symlinkSync(source, join(to, entry));
      linked += 1;
    }
  }
  console.log(`${site}: ${linked} entries linked`);
}
