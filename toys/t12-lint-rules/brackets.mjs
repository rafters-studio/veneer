// node brackets.mjs <dist/index.mjs> <project> [componentsFolder]: finds bracket utilities in class attributes.
import { createRequire } from "node:module";
import { readFileSync, realpathSync } from "node:fs";
import { join, relative, sep } from "node:path";
const [dist, root, folderArg] = process.argv.slice(2);
const { openProject } = await import(dist);
const p = await openProject(root);
const { ts, checker } = p;
const req = createRequire(join(dist, ".."));
const { URI } = createRequire(req.resolve("@astrojs/language-server/package.json"))("vscode-uri");
const config = folderArg ?? JSON.parse(readFileSync(join(root, ".rafters/config.rafters.json"), "utf8")).componentsPath;
const folder = realpathSync(join(root, typeof config === "string" ? config : config[0].path ?? config[0])) + sep;

// The utility is what follows the last colon outside brackets. Brackets in it make it arbitrary.
function utilityOf(cls) {
  let depth = 0, start = 0;
  for (let i = 0; i < cls.length; i++) {
    if (cls[i] === "[" || cls[i] === "(") depth++;
    else if (cls[i] === "]" || cls[i] === ")") depth--;
    else if (cls[i] === ":" && depth === 0) start = i + 1;
  }
  return cls.slice(start);
}
const isBracketed = (cls) => utilityOf(cls).includes("[");

function classLiterals(sf) {
  const found = [];
  const attrName = (a) => (ts.isJsxNamespacedName?.(a.name) ? `${a.name.namespace.text}:${a.name.name.text}` : a.name.getText(sf));
  const strings = (n) => {
    if (ts.isStringLiteralLike(n)) found.push(n);
    else if (ts.isArrayLiteralExpression(n)) n.elements.forEach(strings);
    else if (ts.isObjectLiteralExpression(n)) { for (const q of n.properties) if (ts.isPropertyAssignment(q) && ts.isStringLiteralLike(q.name)) found.push(q.name); }
    else if (ts.isParenthesizedExpression(n) || ts.isJsxExpression(n)) { if (n.expression) strings(n.expression); }
  };
  const visit = (n) => {
    if (ts.isJsxAttribute(n) && ["class", "className", "class:list"].includes(attrName(n)) && n.initializer) strings(n.initializer);
    ts.forEachChild(n, visit);
  };
  visit(sf);
  return found;
}
function source(file, gen) {
  const script = checker.language.scripts.get(URI.file(file));
  const tsx = script?.generated?.embeddedCodes.get("tsx");
  if (!script || !tsx) return gen;
  for (const [s] of checker.language.maps.get(tsx, script).toSourceLocation(gen)) return s;
  return undefined;
}
const t0 = performance.now();
const results = [];
let skippedFiles = 0, literals = 0;
for (const file of checker.getRootFileNames()) {
  if (!/\.(astro|tsx)$/.test(file)) continue;
  if (realpathSync(file).startsWith(folder)) { skippedFiles++; continue; }
  const sf = p.program().getSourceFile(file);
  if (!sf) continue;
  const text = readFileSync(file, "utf8");
  for (const lit of classLiterals(sf)) {
    literals++;
    const raw = lit.getText(sf).slice(1, -1);
    for (const m of raw.matchAll(/\S+/g)) {
      if (!isBracketed(m[0])) continue;
      const genAt = lit.getStart(sf) + 1 + m.index;
      const at = source(file, genAt);
      const pos = at === undefined ? null : text.slice(0, at).split("\n");
      results.push({ file: relative(root, file), cls: m[0], line: pos ? pos.length : null, col: pos ? pos.at(-1).length + 1 : null, atSource: at === undefined ? null : text.slice(at, at + m[0].length) === m[0] });
    }
  }
}
console.log(JSON.stringify({ ms: +(performance.now() - t0).toFixed(1), skippedFiles, literals, flagged: results.length, results }, null, 0));
p.close();
process.exit(0);
