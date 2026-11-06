// Compare the colours the browser computes for the class-based reference with
// the colours the email inlined, element by element, light and dark.
// The browser resolves the reference's oklch itself; a canvas turns it into sRGB bytes.
//
//   node tools/colors.mjs <dir holding node_modules/playwright>
import { createRequire } from 'node:module';
import { join, resolve } from 'node:path';

const require = createRequire(join(resolve(process.argv[2]), 'package.json'));
const { chromium } = require('playwright');

const pick = {
  body: ['body', 'body > table > tbody > tr > td'],
  h1: ['h1', 'h1'],
  p: ['p', 'p'],
  button: ['a', 'a'],
  separator: ['div.bg-border', 'td[height="1"]'],
};

const browser = await chromium.launch();
let worst = 0;
for (const scheme of ['light', 'dark']) {
  const ctx = await browser.newContext({ colorScheme: scheme, viewport: { width: 600, height: 900 } });
  const page = await ctx.newPage();
  for (const name of ['newsletter', 'transactional']) {
    const read = async (file, which) => {
      await page.goto(`file://${resolve('out', file)}`);
      // The reference button carries colour transitions; let the page settle before reading.
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(300);
      return page.evaluate(([pick, which]) => {
        const cv = document.createElement('canvas').getContext('2d', { willReadFrequently: true });
        const bytes = (c) => {
          cv.clearRect(0, 0, 1, 1);
          cv.fillStyle = c;
          cv.fillRect(0, 0, 1, 1);
          return Array.from(cv.getImageData(0, 0, 1, 1).data.slice(0, 3));
        };
        const out = {};
        for (const [k, sels] of Object.entries(pick)) {
          const el = document.querySelector(sels[which]);
          if (!el) continue;
          const cs = getComputedStyle(el);
          out[k] = { bg: bytes(cs.backgroundColor), fg: bytes(cs.color) };
        }
        return out;
      }, [pick, which]);
    };
    const ref = await read(`${name}.reference.${scheme}.html`, 0);
    const em = await read(`${name}.html`, 1);
    for (const k of Object.keys(ref)) {
      for (const prop of ['bg', 'fg']) {
        if (k === 'separator' && prop === 'fg') continue;
        const a = ref[k][prop];
        const b = em[k]?.[prop];
        if (!b) continue;
        const d = Math.max(...a.map((v, i) => Math.abs(v - b[i])));
        worst = Math.max(worst, d);
        console.log(`${scheme} ${name} ${k} ${prop}: reference rgb(${a}) email rgb(${b}) max channel diff ${d}`);
      }
    }
  }
  await ctx.close();
}
console.log(`worst channel difference: ${worst}`);
await browser.close();
