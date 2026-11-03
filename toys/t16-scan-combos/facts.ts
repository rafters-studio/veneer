// T16 facts. Usage: node facts.ts <site root> <out folder> [site name]
//
// Opens the project through veneer's openProject, runs veneer's lint as the baseline, then walks the
// same TypeScript program that lint walks and writes one row per JSX element. Every combination is
// then an offline query over those rows (combos.ts), so the program is opened once per site.
import { existsSync, mkdirSync, readFileSync, realpathSync, writeFileSync } from "node:fs";
import path from "node:path";
import { format } from "node:util";
import type * as TypeScript from "typescript";
import { readConfig } from "../../packages/veneer/src/config.ts";
import { lint } from "../../packages/veneer/src/lint.ts";
import { openProject, type Diagnostic } from "../../packages/veneer/src/read.ts";
import {
  generatedOffset,
  positionOf,
  relativeFile,
  scriptOf,
  type Reader,
} from "../../packages/veneer/src/resolve.ts";

type Ts = typeof TypeScript;

const [rootArg, outArg, nameArg] = process.argv.slice(2);
if (!rootArg || !outArg) throw new Error("usage: node facts.ts <site root> <out folder> [name]");
const root = realpathSync(path.resolve(rootArg));
const site = nameArg ?? path.basename(root);
const outDir = path.resolve(outArg);
mkdirSync(outDir, { recursive: true });

const SIX_TAGS = [
  "cognitive-load",
  "attention-economics",
  "trust-building",
  "accessibility",
  "semantic-meaning",
  "usage-patterns",
];

// ---- Copied from packages/veneer/src/lint.ts, where these are not exported. Keep them the same.

function offsetAt(text: string, line: number, column: number): number {
  const breaks = /\r\n|\r|\n/g;
  let start = 0;
  for (let at = 1; at < line && breaks.exec(text); at += 1) start = breaks.lastIndex;
  return start + column - 1;
}

function programOffset(
  reader: Reader,
  sourceFile: TypeScript.SourceFile,
  diagnostic: Diagnostic,
): number | undefined {
  const file = sourceFile.fileName;
  if (!file.endsWith(".astro")) {
    return offsetAt(sourceFile.text, diagnostic.line, diagnostic.column);
  }
  const snapshot = scriptOf(reader, file)?.snapshot;
  if (!snapshot) return undefined;
  const text = snapshot.getText(0, snapshot.getLength());
  return generatedOffset(reader, file, offsetAt(text, diagnostic.line, diagnostic.column));
}

function innermostNode(ts: Ts, sourceFile: TypeScript.SourceFile, offset: number): TypeScript.Node {
  let node: TypeScript.Node = sourceFile;
  for (;;) {
    let inner: TypeScript.Node | undefined;
    ts.forEachChild(node, (child) => {
      if (child.getStart(sourceFile) <= offset && offset < child.end) inner = child;
    });
    if (!inner) return node;
    node = inner;
  }
}

function elementUsing(ts: Ts, node: TypeScript.Node): TypeScript.JsxOpeningLikeElement | undefined {
  if (ts.isSourceFile(node)) return undefined;
  const { parent } = node;
  if (ts.isJsxAttribute(parent) && parent.name === node) return parent.parent.parent;
  const tag = ts.isPropertyAccessExpression(parent) && parent.name === node ? parent : node;
  const element = tag.parent;
  return ts.isJsxOpeningLikeElement(element) && element.tagName === tag ? element : undefined;
}

function resolveTag(
  { ts, tc }: Reader,
  { tagName }: TypeScript.JsxOpeningLikeElement,
): TypeScript.Symbol | undefined {
  let symbol = tc.getSymbolAtLocation(
    ts.isPropertyAccessExpression(tagName) ? tagName.name : tagName,
  );
  for (let step = 0; symbol && step < 8; step += 1) {
    const declaration = symbol.valueDeclaration;
    if (symbol.flags & ts.SymbolFlags.Alias) {
      symbol = tc.getAliasedSymbol(symbol);
    } else if (
      declaration &&
      ts.isVariableDeclaration(declaration) &&
      declaration.initializer &&
      ts.isIdentifier(declaration.initializer)
    ) {
      symbol = tc.getSymbolAtLocation(declaration.initializer);
    } else {
      return symbol;
    }
  }
  return undefined;
}

function classLiterals(ts: Ts, node: TypeScript.Node): TypeScript.StringLiteralLike[] {
  if (ts.isStringLiteralLike(node)) return [node];
  if (ts.isJsxExpression(node) || ts.isParenthesizedExpression(node)) {
    return node.expression ? classLiterals(ts, node.expression) : [];
  }
  if (ts.isArrayLiteralExpression(node)) {
    return node.elements.flatMap((element) => classLiterals(ts, element));
  }
  if (ts.isObjectLiteralExpression(node)) {
    return node.properties.flatMap((property) =>
      ts.isPropertyAssignment(property) && ts.isStringLiteral(property.name) ? [property.name] : [],
    );
  }
  return [];
}

const FILL_WORD = /^[a-z][a-z0-9]*(-[a-z0-9]+)*$/;
const FILL_ALPHA = /^(100|[1-9]?[0-9])$/;
const isFillStop = (stop: string): boolean => {
  const [word = "", alpha, ...rest] = stop.split("/");
  return (
    FILL_WORD.test(word) && (alpha === undefined || FILL_ALPHA.test(alpha)) && rest.length === 0
  );
};
function retiredFillForm(value: string): "bare" | "composed" | undefined {
  if (value.startsWith("bg-fill-")) return undefined;
  if (FILL_WORD.test(value) && !value.includes("-to-")) return "bare";
  return value.split("-to-").length <= 2 && value.split("-to-").every(isFillStop)
    ? "composed"
    : undefined;
}

// ---- The component shape, read from files. Runs none of the project's code.

interface Shape {
  file: string;
  behavior: boolean;
  classes: boolean;
  bestBlock: number;
  union: string[];
}

const shapes = new Map<string, Shape>();
function shapeOf(file: string): Shape {
  const known = shapes.get(file);
  if (known) return known;
  const ext = path.extname(file);
  const base = file.slice(0, -ext.length);
  let text = "";
  try {
    text = readFileSync(file, "utf8");
  } catch {
    text = "";
  }
  const union = new Set<string>();
  let bestBlock = 0;
  for (const [block] of text.matchAll(/\/\*\*[\s\S]*?\*\//g)) {
    const found = SIX_TAGS.filter((tag) => new RegExp(`@${tag}(?![a-z-])`).test(block));
    for (const tag of found) union.add(tag);
    bestBlock = Math.max(bestBlock, found.length);
  }
  const shape: Shape = {
    file: relativeFile(root, file),
    behavior: existsSync(`${base}.behavior.ts`),
    classes: existsSync(`${base}.classes.ts`),
    bestBlock,
    union: [...union],
  };
  shapes.set(file, shape);
  return shape;
}

// ---- The run.

const silence = (message: string): void => {
  writeFileSync(path.join(outDir, `${site}.log`), `${message}\n`, { flag: "a" });
};
console.info = (...args: unknown[]) => silence(format(...args));

const config = await readConfig(root);
if (!config.ok) throw new Error(`config: ${JSON.stringify(config.errors)}`);

const timing: Record<string, number> = {};
const lap = async <T>(name: string, work: () => Promise<T> | T): Promise<T> => {
  const start = performance.now();
  const result = await work();
  timing[name] = Math.round(performance.now() - start);
  return result;
};

const project = await lap("openMs", () => openProject(root));
const rootFiles = project.checker
  .getRootFileNames()
  .filter((file: string) => file.endsWith(".tsx") || file.endsWith(".astro"))
  .map((file: string) => relativeFile(root, file));

// The whole checker's answer per file, then veneer's answer, so the walk reads the final program.
const raw: Diagnostic[] = [];
await lap("rawDiagnosticsMs", async () => {
  for (const file of rootFiles) raw.push(...(await project.diagnostics(file)));
});
const baseline = await lap("lintMs", () =>
  lint(project, { componentFolders: config.componentFolders }),
);

const program = project.program();
const reader: Reader = { project, ts: project.ts, tc: program.getTypeChecker() };
const { ts } = reader;
const realPath = (file: string): string => (existsSync(file) ? realpathSync(file) : file);
const folders = config.componentFolders.map(
  (folder) => realPath(path.resolve(root, folder)) + path.sep,
);
const underFolder = (file: string): boolean => folders.some((f) => realPath(file).startsWith(f));

type Kind = "intrinsic" | "fragment" | "system" | "own" | "external" | "dynamic" | "unresolved";

interface Row {
  id: number;
  parent: number | null;
  file: string;
  line: number;
  column: number;
  tag: string;
  kind: Kind;
  declaredIn?: string;
  classes: string;
  classDynamic: boolean;
  attrs: string[];
  hasSpread: boolean;
  fill?: string;
  fillForm?: string;
  text: boolean;
  expressions: number;
  mapped: boolean;
}

function classify(element: TypeScript.JsxOpeningLikeElement): {
  kind: Kind;
  declaredIn?: string;
} {
  const { tagName } = element;
  const written = tagName.getText();
  if (written === "Fragment") return { kind: "fragment" };
  if (ts.isIdentifier(tagName) && /^[a-z]/.test(written)) return { kind: "intrinsic" };
  const symbol = resolveTag(reader, element);
  const declaration = symbol?.valueDeclaration ?? symbol?.declarations?.[0];
  if (!declaration) return { kind: "unresolved" };
  const declared = declaration.getSourceFile().fileName;
  const here = element.getSourceFile().fileName;
  if (declared.includes("/node_modules/")) return { kind: "external", declaredIn: declared.split("/node_modules/").pop() };
  if (underFolder(declared)) {
    shapeOf(declared);
    return { kind: "system", declaredIn: relativeFile(root, declared) };
  }
  // A local variable, parameter or binding: <Tag>, <Element>, <Content> and so on.
  if (declared === here && !ts.isFunctionDeclaration(declaration) && !ts.isSourceFile(declaration)) {
    return { kind: "dynamic", declaredIn: relativeFile(root, declared) };
  }
  shapeOf(declared);
  return { kind: "own", declaredIn: relativeFile(root, declared) };
}

const rows: Row[] = [];
const fileFacts: { file: string; underSystem: boolean; shape: Shape | null; elements: number }[] = [];

function walkFile(file: string, sourceFile: TypeScript.SourceFile): number {
  const before = rows.length;
  const visit = (node: TypeScript.Node, parent: number | null, mapped: boolean): void => {
    if (ts.isJsxElement(node) || ts.isJsxSelfClosingElement(node)) {
      const opening = ts.isJsxElement(node) ? node.openingElement : node;
      const { kind, declaredIn } = classify(opening);
      const start = opening.getStart(sourceFile);
      const position = positionOf(reader, sourceFile, start);
      if (kind === "fragment") {
        if (ts.isJsxElement(node)) for (const child of node.children) visit(child, parent, mapped);
        return;
      }
      const id = rows.length;
      let classes = "";
      let classDynamic = false;
      let hasSpread = false;
      const attrs: string[] = [];
      let fill: string | undefined;
      let fillForm: string | undefined;
      for (const attribute of opening.attributes.properties) {
        if (ts.isJsxSpreadAttribute(attribute)) {
          hasSpread = true;
          continue;
        }
        const name = ts.isJsxNamespacedName(attribute.name)
          ? `${attribute.name.namespace.text}:${attribute.name.name.text}`
          : attribute.name.text;
        attrs.push(name);
        if ((name === "class" || name === "className" || name === "class:list") && attribute.initializer) {
          const literals = classLiterals(ts, attribute.initializer);
          classes = literals.map((literal) => literal.text).join(" ");
          if (!ts.isStringLiteral(attribute.initializer)) classDynamic = true;
          if (ts.isJsxExpression(attribute.initializer) && literals.length > 0 && attribute.initializer.expression && ts.isStringLiteralLike(attribute.initializer.expression)) classDynamic = false;
        }
        if (name === "fill" && attribute.initializer) {
          const inner = ts.isJsxExpression(attribute.initializer) ? attribute.initializer.expression : attribute.initializer;
          if (inner && ts.isStringLiteralLike(inner)) {
            fill = inner.text;
            fillForm = retiredFillForm(inner.text);
          }
        }
      }
      let text = false;
      let expressions = 0;
      if (ts.isJsxElement(node)) {
        for (const child of node.children) {
          if (ts.isJsxText(child) && child.text.trim() !== "") text = true;
          if (ts.isJsxExpression(child) && child.expression) expressions += 1;
        }
      }
      rows.push({
        id,
        parent,
        file,
        line: position?.line ?? 0,
        column: position?.column ?? 0,
        tag: opening.tagName.getText(sourceFile),
        kind,
        ...(declaredIn ? { declaredIn } : {}),
        classes,
        classDynamic,
        attrs,
        hasSpread,
        ...(fill !== undefined ? { fill } : {}),
        ...(fillForm ? { fillForm } : {}),
        text,
        expressions,
        mapped,
      });
      if (ts.isJsxElement(node)) {
        for (const child of node.children) visit(child, id, mapped);
      }
      // A component's props can hold JSX too.
      ts.forEachChild(opening.attributes, (attribute) => visit(attribute, id, mapped));
      return;
    }
    if (ts.isJsxFragment(node)) {
      for (const child of node.children) visit(child, parent, mapped);
      return;
    }
    const isMap =
      ts.isCallExpression(node) &&
      ts.isPropertyAccessExpression(node.expression) &&
      node.expression.name.text === "map";
    ts.forEachChild(node, (child) => visit(child, parent, mapped || isMap));
  };
  visit(sourceFile, null, false);
  return rows.length - before;
}

await lap("walkMs", () => {
  for (const file of rootFiles) {
    const absolute = path.join(root, file);
    const sourceFile = program.getSourceFile(absolute);
    const count = sourceFile ? walkFile(file, sourceFile) : 0;
    fileFacts.push({
      file,
      underSystem: underFolder(absolute),
      shape: null,
      elements: count,
    });
  }
});

// Each root file's own shape, so a file can be exempted as a qualified component's own file.
for (const fact of fileFacts) fact.shape = shapeOf(path.join(root, fact.file));

// ---- Type errors, classified by the kind of the component they land on, as reportedUse does.

interface RawFinding {
  file: string;
  line: number;
  column: number;
  code: string;
  severity: string;
  message: string;
  kind: Kind | "none";
  declaredIn?: string;
}
const rawFindings: RawFinding[] = raw.map((diagnostic) => {
  const message = typeof diagnostic.message === "string" ? diagnostic.message : diagnostic.message.value;
  const base = {
    file: diagnostic.file,
    line: diagnostic.line,
    column: diagnostic.column,
    code: diagnostic.code,
    severity: diagnostic.severity,
    message: message.split("\n", 1)[0] ?? "",
  };
  const sourceFile = program.getSourceFile(path.join(root, diagnostic.file));
  const offset = sourceFile && programOffset(reader, sourceFile, diagnostic);
  if (!sourceFile || offset === undefined) return { ...base, kind: "none" as const };
  const element = elementUsing(ts, innermostNode(ts, sourceFile, offset));
  if (!element) return { ...base, kind: "none" as const };
  const { kind, declaredIn } = classify(element);
  return { ...base, kind, ...(declaredIn ? { declaredIn } : {}) };
});

project.close();

writeFileSync(path.join(outDir, `${site}.rows.jsonl`), rows.map((r) => JSON.stringify(r)).join("\n") + "\n");
writeFileSync(
  path.join(outDir, `${site}.meta.json`),
  `${JSON.stringify(
    {
      site,
      timing,
      rootFiles: rootFiles.length,
      componentFolders: config.componentFolders,
      files: fileFacts,
      shapes: [...shapes.values()],
      baseline: baseline.map(({ file, line, column, code, severity, message, help }) => ({
        file, line, column, code, severity, message: typeof message === "string" ? message : message.value, help,
      })),
      raw: rawFindings,
    },
    null,
    1,
  )}\n`,
);
console.log(
  `${site}: ${rootFiles.length} files, ${rows.length} elements, baseline ${baseline.length}, raw ${raw.length}, ${JSON.stringify(timing)}`,
);
