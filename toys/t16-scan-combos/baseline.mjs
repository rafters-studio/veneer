// Runs `astro check --noSync` on each site copy and compares it with what facts.ts recorded from
// veneer: the raw diagnostics on .astro and .tsx files, and veneer's rafters-filtered findings.
// Usage: node baseline.mjs <copies>
import { spawnSync } from "node:child_process";
import { existsSync, writeFileSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { SITES } from "./lib.mjs";

const here = import.meta.dirname;
const copies = process.argv[2];
const summary = {};
for (const site of SITES) {
  const start = performance.now();
  // Never `pnpm exec` here. The recorded run did, and pnpm's install-before-run wrote bin shims through
  // a symlinked .bin into two original repos. See NOTES.md, "A write I should not have made".
  const out = spawnSync(process.execPath, ["node_modules/astro/bin/astro.mjs", "check", "--noSync"], {
    cwd: join(copies, site),
    encoding: "utf8",
    env: { ...process.env, NO_COLOR: "1", FORCE_COLOR: "0" },
    timeout: 300000,
    maxBuffer: 1 << 27,
  });
  const ms = Math.round(performance.now() - start);
  const text = `${out.stdout}\n${out.stderr}`.replace(/\u001b\[[0-9;]*m/g, "");
  writeFileSync(join(here, "results/astro-check", `${site}.txt`), text);
  const found = [...text.matchAll(/^(\S+?):(\d+):(\d+) - (error|warning|hint) ts\((\d+)\): (.*)$/gm)].map((m) => ({
    file: m[1], line: Number(m[2]), code: `ts(${m[5]})`, severity: m[4],
  }));
  const totals = text.match(/Result \((\d+) files\):\s*\n- (\d+) errors?\n- (\d+) warnings?\n- (\d+) hints?/);
  const metaPath = join(here, "results/facts", `${site}.meta.json`);
  const meta = existsSync(metaPath) ? JSON.parse(readFileSync(metaPath, "utf8")) : { raw: [], baseline: [] };
  const key = (d) => `${d.file}:${d.line}:${d.code}`;
  const veneerRaw = new Set(meta.raw.map(key));
  const astroKeys = new Set(found.map(key));
  const astroOnErrors = found.filter((d) => d.severity === "error");
  const inProgramFiles = found.filter((d) => /\.(astro|tsx)$/.test(d.file));
  summary[site] = {
    exit: out.status,
    ms,
    astro: totals ? { files: +totals[1], errors: +totals[2], warnings: +totals[3], hints: +totals[4] } : "no result line",
    astroErrorsParsed: astroOnErrors.map(key),
    astroAstroTsx: inProgramFiles.length,
    veneerRaw: meta.raw.length,
    veneerRawErrors: meta.raw.filter((d) => d.severity === "error").map(key),
    veneerRafters: meta.baseline.map(key),
    onlyAstro: inProgramFiles.map(key).filter((k) => !veneerRaw.has(k)),
    onlyVeneer: [...veneerRaw].filter((k) => !astroKeys.has(k)),
  };
  console.log(site, JSON.stringify(summary[site]));
}
writeFileSync(join(here, "results/astro-check/summary.json"), `${JSON.stringify(summary, null, 1)}\n`);
