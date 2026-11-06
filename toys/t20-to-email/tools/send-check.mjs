// Evidence for part 2 (send-ready output). In a browser stand-in, not a mail client.
//
//   node tools/send-check.mjs <dir holding node_modules/playwright>
//
// 1. The plain text carries every heading, paragraph and link URL, and no image alt or preheader.
// 2. Longest line, ours and react-email's, against RFC 5322's 998 octets.
// 3. The doctype: our HTML as written (HTML5) and with react-email's XHTML 1.0
//    Transitional doctype, screenshotted at 600px in Chromium and WebKit, compared byte for byte.
// 4. The preheader: not rendered, and not in the page's visible text.
import { createRequire } from 'node:module';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { join, resolve } from 'node:path';

const require = createRequire(join(resolve(process.argv[2]), 'package.json'));
const { chromium, webkit } = require('playwright');
const XHTML = '<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd">';
const PUBLIC = '/Volumes/store/projects/huttspawn/site/public';
const names = ['newsletter', 'transactional'];
const lines = [];
const say = (s) => {
  console.log(s);
  lines.push(s);
};

for (const name of names) {
  const rafter = JSON.parse(readFileSync(`samples/${name}.rafter`, 'utf8'));
  const text = readFileSync(`out/${name}.txt`, 'utf8');
  const missing = [];
  const leaked = [];
  for (const b of rafter.blocks) {
    if ((b.type === 'heading' || b.type === 'text' || b.type === 'button') && !text.includes(b.content)) missing.push(b.id);
    if (b.meta?.href && !text.includes(b.meta.href)) missing.push(`${b.id}.href`);
    if (b.type === 'image' && text.includes(b.meta.alt)) leaked.push(`${b.id}.alt`);
    if (b.type === 'preview' && text.includes(b.content)) leaked.push(b.id);
  }
  say(`${name} plain text: missing [${missing.join(', ')}], leaked [${leaked.join(', ')}]`);
  for (const f of [`${name}.html`, `${name}.react-email.html`]) {
    const html = readFileSync(`out/${f}`, 'utf8');
    say(`${f}: longest line ${Math.max(...html.split('\n').map((l) => Buffer.byteLength(l)))} octets, ${Buffer.byteLength(html)} octets in all`);
  }
}

mkdirSync('out/doctype', { recursive: true });
for (const [engineName, engine] of [['chromium', chromium], ['webkit', webkit]]) {
  const browser = await engine.launch();
  const ctx = await browser.newContext({ viewport: { width: 600, height: 900 }, deviceScaleFactor: 1 });
  await ctx.route('https://huttspawn.com/**', (route) =>
    route.fulfill({ body: readFileSync(join(PUBLIC, new URL(route.request().url()).pathname)), contentType: 'image/png' }),
  );
  const page = await ctx.newPage();
  for (const name of names) {
    const html = readFileSync(`out/${name}.html`, 'utf8');
    const shots = {};
    for (const [kind, doc] of [['html5', html], ['xhtml', html.replace('<!DOCTYPE html>', XHTML)]]) {
      const path = resolve(`out/doctype/${name}.${kind}.html`);
      writeFileSync(path, doc);
      await page.goto(`file://${path}`);
      await page.waitForLoadState('networkidle');
      shots[kind] = await page.screenshot({ fullPage: true });
      if (kind === 'html5') {
        const pre = await page.evaluate((preview) => {
          const div = document.body.querySelector('div');
          const r = div.getBoundingClientRect();
          return { display: getComputedStyle(div).display, height: r.height, inVisibleText: document.body.innerText.includes(preview) };
        }, JSON.parse(readFileSync(`samples/${name}.rafter`, 'utf8')).blocks.find((b) => b.type === 'preview').content);
        say(`${engineName} ${name} preheader: display ${pre.display}, height ${pre.height}px, in visible text ${pre.inVisibleText}`);
      }
      const mode = await page.evaluate(() => document.compatMode);
      say(`${engineName} ${name} ${kind}: compatMode ${mode}`);
    }
    say(`${engineName} ${name}: HTML5 and XHTML screenshots byte-identical: ${shots.html5.equals(shots.xhtml)}`);
  }
  await browser.close();
}
writeFileSync('out/send-check.txt', lines.join('\n') + '\n');
