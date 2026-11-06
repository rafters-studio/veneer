// OKLCH to hex with rafters' own code, before the Rust transform runs.
//
// Every `oklch(...)` literal in the compiled CSS (the token values) is replaced by
// the hex `@rafters/color-utils` gives: `toGamutCeiling(c, 'srgb')` first, as
// `oklchToSRGB`'s contract asks for an out-of-gamut colour, then `oklchToSRGB`.
// A literal with alpha becomes `rgba()` from the same components. The package is
// imported from the rafters checkout (read only) and run under its own tsx, since
// its source imports `./x.js` names that are `.ts` files.
//
//   node <rafters>/packages/color-utils/node_modules/tsx/dist/cli.mjs tools/rafters-colors.mjs \
//     css/huttspawn.compiled.css css/huttspawn.rafters-hex.css target/release/t20-to-email
//
// Writes the rewritten CSS, and out/color-diff.tsv: each literal, rafters' hex,
// lightningcss's own lowering of the same literal, and the largest channel difference.
import { readFileSync, writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { oklchToSRGB, toGamutCeiling } from '/Volumes/store/projects/rafters-studio/rafters/packages/color-utils/src/index.ts';

const [input, output, bin] = process.argv.slice(2);
const css = readFileSync(input, 'utf8');
const RE = /oklch\(\s*([\d.]+%?)\s+([\d.]+)\s+([\d.]+)(?:deg)?\s*(?:\/\s*([\d.]+%?))?\s*\)/g;
const num = (v) => (v.endsWith('%') ? Number.parseFloat(v) / 100 : Number(v));

const seen = new Map();
const out = css.replace(RE, (lit, l, c, h, a) => {
  if (!seen.has(lit)) {
    const oklch = { l: num(l), c: Number(c), h: Number(h) };
    const mapped = toGamutCeiling(oklch, 'srgb');
    const { components, hex } = oklchToSRGB(mapped);
    const alpha = a === undefined ? 1 : num(a);
    const value = alpha === 1 ? hex : `rgba(${components.map((v) => Math.round(v * 255)).join(',')},${alpha})`;
    seen.set(lit, { value, hex, mapped: mapped.c !== oklch.c });
  }
  return seen.get(lit).value;
});
writeFileSync(output, out);

// What lightningcss would have printed for each literal.
const lits = [...seen.keys()];
const lc = execFileSync(bin, ['lower'], { input: lits.join('\n') + '\n', encoding: 'utf8' })
  .trim()
  .split('\n')
  .map((line) => line.split('\t')[1]);
const bytes = (hex) => (/^#[0-9a-f]{6}$/i.test(hex) ? [1, 3, 5].map((i) => Number.parseInt(hex.slice(i, i + 2), 16)) : null);
const expand = (v) => (/^#[0-9a-f]{3}$/i.test(v) ? `#${[...v.slice(1)].map((x) => x + x).join('')}` : v);
const rows = ['literal\trafters\tlightningcss\tmax_channel_diff\tgamut_mapped'];
let differ = 0;
let worst = 0;
let mapped = 0;
lits.forEach((lit, i) => {
  const r = seen.get(lit);
  const l = expand(lc[i] ?? '?');
  const [x, y] = [bytes(r.hex), bytes(l)];
  const d = x && y ? Math.max(...x.map((v, k) => Math.abs(v - y[k]))) : 'n/a';
  if (d !== 0) differ++;
  if (typeof d === 'number') worst = Math.max(worst, d);
  if (r.mapped) mapped++;
  rows.push(`${lit}\t${r.hex}\t${l}\t${d}\t${r.mapped}`);
});
writeFileSync('out/color-diff.tsv', rows.join('\n') + '\n');
console.log(`${lits.length} distinct oklch literals; ${differ} differ from lightningcss; worst channel difference ${worst}; ${mapped} were out of sRGB and gamut-mapped`);
