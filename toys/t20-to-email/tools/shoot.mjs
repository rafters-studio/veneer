// Screenshot each email and its class-based reference at 600px, light and dark,
// in Chromium and WebKit. A browser is a stand-in: it is not a mail client.
//
//   node tools/shoot.mjs <dir holding node_modules/playwright>
//
// The sample image URL is served from the huttspawn site's public/ folder (read only).
import { createRequire } from 'node:module';
import { mkdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

const require = createRequire(join(resolve(process.argv[2]), 'package.json'));
const { chromium, webkit } = require('playwright');
const PUBLIC = '/Volumes/store/projects/huttspawn/site/public';
const out = resolve('out/shots');
mkdirSync(out, { recursive: true });

for (const [engineName, engine] of [['chromium', chromium], ['webkit', webkit]]) {
  const browser = await engine.launch();
  for (const scheme of ['light', 'dark']) {
    const ctx = await browser.newContext({ viewport: { width: 600, height: 900 }, colorScheme: scheme, deviceScaleFactor: 1 });
    await ctx.route('https://huttspawn.com/**', (route) => {
      const path = new URL(route.request().url()).pathname;
      route.fulfill({ body: readFileSync(join(PUBLIC, path)), contentType: 'image/png' });
    });
    const page = await ctx.newPage();
    for (const name of ['newsletter', 'transactional']) {
      for (const [kind, file] of [['email', `${name}.html`], ['reference', `${name}.reference.${scheme}.html`], ['react-email', `${name}.react-email.html`]]) {
        await page.goto(`file://${resolve('out', file)}`);
        await page.waitForLoadState('networkidle');
        const width = await page.evaluate(() => document.documentElement.scrollWidth);
        await page.screenshot({ path: join(out, `${name}.${kind}.${scheme}.${engineName}.png`), fullPage: true });
        console.log(`${name} ${kind} ${scheme} ${engineName}: scrollWidth ${width}px`);
      }
    }
    await ctx.close();
  }
  await browser.close();
}
