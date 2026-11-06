// Look up every CSS property, value form and HTML feature an output email uses
// in caniemail's data (the repo's _features/*.md front matter).
//
//   node tools/caniemail.mjs <caniemail repo> out/newsletter.html out/transactional.html
//
// Prints a table per email and writes out/caniemail.json.
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { join, basename } from 'node:path';

const [repo, ...files] = process.argv.slice(2);

// The clients the question names. caniemail has no separate key for the new
// Outlook for Windows; it is Outlook.com's web engine, so outlook-com stands in.
const CLIENTS = [
  ['Gmail web', 'gmail', 'desktop-webmail'],
  ['Gmail iOS', 'gmail', 'ios'],
  ['Gmail Android', 'gmail', 'android'],
  ['Outlook Windows (Word)', 'outlook', 'windows'],
  ['Outlook.com / new Outlook', 'outlook', 'outlook-com'],
  ['Apple Mail macOS', 'apple-mail', 'macos'],
  ['Apple Mail iOS', 'apple-mail', 'ios'],
];

/** caniemail front matter blocks are YAML flow maps; quote the keys and read them as JSON. */
function block(src, key) {
  const at = src.indexOf(`${key}: {`);
  if (at < 0) return null;
  const i = src.indexOf('{', at);
  let depth = 0;
  let j = i;
  let inString = false;
  for (; j < src.length; j++) {
    const ch = src[j];
    if (inString) {
      if (ch === '\\') j++;
      else if (ch === '"') inString = false;
      continue;
    }
    if (ch === '"') inString = true;
    else if (ch === '{') depth++;
    else if (ch === '}' && --depth === 0) break;
  }
  // Quote bare keys and drop trailing commas outside string literals only.
  const body = src
    .slice(i, j + 1)
    .split(/("(?:[^"\\]|\\.)*")/)
    .map((part, k) => (k % 2 ? part : part.replace(/([{,\s])([A-Za-z0-9_.-]+)\s*:/g, '$1"$2":').replace(/,(\s*})/g, '$1')))
    .join('');
  return JSON.parse(body);
}

function feature(slug) {
  const path = join(repo, '_features', `${slug}.md`);
  if (!existsSync(path)) return null;
  const src = readFileSync(path, 'utf8');
  const title = (src.match(/^title:\s*"(.*)"/m) || [])[1] || slug;
  return { slug, title, stats: block(src, 'stats'), notes: block(src, 'notes_by_num') || {} };
}

function support(f, family, platform) {
  const versions = f.stats?.[family]?.[platform];
  if (!versions) return { v: '?', note: 'not tested' };
  // The newest entry: versions are years, year-months or dotted numbers; compare numerically.
  const num = (k) => k.split(/[.-]/).map(Number);
  const keys = Object.keys(versions).sort((a, b) => {
    const [x, y] = [num(a), num(b)];
    for (let i = 0; i < Math.max(x.length, y.length); i++) if ((x[i] || 0) !== (y[i] || 0)) return (x[i] || 0) - (y[i] || 0);
    return 0;
  });
  const last = keys[keys.length - 1];
  const [flag, ...nums] = String(versions[last]).split(/\s+/);
  const note = nums.map((n) => f.notes[n.replace('#', '')]).filter(Boolean).join(' ');
  return { v: flag, version: last, note };
}

/** What the email uses, each mapped to the caniemail slug that covers it. */
function uses(html) {
  const out = new Map();
  const add = (slug, what) => {
    if (!out.has(slug)) out.set(slug, new Set());
    out.get(slug).add(what);
  };
  const decls = [];
  for (const m of html.matchAll(/style="([^"]*)"/g)) decls.push(...m[1].split(';').map((d) => ['inline', d]));
  const style = (html.match(/<style>([\s\S]*?)<\/style>/) || [])[1] || '';
  for (const m of style.matchAll(/\{([^{}]*)\}/g)) decls.push(...m[1].split(';').map((d) => ['style', d]));
  const untracked = new Set();
  for (const [where, d] of decls) {
    const [p, ...rest] = d.split(':');
    if (!p || !rest.length) continue;
    const prop = p.trim().replace(/&quot;/g, '"');
    const value = rest.join(':').trim().replace(/&quot;/g, '"');
    const what = `${prop}: ${value} (${where})`;
    const slug =
      prop.startsWith('padding') ? 'css-padding'
      : prop.startsWith('margin') ? 'css-margin'
      : prop === 'display' ? (value === 'none' ? 'css-display-none' : 'css-display')
      : prop === 'border' ? 'css-border'
      : `css-${prop}`;
    if (existsSync(join(repo, '_features', `${slug}.md`))) add(slug, what);
    else untracked.add(prop);
    if (/\d(px)\b/.test(value)) add('css-unit-px', what);
    if (/\d%/.test(value)) add('css-unit-percent', what);
    if (/\dem\b/.test(value)) add('css-unit-em', what);
    if (/!important/.test(value)) add('css-important', what);
    if (/rgba?\(/.test(value)) add(value.includes('rgba') ? 'css-rgba' : 'css-rgb', what);
    if (/system-ui/.test(value)) add('css-sytem-ui', what);
  }
  if (style) add('html-style', '<style> in <head>');
  if (/@media/.test(style)) add('css-at-media', '@media');
  if (/prefers-color-scheme/.test(style)) add('css-at-media-prefers-color-scheme', '@media (prefers-color-scheme: dark)');
  if (/:hover/.test(style)) add('css-pseudo-class-hover', ':hover');
  if (/\.r\d+\{/.test(style)) add('css-selector-class', '.rN class selectors');
  if (/<meta name="color-scheme"/.test(html)) add('html-meta-color-scheme', '<meta name="color-scheme">');
  const tags = { table: 'html-table', 'h1|h2': 'html-h1-h6', p: 'html-p', img: 'html-img', body: 'html-body' };
  for (const [t, slug] of Object.entries(tags)) if (new RegExp(`<(${t})[\\s>]`).test(html)) add(slug, `<${t}>`);
  const attrs = { role: 'html-role', align: 'html-align', valign: 'html-valign', width: 'html-width', height: 'html-height', cellpadding: 'html-cellpadding', cellspacing: 'html-cellspacing', target: 'html-target', lang: 'html-lang', dir: 'html-dir' };
  for (const [a, slug] of Object.entries(attrs)) if (new RegExp(`\\s${a}="`).test(html)) add(slug, `${a}=""`);
  // caniemail's html-doctype is the HTML5 doctype; react-email's XHTML 1.0 one is not tracked.
  if (/<!DOCTYPE html>/i.test(html)) add('html-doctype', '<!DOCTYPE html>');
  else if (/<!DOCTYPE/i.test(html)) untracked.add('XHTML 1.0 Transitional doctype');
  if (/<!--\[if mso\]>/.test(html)) add('html-comments', 'conditional comments');
  if (/\.png"/.test(html)) add('image-png', 'PNG image');
  if (/\sbgcolor="/.test(html)) untracked.add('bgcolor attribute');
  if (/:root/.test(style)) untracked.add(':root selector');
  return { used: out, untracked: [...untracked] };
}

// Is there a separate entry for the new Outlook for Windows anywhere? List every
// Outlook platform and version key the data holds.
{
  const { readdirSync } = await import('node:fs');
  const seen = {};
  for (const f of readdirSync(join(repo, '_features')).filter((f) => f.endsWith('.md'))) {
    const stats = feature(f.replace(/\.md$/, ''))?.stats?.outlook || {};
    for (const [plat, vs] of Object.entries(stats)) for (const v of Object.keys(vs)) (seen[plat] ||= new Set()).add(v);
  }
  console.log('Outlook keys in caniemail:', Object.entries(seen).map(([p, v]) => `${p}: ${[...v].sort().join(' ')}`).join(' | '));
}

const result = {};
for (const file of files) {
  const html = readFileSync(file, 'utf8');
  const { used, untracked } = uses(html);
  const rows = [];
  for (const [slug, whats] of used) {
    const f = feature(slug);
    if (!f) continue;
    const cells = CLIENTS.map(([name, fam, plat]) => ({ client: name, ...support(f, fam, plat) }));
    rows.push({ slug, title: f.title, uses: [...whats], cells });
  }
  rows.sort((a, b) => a.slug.localeCompare(b.slug));
  result[basename(file)] = { rows, untracked };

  console.log(`\n## ${basename(file)}\n`);
  console.log(`| feature | ${CLIENTS.map((c) => c[0]).join(' | ')} |`);
  console.log(`|---|${CLIENTS.map(() => '---').join('|')}|`);
  for (const r of rows) console.log(`| ${r.title} | ${r.cells.map((c) => c.v + (c.note ? '*' : '')).join(' | ')} |`);
  console.log('\nNot fully supported (n = no, a = partial, u = unknown):');
  for (const r of rows)
    for (const c of r.cells)
      if (c.v !== 'y') console.log(`- ${r.title} in ${c.client} [${c.v}, ${c.version ?? ''}]: ${c.note || 'no note'}  -- used as: ${r.uses.slice(0, 2).join('; ')}`);
  console.log(`\nNot in caniemail's data: ${untracked.join(', ')}`);
  console.log('\nCounts on the newest entry (n = unsupported, a = partial):');
  for (const [name] of CLIENTS) {
    const cells = rows.map((r) => ({ r, c: r.cells.find((c) => c.client === name) }));
    const n = cells.filter((x) => x.c.v === 'n').map((x) => x.r.title);
    const a = cells.filter((x) => x.c.v === 'a').length;
    console.log(`- ${name}: n ${n.length} (${n.join('; ')}), a ${a}`);
  }
}
writeFileSync('out/caniemail.json', JSON.stringify(result, null, 2));
