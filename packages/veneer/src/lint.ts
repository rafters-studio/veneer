import { existsSync, realpathSync } from "node:fs";
import path from "node:path";
import type * as TypeScript from "typescript";
import type { Scan } from "./config.ts";
import type { Diagnostic, Project } from "./read.ts";
import {
  compareCodePoints,
  generatedOffset,
  holdsAllIntelligenceTags,
  ownPropNames,
  positionOf,
  relativeFile,
  scriptOf,
  type Reader,
} from "./resolve.ts";

type Ts = typeof TypeScript;

export interface LintDiagnostic extends Diagnostic {
  help?: string;
}

const CLASS_ATTRIBUTES = new Set(["class", "className", "class:list"]);
const MAX_RESOLVE_STEPS = 8;

const realPath = (file: string): string => (existsSync(file) ? realpathSync(file) : file);
const isClassesFile = (file: string): boolean => file.endsWith(".classes.ts");

// ---- Which components are rafters.

// The rafters shape, read from the file's text and the folder's names; nothing here runs.
function qualifies(reader: Reader, sourceFile: TypeScript.SourceFile): boolean {
  const { fileName } = sourceFile;
  const stem = fileName.replace(/\.(tsx|astro)$/, "");
  const { sys } = reader.ts;
  return (
    stem !== fileName &&
    sys.fileExists(`${stem}.behavior.ts`) &&
    sys.fileExists(`${stem}.classes.ts`) &&
    holdsAllIntelligenceTags(reader, sourceFile)
  );
}

// ---- Rule one: a use of a rafters component.

// A diagnostic's position counts lines as the language server does: \n, \r\n and \r only.
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
  // A diagnostic in top-level trivia, such as an unused @ts-expect-error, lands on the file itself.
  if (ts.isSourceFile(node)) return undefined;
  const { parent } = node;
  if (ts.isJsxAttribute(parent) && parent.name === node) return parent.parent.parent;
  const tag = ts.isPropertyAccessExpression(parent) && parent.name === node ? parent : node;
  const element = tag.parent;
  return ts.isJsxOpeningLikeElement(element) && element.tagName === tag ? element : undefined;
}

function resolveSymbol(
  { ts, tc }: Reader,
  start: TypeScript.Symbol | undefined,
): TypeScript.Symbol | undefined {
  let symbol = start;
  for (let step = 0; symbol && step < MAX_RESOLVE_STEPS; step += 1) {
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

function resolveTag(
  reader: Reader,
  { tagName }: TypeScript.JsxOpeningLikeElement,
): TypeScript.Symbol | undefined {
  const { ts, tc } = reader;
  return resolveSymbol(
    reader,
    tc.getSymbolAtLocation(ts.isPropertyAccessExpression(tagName) ? tagName.name : tagName),
  );
}

function deprecatedText(
  { ts, tc }: Reader,
  element: TypeScript.JsxOpeningLikeElement,
  name: string,
): string | undefined {
  const parameter = tc.getResolvedSignature(element)?.parameters[0];
  const property = parameter && tc.getTypeOfSymbol(parameter).getProperty(name);
  const tag = property?.getJsDocTags(tc).find((found) => found.name === "deprecated");
  return tag ? ts.displayPartsToString(tag.text) || undefined : undefined;
}

// ---- Rule two: a bracket value.

// The utility follows the last colon outside brackets, so a modifier's brackets are not its own.
function utilityOf(className: string): string {
  let depth = 0;
  let start = 0;
  for (let at = 0; at < className.length; at += 1) {
    const character = className[at];
    if (character === "[" || character === "(") depth += 1;
    else if (character === "]" || character === ")") depth -= 1;
    else if (character === ":" && depth === 0) start = at + 1;
  }
  return className.slice(start);
}

// What a const is initialized with, wherever the identifier's symbol leads.
function constInitializer(
  reader: Reader,
  identifier: TypeScript.Identifier,
): TypeScript.Expression | undefined {
  const { ts, tc } = reader;
  const declaration = resolveSymbol(reader, tc.getSymbolAtLocation(identifier))?.valueDeclaration;
  return declaration &&
    ts.isVariableDeclaration(declaration) &&
    ts.getCombinedNodeFlags(declaration) & ts.NodeFlags.Const
    ? declaration.initializer
    : undefined;
}

function constString(
  reader: Reader,
  identifier: TypeScript.Identifier,
): TypeScript.StringLiteralLike | undefined {
  const initializer = constInitializer(reader, identifier);
  return initializer && reader.ts.isStringLiteralLike(initializer) ? initializer : undefined;
}

// The string box.text holds, for a const box = { text: "..." }.
function constProperty(
  reader: Reader,
  { expression, name }: TypeScript.PropertyAccessExpression,
): TypeScript.StringLiteralLike | undefined {
  const { ts } = reader;
  let object = ts.isIdentifier(expression) ? constInitializer(reader, expression) : undefined;
  while (object && (ts.isAsExpression(object) || ts.isSatisfiesExpression(object))) {
    object = object.expression;
  }
  if (!object || !ts.isObjectLiteralExpression(object)) return undefined;
  for (const property of object.properties) {
    if (
      ts.isPropertyAssignment(property) &&
      ts.isIdentifier(property.name) &&
      property.name.text === name.text
    ) {
      return ts.isStringLiteralLike(property.initializer) ? property.initializer : undefined;
    }
  }
  return undefined;
}

// Each literal by the node a finding on it sits at: itself, or the use of a const written in
// another file, since a file's answer holds only that file. A map, because a node can be reached
// twice: a classy call in a class attribute, a const used twice.
type ClassStrings = Map<TypeScript.Node, TypeScript.StringLiteralLike>;

function collectClassStrings(
  reader: Reader,
  sourceFile: TypeScript.SourceFile,
  isOwnSource: (file: string) => boolean,
  node: TypeScript.Node,
  found: ClassStrings,
): void {
  const { ts } = reader;
  if (ts.isStringLiteralLike(node)) {
    found.set(node, node);
  } else if (ts.isIdentifier(node)) {
    const literal = constString(reader, node);
    const declaredIn = literal?.getSourceFile();
    if (!literal || !declaredIn) return;
    if (declaredIn === sourceFile) found.set(literal, literal);
    else if (isOwnSource(declaredIn.fileName)) found.set(node, literal);
  } else {
    const literal = ts.isPropertyAccessExpression(node) ? constProperty(reader, node) : undefined;
    const declaredIn = literal?.getSourceFile();
    if (literal && declaredIn && (declaredIn === sourceFile || isOwnSource(declaredIn.fileName))) {
      found.set(node, literal);
      return;
    }
    ts.forEachChild(node, (child) => {
      collectClassStrings(reader, sourceFile, isOwnSource, child, found);
    });
  }
}

function bracketsInLiteral(
  reader: Reader,
  file: string,
  sourceFile: TypeScript.SourceFile,
  at: TypeScript.Node,
  literal: TypeScript.StringLiteralLike,
): LintDiagnostic[] {
  // The literal as written, so an offset in it is an offset in the file.
  const written = literal.getText(literal.getSourceFile()).slice(1, -1);
  return [...written.matchAll(/\S+/g)].flatMap((match): LintDiagnostic[] => {
    const [className] = match;
    if (!utilityOf(className).includes("[")) return [];
    const inLiteral = at === literal;
    const position = positionOf(
      reader,
      sourceFile,
      at.getStart(sourceFile) + (inLiteral ? 1 + match.index : 0),
    );
    if (!position) return [];
    return [
      {
        file,
        line: position.line,
        column: position.column,
        endLine: position.line,
        endColumn: position.column + (inLiteral ? className.length : at.getWidth(sourceFile)),
        severity: "error",
        code: "rafters(arbitrary-value)",
        message: `${className} is an arbitrary value`,
      },
    ];
  });
}

function bracketValues(
  reader: Reader,
  file: string,
  sourceFile: TypeScript.SourceFile,
  isOwnSource: (file: string) => boolean,
): LintDiagnostic[] {
  const { ts } = reader;
  const strings: ClassStrings = new Map();
  const everyLiteral = isClassesFile(sourceFile.fileName);
  if (everyLiteral && !isOwnSource(sourceFile.fileName)) return [];
  const visit = (node: TypeScript.Node): void => {
    if (everyLiteral) {
      if (ts.isStringLiteralLike(node)) strings.set(node, node);
    } else if (ts.isJsxAttribute(node) && node.initializer) {
      const { name } = node;
      const attribute = ts.isJsxNamespacedName(name)
        ? `${name.namespace.text}:${name.name.text}`
        : name.text;
      if (CLASS_ATTRIBUTES.has(attribute)) {
        collectClassStrings(reader, sourceFile, isOwnSource, node.initializer, strings);
      }
    } else if (
      ts.isCallExpression(node) &&
      ts.isIdentifier(node.expression) &&
      node.expression.text === "classy"
    ) {
      for (const argument of node.arguments) {
        collectClassStrings(reader, sourceFile, isOwnSource, argument, strings);
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  return [...strings].flatMap(([at, literal]) =>
    bracketsInLiteral(reader, file, sourceFile, at, literal),
  );
}

// ---- Rule three: a fill or surface value.

const FILL_WORD = /^[a-z][a-z0-9]*(-[a-z0-9]+)*$/;
const FILL_TOKEN = /^bg-fill-[a-z][a-z0-9]*(-[a-z0-9]+)*$/;
const FILL_ALPHA = /^(100|[1-9]?[0-9])$/;
const AUTHORED_FILL = 'a fill the designer authors, fill="bg-fill-<name>"';

const isFillStop = (stop: string): boolean => {
  const [word = "", alpha, ...rest] = stop.split("/");
  return (
    FILL_WORD.test(word) && (alpha === undefined || FILL_ALPHA.test(alpha)) && rest.length === 0
  );
};

// The forms rafters' retired fill signature took. A bg-fill-* value is the current form.
function retiredFillForm(value: string): "bare" | "composed" | undefined {
  if (value.startsWith("bg-fill-")) return undefined;
  if (FILL_WORD.test(value) && !value.includes("-to-")) return "bare";
  return value.split("-to-").length <= 2 && value.split("-to-").every(isFillStop)
    ? "composed"
    : undefined;
}

type ValueFinding = Pick<LintDiagnostic, "code" | "message" | "help">;

// Asked for only when a bare word or a surface is found, so nothing else pays for the tokens or their code.
type Surfaces = { words: ReadonlySet<string> } | { failure: string };

async function loadSurfaces(root: string): Promise<Surfaces> {
  try {
    const { surfaceWords } = await import("./tokens.ts");
    return { words: surfaceWords(root) };
  } catch (error) {
    return { failure: error instanceof Error ? error.message : String(error) };
  }
}

function valueLiteral(
  ts: Ts,
  node: TypeScript.JsxAttribute,
): TypeScript.StringLiteralLike | undefined {
  const { initializer } = node;
  if (!initializer) return undefined;
  if (ts.isStringLiteral(initializer)) return initializer;
  const inner = ts.isJsxExpression(initializer) ? initializer.expression : undefined;
  return inner && ts.isStringLiteralLike(inner) ? inner : undefined;
}

function valueAttributes(
  { ts }: Reader,
  sourceFile: TypeScript.SourceFile,
): TypeScript.JsxAttribute[] {
  const found: TypeScript.JsxAttribute[] = [];
  const visit = (node: TypeScript.Node): void => {
    if (
      ts.isJsxAttribute(node) &&
      ts.isIdentifier(node.name) &&
      (node.name.text === "fill" || node.name.text === "surface")
    ) {
      found.push(node);
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  return found;
}

// ---- Rule four: a freehand div.

const FREEHAND_HELP = "use Container or Grid for layout";

function openingElements(
  { ts }: Reader,
  sourceFile: TypeScript.SourceFile,
): TypeScript.JsxOpeningLikeElement[] {
  const found: TypeScript.JsxOpeningLikeElement[] = [];
  const visit = (node: TypeScript.Node): void => {
    if (ts.isJsxOpeningLikeElement(node)) found.push(node);
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  return found;
}

const isDiv = (ts: Ts, { tagName }: TypeScript.JsxOpeningLikeElement): boolean =>
  ts.isIdentifier(tagName) && tagName.text === "div";

// ---- The call.

export async function lint(
  project: Project,
  options: { componentFolders: string[]; scan?: Scan; files?: string[] },
): Promise<LintDiagnostic[]> {
  const files =
    options.files?.filter((file) => !file.endsWith(".ts") || isClassesFile(file)) ??
    project.checker
      .getRootFileNames()
      .filter((file) => file.endsWith(".tsx") || file.endsWith(".astro") || isClassesFile(file))
      .map((file) => relativeFile(project.root, file));

  // diagnostics() replaces the program, so nothing reads it until they have all run.
  const perFile: [string, Diagnostic[]][] = [];
  for (const file of files) perFile.push([file, await project.diagnostics(file)]);

  const program = project.program();
  const reader: Reader = { project, ts: project.ts, tc: program.getTypeChecker() };
  const { ts } = reader;
  const folders = options.componentFolders.map(
    (folder) => realPath(path.resolve(project.root, folder)) + path.sep,
  );
  const isUnderFolder = (file: string): boolean => {
    const real = realPath(file);
    return folders.some((folder) => real.startsWith(folder));
  };
  const scan = options.scan ?? "format";
  const projectRoot = realPath(project.root);
  const isOwnSource = (file: string): boolean => {
    const [first, ...rest] = path.relative(projectRoot, realPath(file)).split(path.sep);
    return first !== ".." && first !== "node_modules" && !rest.includes("node_modules");
  };
  const isExempt = (sourceFile: TypeScript.SourceFile): boolean =>
    isUnderFolder(sourceFile.fileName) || qualifies(reader, sourceFile);
  const isCheckedUse = (declaredIn: TypeScript.SourceFile): boolean => {
    if (scan === "system") return isUnderFolder(declaredIn.fileName);
    return isExempt(declaredIn) || (scan === "all" && isOwnSource(declaredIn.fileName));
  };
  const propsHelp = new Map<TypeScript.Symbol, string>();

  const reportedUse = (
    sourceFile: TypeScript.SourceFile,
    diagnostic: Diagnostic,
  ): LintDiagnostic | undefined => {
    const offset = programOffset(reader, sourceFile, diagnostic);
    if (offset === undefined) return undefined;
    const node = innermostNode(ts, sourceFile, offset);
    const element = elementUsing(ts, node);
    const component = element && resolveTag(reader, element);
    const declaration = component?.valueDeclaration ?? component?.declarations?.[0];
    if (!element || !component || !declaration) return undefined;
    const declaredIn = declaration.getSourceFile();
    if (!isCheckedUse(declaredIn)) return undefined;

    let help: string | undefined;
    if (diagnostic.code === "ts(6385)") {
      help = deprecatedText(reader, element, node.getText(sourceFile));
    } else {
      help = propsHelp.get(component);
      if (help === undefined) {
        help = ownPropNames(reader, component, declaredIn).join(", ");
        propsHelp.set(component, help);
      }
    }
    return help ? { ...diagnostic, help } : diagnostic;
  };

  let surfaces: Promise<Surfaces> | undefined;
  const replacement = async (value: string, form: "bare" | "composed"): Promise<string> => {
    if (form === "composed") return AUTHORED_FILL;
    surfaces ??= loadSurfaces(project.root);
    const loaded = await surfaces;
    if ("failure" in loaded) {
      return `the project's tokens could not be loaded, so no replacement is named: ${loaded.failure}`;
    }
    return loaded.words.has(value) ? `surface="${value}"` : AUTHORED_FILL;
  };

  const fillFinding = async (value: string): Promise<ValueFinding | undefined> => {
    if (value === "" || FILL_TOKEN.test(value)) return undefined;
    const form = retiredFillForm(value);
    if (!form) {
      return {
        code: "rafters(invalid-fill)",
        message: `fill="${value}" is not exactly one bg-fill-* token`,
      };
    }
    return {
      code: "rafters(retired-fill)",
      message: `fill="${value}" is a retired fill value`,
      help: await replacement(value, form),
    };
  };

  const surfaceFinding = async (value: string): Promise<ValueFinding | undefined> => {
    if (value === "") return undefined;
    surfaces ??= loadSurfaces(project.root);
    const loaded = await surfaces;
    if ("failure" in loaded) {
      return {
        code: "rafters(unknown-surface)",
        message: `surface="${value}" could not be checked`,
        help: `the project's tokens could not be loaded, so no surface is named: ${loaded.failure}`,
      };
    }
    if (loaded.words.has(value)) return undefined;
    return {
      code: "rafters(unknown-surface)",
      message: `surface="${value}" is not a surface in the project's tokens`,
    };
  };

  const badValues = async (
    file: string,
    sourceFile: TypeScript.SourceFile,
  ): Promise<LintDiagnostic[]> => {
    const flagged: LintDiagnostic[] = [];
    for (const attribute of valueAttributes(reader, sourceFile)) {
      const literal = valueLiteral(ts, attribute);
      if (!literal) continue;
      const component = resolveTag(reader, attribute.parent.parent);
      const declaration = component?.valueDeclaration ?? component?.declarations?.[0];
      if (!declaration || !isCheckedUse(declaration.getSourceFile())) continue;
      const finding = await (attribute.name.getText(sourceFile) === "fill"
        ? fillFinding(literal.text)
        : surfaceFinding(literal.text));
      if (!finding) continue;
      const start = positionOf(reader, sourceFile, attribute.getStart(sourceFile));
      const end = positionOf(reader, sourceFile, attribute.end);
      if (!start || !end) continue;
      flagged.push({
        file,
        line: start.line,
        column: start.column,
        endLine: end.line,
        endColumn: end.column,
        severity: "error",
        ...finding,
      });
    }
    return flagged;
  };

  const isComponentUse = (element: TypeScript.JsxOpeningLikeElement): boolean => {
    if (isDiv(ts, element)) return false;
    const component = resolveTag(reader, element);
    const declaration = component?.valueDeclaration ?? component?.declarations?.[0];
    return declaration !== undefined && isCheckedUse(declaration.getSourceFile());
  };

  const freehandDivs = (file: string, sourceFile: TypeScript.SourceFile): LintDiagnostic[] => {
    const elements = openingElements(reader, sourceFile);
    const divs = elements.filter((element) => isDiv(ts, element));
    if (divs.length === 0 || !elements.some(isComponentUse)) return [];
    return divs.flatMap(({ tagName }): LintDiagnostic[] => {
      const start = positionOf(reader, sourceFile, tagName.getStart(sourceFile));
      const end = positionOf(reader, sourceFile, tagName.end);
      if (!start || !end) return [];
      return [
        {
          file,
          line: start.line,
          column: start.column,
          endLine: end.line,
          endColumn: end.column,
          severity: "warning",
          code: "rafters(freehand-div)",
          message: "a div laid by hand around rafters components",
          help: FREEHAND_HELP,
        },
      ];
    });
  };

  const found: LintDiagnostic[] = [];
  for (const [file, diagnostics] of perFile) {
    const absolute = path.join(project.root, file);
    const sourceFile = program.getSourceFile(absolute);
    if (!sourceFile) continue;
    for (const diagnostic of diagnostics) {
      const reported = reportedUse(sourceFile, diagnostic);
      if (reported) found.push(reported);
    }
    if (!isExempt(sourceFile)) {
      found.push(...bracketValues(reader, file, sourceFile, isOwnSource));
      found.push(...(await badValues(file, sourceFile)));
      found.push(...freehandDivs(file, sourceFile));
    }
  }
  return found.sort(
    (a, b) => compareCodePoints(a.file, b.file) || a.line - b.line || a.column - b.column,
  );
}

function firstLine(message: Diagnostic["message"]): string {
  const text: string = typeof message === "string" ? message : message.value;
  return text.split(/\r\n|\r|\n/, 1)[0] ?? "";
}

export function report(diagnostics: LintDiagnostic[]): string {
  const lines = diagnostics.flatMap(({ file, line, column, severity, code, message, help }) => {
    const entry = `${file}:${line}:${column} ${severity} ${code} ${firstLine(message)}`;
    return help === undefined ? [entry] : [entry, `  help: ${help}`];
  });
  const count = diagnostics.length;
  lines.push(count === 1 ? "1 problem" : `${count} problems`);
  return lines.map((line) => `${line}\n`).join("");
}
