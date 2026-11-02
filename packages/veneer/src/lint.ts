import { existsSync, realpathSync } from "node:fs";
import path from "node:path";
import type * as TypeScript from "typescript";
import type { Diagnostic, Project } from "./read.ts";
import {
  compareCodePoints,
  generatedOffset,
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

function resolveTag(
  { ts, tc }: Reader,
  { tagName }: TypeScript.JsxOpeningLikeElement,
): TypeScript.Symbol | undefined {
  let symbol = tc.getSymbolAtLocation(
    ts.isPropertyAccessExpression(tagName) ? tagName.name : tagName,
  );
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

function bracketsInLiteral(
  reader: Reader,
  file: string,
  sourceFile: TypeScript.SourceFile,
  literal: TypeScript.StringLiteralLike,
): LintDiagnostic[] {
  // The literal as written, so an offset in it is an offset in the file.
  const written = literal.getText(sourceFile).slice(1, -1);
  return [...written.matchAll(/\S+/g)].flatMap((match): LintDiagnostic[] => {
    const [className] = match;
    if (!utilityOf(className).includes("[")) return [];
    const position = positionOf(reader, sourceFile, literal.getStart(sourceFile) + 1 + match.index);
    if (!position) return [];
    return [
      {
        file,
        line: position.line,
        column: position.column,
        endLine: position.line,
        endColumn: position.column + className.length,
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
): LintDiagnostic[] {
  const { ts } = reader;
  const found: LintDiagnostic[] = [];
  const visit = (node: TypeScript.Node): void => {
    if (ts.isJsxAttribute(node) && node.initializer) {
      const { name } = node;
      const attribute = ts.isJsxNamespacedName(name)
        ? `${name.namespace.text}:${name.name.text}`
        : name.text;
      if (CLASS_ATTRIBUTES.has(attribute)) {
        for (const literal of classLiterals(ts, node.initializer)) {
          found.push(...bracketsInLiteral(reader, file, sourceFile, literal));
        }
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  return found;
}

// ---- The call.

export async function lint(
  project: Project,
  options: { componentFolders: string[]; files?: string[] },
): Promise<LintDiagnostic[]> {
  const files =
    options.files ??
    project.checker
      .getRootFileNames()
      .filter((file) => file.endsWith(".tsx") || file.endsWith(".astro"))
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
    if (!isUnderFolder(declaredIn.fileName)) return undefined;

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

  const found: LintDiagnostic[] = [];
  for (const [file, diagnostics] of perFile) {
    const absolute = path.join(project.root, file);
    const sourceFile = program.getSourceFile(absolute);
    if (!sourceFile) continue;
    for (const diagnostic of diagnostics) {
      const reported = reportedUse(sourceFile, diagnostic);
      if (reported) found.push(reported);
    }
    if (!isUnderFolder(absolute)) found.push(...bracketValues(reader, file, sourceFile));
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
