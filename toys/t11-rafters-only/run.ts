// T11 driver. Usage: node run.ts <project root> [--plant <page> <file holding its unsaved text>]
// Prints JSON: every diagnostic, how each rule classifies it, and each marked case's result.
import { createRequire } from 'node:module';
import { readFileSync, realpathSync } from 'node:fs';
import { join, relative, resolve, sep } from 'node:path';
import type TS from 'typescript';

const root = realpathSync(resolve(process.argv[2]));
const here = import.meta.dirname;
const lsDir = realpathSync(join(here, 'node_modules/@astrojs/language-server'));
const req = createRequire(join(lsDir, 'package.json'));
const toyReq = createRequire(join(here, 'package.json'));
const ts: typeof TS = toyReq('typescript');
const { URI } = toyReq('vscode-uri');

// The capture FR-VEN-064 names: one more service plugin, on the module AstroCheck calls.
interface Ctx { inject(key: string): { getProgram(): TS.Program | undefined } }
let ctx: Ctx | undefined;
const kitLib = req('@volar/kit/lib/createChecker.js');
const orig = kitLib.createTypeScriptChecker;
kitLib.createTypeScriptChecker = (langs: unknown[], services: unknown[], ...rest: unknown[]) =>
  orig(langs, [...services, { name: 'capture', capabilities: {}, create(c: Ctx) { ctx = c; return {}; } }], ...rest);

const config = JSON.parse(readFileSync(join(root, '.rafters/config.rafters.json'), 'utf8'));
const entries: unknown[] = Array.isArray(config.componentsPath) ? config.componentsPath : [config.componentsPath];
const folders = entries.map((e) => realpathSync(join(root, typeof e === 'string' ? e : (e as { path: string }).path)) + sep);
const installed: string[] = config.installed?.components ?? [];

const { AstroCheck } = req('./dist/index.js');
const t0 = performance.now();
const ac = new AstroCheck(root, toyReq.resolve('typescript'), undefined);
const linter = ac.linter;

// Unsaved text, as FR-VEN-064 says: ts.sys answers, then fileUpdated.
const plantAt = process.argv.indexOf('--plant');
if (plantAt > 0) {
  const page = join(root, process.argv[plantAt + 1]);
  const text = readFileSync(resolve(process.argv[plantAt + 2]), 'utf8');
  const sys = ts.sys;
  const [read, mtime, exists] = [sys.readFile, sys.getModifiedTime!, sys.fileExists];
  sys.readFile = (p, e) => (p === page ? text : read(p, e));
  sys.getModifiedTime = (p) => (p === page ? new Date(Date.now()) : mtime(p));
  sys.fileExists = (p) => p === page || exists(p);
  linter.fileUpdated(page);
}

const files: string[] = linter.getRootFileNames().filter((f: string) => !/\.(vue|svelte)$/.test(f));
const raw: { file: string; d: { range: { start: { line: number; character: number }; end: { line: number; character: number } }; severity?: number; code?: unknown; source?: string; message: string } }[] = [];
for (const file of files) for (const d of await linter.check(file)) raw.push({ file, d });
const t1 = performance.now();

const program: TS.Program = ctx!.inject('typescript/languageService').getProgram()!;
const tc = program.getTypeChecker();

function sourceText(file: string): string {
  const s = linter.language.scripts.get(URI.file(file))?.snapshot;
  return s ? s.getText(0, s.getLength()) : '';
}
function offsetOf(text: string, line: number, ch: number): number {
  let o = 0;
  for (let i = 0; i < line; i++) o = text.indexOf('\n', o) + 1;
  return o + ch;
}
function programOffset(file: string, src: number): number | undefined {
  const script = linter.language.scripts.get(URI.file(file));
  const tsx = script?.generated?.embeddedCodes.get('tsx');
  if (!script || !tsx) return src;
  for (const [g] of linter.language.maps.get(tsx, script).toGeneratedLocation(src)) return g;
  return undefined;
}
function innermost(sf: TS.SourceFile, pos: number): TS.Node {
  let node: TS.Node = sf;
  for (;;) {
    let next: TS.Node | undefined;
    ts.forEachChild(node, (c) => { if (c.getStart(sf) <= pos && pos < c.end) next = c; });
    if (!next) return node;
    node = next;
  }
}
type Opening = TS.JsxOpeningElement | TS.JsxSelfClosingElement;
function useOf(node: TS.Node): Opening | undefined {
  const p = node.parent;
  if (p && ts.isJsxAttribute(p) && p.name === node) return p.parent.parent as Opening;
  let n: TS.Node = node;
  while (n.parent && ts.isPropertyAccessExpression(n.parent)) n = n.parent;
  const q = n.parent;
  if (q && (ts.isJsxOpeningElement(q) || ts.isJsxSelfClosingElement(q)) && q.tagName === n) return q;
  return undefined;
}
function realFile(decl: TS.Declaration): string {
  const f = decl.getSourceFile()?.fileName ?? ''; // a synthetic signature has no file
  try { return f ? realpathSync(f) : ''; } catch { return f; }
}
function resolveTag(el: Opening): TS.Declaration | undefined {
  const tag = ts.isPropertyAccessExpression(el.tagName) ? el.tagName.name : el.tagName;
  let sym = tc.getSymbolAtLocation(tag);
  for (let hops = 0; sym && hops < 8; hops++) {
    if (sym.flags & ts.SymbolFlags.Alias) { sym = tc.getAliasedSymbol(sym); continue; }
    const v = sym.valueDeclaration;
    if (v && ts.isVariableDeclaration(v) && v.initializer && ts.isIdentifier(v.initializer)) { sym = tc.getSymbolAtLocation(v.initializer); continue; }
    return v ?? sym.declarations?.[0];
  }
  return undefined;
}
const under = (f: string): boolean => folders.some((d) => f.startsWith(d));
const baseName = (f: string): string => f.slice(f.lastIndexOf('/') + 1).replace(/\.(astro|tsx)$/, '');

const SEV: Record<number, string> = { 1: 'error', 2: 'warning', 3: 'info', 4: 'hint' };
const t2 = performance.now();
const rows = raw.map(({ file, d }) => {
  const text = sourceText(file);
  const src = offsetOf(text, d.range.start.line, d.range.start.character);
  const gen = programOffset(file, src);
  const sf = program.getSourceFile(file);
  const row: Record<string, unknown> = {
    file: relative(root, file), line: d.range.start.line + 1, col: d.range.start.character + 1,
    severity: SEV[d.severity ?? 1], code: `${d.source ?? ''}(${d.code ?? ''})`, message: d.message.split('\n')[0],
    slice: text.slice(src, offsetOf(text, d.range.end.line, d.range.end.character)),
    isUse: false, symbol: false, signature: false, installed: false,
  };
  if (gen === undefined || !sf) { row.unmapped = true; return row; }
  const node = innermost(sf, gen);
  row.node = ts.SyntaxKind[node.kind];
  const el = useOf(node);
  if (!el) return row;
  row.isUse = true;
  const decl = resolveTag(el);
  const declFile = decl ? realFile(decl) : '';
  row.resolvedTo = declFile.startsWith(root) ? relative(root, declFile) : declFile.includes('node_modules') ? 'node_modules:' + declFile.split('node_modules/').at(-1) : declFile;
  row.symbol = under(declFile);
  row.installed = under(declFile) && installed.includes(baseName(declFile));
  const sigDecl = tc.getResolvedSignature(el)?.declaration;
  row.signature = !!sigDecl && !ts.isJSDocSignature(sigDecl) && under(realFile(sigDecl));
  if (String(d.code) === '6385') {
    const attr = node.parent && ts.isJsxAttribute(node.parent) ? tc.getSymbolAtLocation(node) : undefined;
    row.deprecatedAtAttribute = attr?.getJsDocTags(tc).filter((t) => t.name === 'deprecated').map((t) => ts.displayPartsToString(t.text));
    const param = tc.getResolvedSignature(el)?.parameters[0];
    const prop = param ? tc.getTypeOfSymbol(param).getProperty(node.getText(sf)) : undefined;
    row.deprecatedText = prop?.getJsDocTags(tc).filter((t) => t.name === 'deprecated').map((t) => ts.displayPartsToString(t.text));
  }
  return row;
});
const t3 = performance.now();

// Marked cases: a marker on the line above a case says what should be reported.
const cases: { id: string; expect: string; file: string; line: number; rows: typeof rows }[] = [];
for (const file of files) {
  sourceText(file).split('\n').forEach((l, i) => {
    const m = /(?:<!--|\/\/|\{\/\*)\s*([ACT]\d+) (rafters|other)\b/.exec(l);
    if (m) cases.push({ id: m[1], expect: m[2], file: relative(root, file), line: i + 2, rows: [] });
  });
}
for (const c of cases) c.rows = rows.filter((r) => r.file === c.file && r.line === c.line);
const verdict = (rule: string) => cases.map((c) => {
  const got = c.rows.some((r) => r[rule]) ? 'rafters' : 'other';
  return { id: c.id, expect: c.expect, got, ok: got === c.expect, hadDiagnostic: c.rows.length > 0 };
});
const score = (rule: string) => {
  const v = verdict(rule);
  return { right: v.filter((x) => x.ok).length, of: v.length, wrong: v.filter((x) => !x.ok).map((x) => `${x.id} expected ${x.expect}`) };
};

console.log(JSON.stringify({
  versions: { typescript: ac.ts.version, sameModule: ac.ts === ts },
  root, rootFiles: files.length, diagnostics: rows.length,
  timeMs: { constructAndCheck: Math.round(t1 - t0), classify: +(t3 - t2).toFixed(2) },
  scores: { symbol: score('symbol'), signature: score('signature'), installed: score('installed') },
  noDiagnostic: verdict('symbol').filter((x) => !x.hadDiagnostic).map((x) => x.id),
  reportedBySymbol: rows.filter((r) => r.symbol).map((r) => `${r.file}:${r.line}:${r.col} ${r.severity} ${r.code} ${r.slice}`),
  unmapped: rows.filter((r) => r.unmapped).length,
  cases: cases.map((c) => ({ id: c.id, expect: c.expect, at: `${c.file}:${c.line}`, rows: c.rows })),
  unmarked: rows.filter((r) => !cases.some((c) => c.file === r.file && c.line === r.line)),
}, null, 2));
