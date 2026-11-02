import type { Dirent } from "node:fs";
import { readdir, stat } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import type * as TypeScript from "typescript";
import type { Notice } from "./config.ts";
import type { Project } from "./read.ts";

type Ts = typeof TypeScript;
type ScriptId = Parameters<Project["checker"]["language"]["scripts"]["get"]>[0];
type Value = string | number;

export interface Prop {
  name: string;
  type: string[];
  optional: boolean;
  default?: string;
  values?: Value[];
  doc?: string;
  file: string;
  line: number;
  column: number;
}

interface Tag {
  name: string;
  text: string;
}

interface Inherited {
  package: string;
  type: string;
  count: number;
}

export interface Component {
  name: string;
  file: string;
  resolved: boolean;
  reason?: string;
  description?: string;
  tags: Tag[];
  props: Prop[];
  inherited: Inherited[];
  inheritedDefaults: Record<string, string>;
}

interface Reader {
  project: Project;
  ts: Ts;
  tc: TypeScript.TypeChecker;
}

const require = createRequire(import.meta.url);
// The URI class the language server's own scripts are keyed with.
const { URI }: { URI: { file(path: string): ScriptId } } = createRequire(
  require.resolve("@astrojs/language-server/package.json"),
)("vscode-uri");

export function compareCodePoints(a: string, b: string): number {
  const left = [...a];
  const right = [...b];
  for (let i = 0; i < Math.min(left.length, right.length); i += 1) {
    const difference = (left[i]?.codePointAt(0) ?? 0) - (right[i]?.codePointAt(0) ?? 0);
    if (difference !== 0) return difference;
  }
  return left.length - right.length;
}

const posix = (file: string): string => file.split(path.sep).join("/");
const relativeFile = (root: string, file: string): string => posix(path.relative(root, file));
const inNodeModules = (declaration: TypeScript.Declaration): boolean =>
  declaration.getSourceFile().fileName.includes("/node_modules/");

// ---- The folders: the .tsx and .astro files under each componentsPath entry.

async function walkFolder(folder: string, into: Set<string>): Promise<void> {
  const entries: Dirent[] = await readdir(folder, { withFileTypes: true });
  for (const entry of entries) {
    const entryPath = path.join(folder, entry.name);
    let isFolder = entry.isDirectory();
    let isFile = entry.isFile();
    if (entry.isSymbolicLink()) {
      const target = await stat(entryPath).catch(() => undefined);
      isFile = target?.isFile() ?? false;
      isFolder = false; // A symbolic link to a folder is skipped.
    }
    if (isFolder) {
      if (entry.name !== "node_modules" && !entry.name.startsWith(".")) {
        await walkFolder(entryPath, into);
      }
    } else if (isFile && (entry.name.endsWith(".tsx") || entry.name.endsWith(".astro"))) {
      into.add(posix(entryPath));
    }
  }
}

// ---- The source map and positions.

function scriptOf(reader: Reader, file: string) {
  return reader.project.checker.language.scripts.get(URI.file(file));
}

function sourceOffset(reader: Reader, file: string, offset: number): number | undefined {
  const { language } = reader.project.checker;
  const script = scriptOf(reader, file);
  const tsx = script?.generated?.embeddedCodes.get("tsx");
  if (!script || !tsx) return undefined;
  for (const [source] of language.maps.get(tsx, script).toSourceLocation(offset)) return source;
  return undefined;
}

function positionOf(
  reader: Reader,
  sourceFile: TypeScript.SourceFile,
  offset: number,
): { line: number; column: number } | undefined {
  const file = sourceFile.fileName;
  let position: TypeScript.LineAndCharacter;
  if (file.endsWith(".astro")) {
    const source = sourceOffset(reader, file, offset);
    const snapshot = scriptOf(reader, file)?.snapshot;
    if (source === undefined || !snapshot) return undefined;
    const text = snapshot.getText(0, snapshot.getLength());
    position = reader.ts.createSourceMapSource(file, text).getLineAndCharacterOfPosition(source);
  } else {
    position = sourceFile.getLineAndCharacterOfPosition(offset);
  }
  return { line: position.line + 1, column: position.character + 1 };
}

// ---- Defaults.

function collectBindingDefaults(
  ts: Ts,
  pattern: TypeScript.ObjectBindingPattern,
  into: Map<string, string>,
): void {
  for (const element of pattern.elements) {
    if (element.dotDotDotToken || !element.initializer) continue;
    const key = element.propertyName ?? element.name;
    if (ts.isIdentifier(key) || ts.isStringLiteral(key)) {
      into.set(key.text, element.initializer.getText());
    }
  }
}

function renderFunction(
  { ts, tc }: Reader,
  start: TypeScript.Node | undefined,
):
  | TypeScript.FunctionDeclaration
  | TypeScript.FunctionExpression
  | TypeScript.ArrowFunction
  | undefined {
  const seen = new Set<TypeScript.Node>();
  let node = start;
  while (node && !seen.has(node)) {
    seen.add(node);
    if (
      ts.isFunctionDeclaration(node) ||
      ts.isFunctionExpression(node) ||
      ts.isArrowFunction(node)
    ) {
      return node;
    }
    if (ts.isVariableDeclaration(node)) node = node.initializer;
    else if (ts.isParenthesizedExpression(node) || ts.isAsExpression(node)) node = node.expression;
    else if (ts.isCallExpression(node)) node = node.arguments[0];
    else if (ts.isIdentifier(node)) {
      const declaration = tc.getSymbolAtLocation(node)?.valueDeclaration;
      const follows =
        declaration &&
        (ts.isVariableDeclaration(declaration) || ts.isFunctionDeclaration(declaration));
      node = follows ? declaration : undefined;
    } else return undefined;
  }
  return undefined;
}

function tsxDefaults(
  reader: Reader,
  declaration: TypeScript.Node | undefined,
): Map<string, string> {
  const { ts, tc } = reader;
  const defaults = new Map<string, string>();
  const render = renderFunction(reader, declaration);
  const parameter = render?.parameters[0];
  if (!parameter) return defaults;
  if (ts.isObjectBindingPattern(parameter.name)) {
    collectBindingDefaults(ts, parameter.name, defaults);
    return defaults;
  }
  const { body } = render;
  const parameterSymbol = tc.getSymbolAtLocation(parameter.name);
  if (!body || !parameterSymbol) return defaults;
  const visit = (node: TypeScript.Node): void => {
    if (ts.isFunctionLike(node)) return;
    if (
      ts.isVariableDeclaration(node) &&
      ts.isObjectBindingPattern(node.name) &&
      node.initializer &&
      ts.isIdentifier(node.initializer) &&
      tc.getSymbolAtLocation(node.initializer) === parameterSymbol
    ) {
      collectBindingDefaults(ts, node.name, defaults);
    }
    ts.forEachChild(node, visit);
  };
  ts.forEachChild(body, visit);
  return defaults;
}

function withoutAssertions(ts: Ts, node: TypeScript.Node): TypeScript.Node {
  let current = node;
  while (
    ts.isAsExpression(current) ||
    ts.isSatisfiesExpression(current) ||
    ts.isTypeAssertionExpression(current) ||
    ts.isNonNullExpression(current) ||
    ts.isParenthesizedExpression(current)
  ) {
    current = current.expression;
  }
  return current;
}

function astroDefaults(ts: Ts, sourceFile: TypeScript.SourceFile): Map<string, string> {
  const defaults = new Map<string, string>();
  const visit = (node: TypeScript.Node): void => {
    if (
      ts.isVariableDeclaration(node) &&
      ts.isObjectBindingPattern(node.name) &&
      node.initializer &&
      withoutAssertions(ts, node.initializer).getText() === "Astro.props"
    ) {
      collectBindingDefaults(ts, node.name, defaults);
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  return defaults;
}

// ---- Values.

function literalValuesOfTypes({ ts }: Reader, types: TypeScript.Type[]): Value[] | undefined {
  const values = new Map<string, Value>();
  for (const type of types) {
    for (const member of type.isUnion() ? type.types : [type]) {
      if (member.flags & (ts.TypeFlags.Undefined | ts.TypeFlags.Null | ts.TypeFlags.Never)) {
        continue;
      }
      if (!member.isStringLiteral() && !member.isNumberLiteral()) return undefined;
      values.set(valueKey(member.value), member.value);
    }
  }
  return values.size >= 2 ? [...values.values()] : undefined;
}

const valueKey = (value: Value): string => `${typeof value}:${value}`;

class WalkFailed extends Error {}

function resolvedDeclaration(
  { ts, tc }: Reader,
  name: TypeScript.Node,
): TypeScript.Declaration | undefined {
  const symbol = tc.getSymbolAtLocation(name);
  const resolved =
    symbol && symbol.flags & ts.SymbolFlags.Alias ? tc.getAliasedSymbol(symbol) : symbol;
  return resolved?.declarations?.[0];
}

function initializerOf(reader: Reader, name: TypeScript.Node): TypeScript.Node {
  const { ts } = reader;
  const declaration = resolvedDeclaration(reader, name);
  if (!declaration || !ts.isVariableDeclaration(declaration) || !declaration.initializer) {
    throw new WalkFailed();
  }
  return withoutAssertions(ts, declaration.initializer);
}

function literalValue(ts: Ts, node: TypeScript.Node): Value | null | undefined {
  if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) return node.text;
  if (ts.isNumericLiteral(node)) return Number(node.text);
  if (
    ts.isPrefixUnaryExpression(node) &&
    node.operator === ts.SyntaxKind.MinusToken &&
    ts.isNumericLiteral(node.operand)
  ) {
    return -Number(node.operand.text);
  }
  return node.kind === ts.SyntaxKind.NullKeyword ? null : undefined;
}

function walkArrayElements(
  reader: Reader,
  array: TypeScript.Node,
  walking: Set<TypeScript.Node>,
): Value[] {
  const { ts } = reader;
  if (!ts.isArrayLiteralExpression(array) || walking.has(array)) throw new WalkFailed();
  walking.add(array);
  const values = array.elements.flatMap((element) => {
    if (ts.isSpreadElement(element)) {
      return walkArrayElements(reader, initializerOf(reader, element.expression), walking);
    }
    const value = literalValue(ts, element);
    if (value === undefined || value === null) throw new WalkFailed();
    return [value];
  });
  walking.delete(array);
  return values;
}

function walkDeclaredType(
  reader: Reader,
  name: TypeScript.Node,
  walking: Set<TypeScript.Node>,
): Value[] {
  const { ts, tc } = reader;
  const declaration = resolvedDeclaration(reader, name);
  if (declaration && ts.isTypeAliasDeclaration(declaration)) {
    if (walking.has(declaration)) throw new WalkFailed();
    walking.add(declaration);
    const values = walkTypeNode(reader, declaration.type, walking);
    walking.delete(declaration);
    return values;
  }
  if (declaration && ts.isEnumDeclaration(declaration)) {
    return declaration.members.map((member) => {
      const value = tc.getConstantValue(member);
      if (value === undefined) throw new WalkFailed();
      return value;
    });
  }
  throw new WalkFailed();
}

function walkPropertyAnnotation(
  reader: Reader,
  object: TypeScript.TypeNode,
  key: string,
  walking: Set<TypeScript.Node>,
): Value[] {
  const { ts, tc } = reader;
  const annotated = tc
    .getPropertyOfType(tc.getTypeFromTypeNode(object), key)
    ?.declarations?.find(
      (declaration): declaration is TypeScript.PropertySignature | TypeScript.PropertyDeclaration =>
        (ts.isPropertySignature(declaration) || ts.isPropertyDeclaration(declaration)) &&
        declaration.type !== undefined,
    );
  if (!annotated?.type || walking.has(annotated)) throw new WalkFailed();
  walking.add(annotated);
  const values = walkTypeNode(reader, annotated.type, walking);
  walking.delete(annotated);
  return values;
}

function walkTypeNode(
  reader: Reader,
  node: TypeScript.TypeNode,
  walking: Set<TypeScript.Node>,
): Value[] {
  const { ts } = reader;
  if (ts.isParenthesizedTypeNode(node)) return walkTypeNode(reader, node.type, walking);
  if (ts.isUnionTypeNode(node)) {
    return node.types.flatMap((member) => walkTypeNode(reader, member, walking));
  }
  if (
    node.kind === ts.SyntaxKind.UndefinedKeyword ||
    node.kind === ts.SyntaxKind.NullKeyword ||
    node.kind === ts.SyntaxKind.NeverKeyword
  ) {
    return [];
  }
  if (ts.isLiteralTypeNode(node)) {
    const value = literalValue(ts, node.literal);
    if (value === undefined) throw new WalkFailed();
    return value === null ? [] : [value];
  }
  if (ts.isTypeReferenceNode(node)) return walkDeclaredType(reader, node.typeName, walking);
  if (ts.isImportTypeNode(node)) {
    if (!node.qualifier || node.isTypeOf) throw new WalkFailed();
    return walkDeclaredType(reader, node.qualifier, walking);
  }
  if (
    ts.isTypeOperatorNode(node) &&
    node.operator === ts.SyntaxKind.KeyOfKeyword &&
    ts.isTypeQueryNode(node.type)
  ) {
    const object = initializerOf(reader, node.type.exprName);
    if (!ts.isObjectLiteralExpression(object)) throw new WalkFailed();
    return object.properties.map((property) => {
      const key = property.name;
      if (key && (ts.isIdentifier(key) || ts.isStringLiteral(key))) return key.text;
      if (key && ts.isNumericLiteral(key)) return Number(key.text);
      throw new WalkFailed();
    });
  }
  if (ts.isIndexedAccessTypeNode(node)) {
    const object = ts.isParenthesizedTypeNode(node.objectType)
      ? node.objectType.type
      : node.objectType;
    if (ts.isTypeQueryNode(object) && node.indexType.kind === ts.SyntaxKind.NumberKeyword) {
      return walkArrayElements(reader, initializerOf(reader, object.exprName), walking);
    }
    if (ts.isLiteralTypeNode(node.indexType) && ts.isStringLiteral(node.indexType.literal)) {
      return walkPropertyAnnotation(reader, object, node.indexType.literal.text, walking);
    }
  }
  throw new WalkFailed();
}

function inCodePointOrder(values: Value[]): Value[] {
  return [...values].sort(
    (a, b) =>
      compareCodePoints(String(a), String(b)) ||
      Number(typeof a === "number") - Number(typeof b === "number"),
  );
}

function orderedValues(
  reader: Reader,
  values: Value[],
  declarations: TypeScript.Declaration[],
): Value[] {
  const { ts } = reader;
  const annotations = declarations.flatMap((declaration) =>
    (ts.isPropertySignature(declaration) || ts.isPropertyDeclaration(declaration)) &&
    declaration.type
      ? [declaration.type]
      : [],
  );
  if (annotations.length === 0) return inCodePointOrder(values);
  const walked = new Map<string, Value>();
  try {
    for (const annotation of annotations) {
      for (const value of walkTypeNode(reader, annotation, new Set())) {
        if (!walked.has(valueKey(value))) walked.set(valueKey(value), value);
      }
    }
  } catch (error) {
    if (error instanceof WalkFailed) return inCodePointOrder(values);
    throw error;
  }
  const exact =
    walked.size === values.length && values.every((value) => walked.has(valueKey(value)));
  return exact ? [...walked.values()] : inCodePointOrder(values);
}

// ---- Props.

function annotationText({ ts }: Reader, declaration: TypeScript.Declaration): string | undefined {
  if (ts.isMethodSignature(declaration)) {
    if (!declaration.type) return undefined;
    // parameters.pos is just past the opening parenthesis.
    return declaration
      .getSourceFile()
      .text.slice(declaration.parameters.pos - 1, declaration.type.end);
  }
  if (!("type" in declaration)) return undefined;
  const annotation = declaration.type as TypeScript.Node | undefined;
  return annotation && ts.isTypeNode(annotation) ? annotation.getText() : undefined;
}

function printedType({ ts, tc }: Reader, types: TypeScript.Type[]): string {
  const flags =
    ts.TypeFormatFlags.NoTruncation | ts.TypeFormatFlags.UseAliasDefinedOutsideCurrentScope;
  const print = (type: TypeScript.Type): string => tc.typeToString(type, undefined, flags);
  const printed = new Set(types.map(print));
  if (printed.size === 1) return [...printed].join("");
  const bare = new Set(types.map((type) => print(tc.getNonNullableType(type))));
  const parts = [...bare].map((text) => (text.includes("=>") ? `(${text})` : text));
  const includesUndefined = types.some((type) =>
    (type.isUnion() ? type.types : [type]).some((part) => part.flags & ts.TypeFlags.Undefined),
  );
  if (includesUndefined) parts.push("undefined");
  return parts.join(" | ");
}

function propType(
  reader: Reader,
  declarations: TypeScript.Declaration[],
  types: TypeScript.Type[],
): string[] {
  const annotations = declarations.map((declaration) => annotationText(reader, declaration));
  if (annotations.some((annotation) => annotation === undefined)) {
    return [printedType(reader, types)];
  }
  const entries = annotations.map((annotation) =>
    (annotation ?? "").replace(/\s+/g, " ").trim().replace(/^\| /, ""),
  );
  return [...new Set(entries)].filter((entry) => entry !== "never");
}

// TypeScript keeps only the first base's symbol when two bases declare a prop.
function declarationInBases(
  tc: TypeScript.TypeChecker,
  type: TypeScript.Type,
  name: string,
  seen = new Set<TypeScript.Type>(),
): TypeScript.Declaration | undefined {
  if (seen.has(type)) return undefined;
  seen.add(type);
  if (type.isUnionOrIntersection()) {
    for (const part of type.types) {
      const found = declarationInBases(tc, part, name, seen);
      if (found) return found;
    }
    return undefined;
  }
  const own = tc
    .getPropertyOfType(type, name)
    ?.declarations?.find((declaration) => !inNodeModules(declaration));
  if (own) return own;
  if (!type.isClassOrInterface()) return undefined;
  for (const base of tc.getBaseTypes(type)) {
    const found = declarationInBases(tc, base, name, seen);
    if (found) return found;
  }
  return undefined;
}

function packageOf(file: string): string {
  const segments = file
    .slice(file.lastIndexOf("/node_modules/") + "/node_modules/".length)
    .split("/");
  return (segments[0] ?? "").startsWith("@") ? segments.slice(0, 2).join("/") : (segments[0] ?? "");
}

function enclosingTypeName({ ts }: Reader, declaration: TypeScript.Declaration): string {
  let node: TypeScript.Node | undefined = declaration.parent;
  while (node) {
    if (ts.isInterfaceDeclaration(node) || ts.isTypeAliasDeclaration(node)) return node.name.text;
    node = node.parent;
  }
  return "(anonymous)";
}

interface ReadProps {
  props: Prop[];
  inherited: Inherited[];
  inheritedDefaults: Record<string, string>;
  anyNames: string[];
}

function readOwnProp(
  reader: Reader,
  name: string,
  symbols: TypeScript.Symbol[],
  recorded: TypeScript.Declaration,
  optional: boolean,
  fallback: string | undefined,
): Prop {
  const { ts, tc, project } = reader;
  const listed = symbols.flatMap((symbol) => symbol.declarations ?? []);
  const declarations = [
    ...new Set([...listed.filter((declaration) => !inNodeModules(declaration)), recorded]),
  ].sort(
    (a, b) =>
      compareCodePoints(a.getSourceFile().fileName, b.getSourceFile().fileName) || a.pos - b.pos,
  );
  const types = symbols.map((symbol) => tc.getTypeOfSymbol(symbol));
  const nameNode = ts.getNameOfDeclaration(recorded) ?? recorded;
  const sourceFile = recorded.getSourceFile();
  const file = relativeFile(project.root, sourceFile.fileName);
  const position = positionOf(reader, sourceFile, nameNode.getStart());
  if (!position) throw new Error(`no source position for prop ${name} in ${file}`);

  const prop: Prop = {
    name,
    type: propType(reader, declarations, types),
    optional,
    file,
    line: position.line,
    column: position.column,
  };
  if (fallback !== undefined) prop.default = fallback;
  const values = literalValuesOfTypes(reader, types);
  if (values) prop.values = orderedValues(reader, values, declarations);
  const documented =
    symbols.find((symbol) => symbol.declarations?.includes(recorded)) ??
    tc.getSymbolAtLocation(nameNode);
  const doc = documented ? ts.displayPartsToString(documented.getDocumentationComment(tc)) : "";
  if (doc !== "") prop.doc = doc;
  return prop;
}

function readProps(
  reader: Reader,
  parameterType: TypeScript.Type,
  sourceFile: TypeScript.SourceFile,
  defaults: Map<string, string>,
): ReadProps {
  const { ts, tc } = reader;
  const branches = parameterType.isUnion() ? parameterType.types : [parameterType];
  const grouped = new Map<string, TypeScript.Symbol[]>();
  for (const branch of branches) {
    for (const symbol of tc.getPropertiesOfType(branch)) {
      grouped.set(symbol.name, [...(grouped.get(symbol.name) ?? []), symbol]);
    }
  }

  const props: Prop[] = [];
  const inherited = new Map<string, Inherited>();
  const inheritedDefaults: [string, string][] = [];
  const anyNames: string[] = [];
  for (const [name, symbols] of grouped) {
    const listed = symbols.flatMap((symbol) => symbol.declarations ?? []);
    const recorded =
      listed.find((declaration) => declaration.getSourceFile() === sourceFile) ??
      listed.find((declaration) => !inNodeModules(declaration)) ??
      declarationInBases(tc, parameterType, name) ??
      listed[0];
    const fallback = defaults.get(name);

    if (recorded && !inNodeModules(recorded)) {
      const optional =
        symbols.length < branches.length ||
        symbols.some((symbol) => (symbol.flags & ts.SymbolFlags.Optional) !== 0);
      props.push(readOwnProp(reader, name, symbols, recorded, optional, fallback));
      if (symbols.some((symbol) => tc.getTypeOfSymbol(symbol).flags & ts.TypeFlags.Any)) {
        anyNames.push(name);
      }
      continue;
    }
    const pkg = recorded ? packageOf(recorded.getSourceFile().fileName) : "(synthetic)";
    const type = recorded ? enclosingTypeName(reader, recorded) : "(synthetic)";
    const key = `${pkg}\0${type}`;
    const group = inherited.get(key) ?? { package: pkg, type, count: 0 };
    group.count += 1;
    inherited.set(key, group);
    if (fallback !== undefined) inheritedDefaults.push([name, fallback]);
  }

  return {
    props: props.sort(
      (a, b) => compareCodePoints(a.file, b.file) || a.line - b.line || a.column - b.column,
    ),
    inherited: [...inherited.values()].sort(
      (a, b) => compareCodePoints(a.package, b.package) || compareCodePoints(a.type, b.type),
    ),
    inheritedDefaults: Object.fromEntries(
      inheritedDefaults.sort(([a], [b]) => compareCodePoints(a, b)),
    ),
    anyNames,
  };
}

// ---- Intelligence.

function intelligenceBlock(
  reader: Reader,
  sourceFile: TypeScript.SourceFile,
): TypeScript.JSDoc | undefined {
  const { ts } = reader;
  const blocks: TypeScript.JSDoc[] = [];
  const visit = (node: TypeScript.Node): void => {
    blocks.push(...((node as TypeScript.Node & { jsDoc?: TypeScript.JSDoc[] }).jsDoc ?? []));
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  const placed = blocks.flatMap((block) => {
    const start = block.getStart();
    const at = sourceFile.fileName.endsWith(".astro")
      ? sourceOffset(reader, sourceFile.fileName, start)
      : start;
    return at === undefined ? [] : [{ block, at }];
  });
  return placed.sort((a, b) => a.at - b.at)[0]?.block;
}

function intelligence(
  reader: Reader,
  sourceFile: TypeScript.SourceFile,
): { description?: string; tags: Tag[] } {
  const { ts } = reader;
  const block = intelligenceBlock(reader, sourceFile);
  if (!block) return { tags: [] };
  const tags = (block.tags ?? []).map((tag) => ({
    name: tag.tagName.text,
    text: ts.getTextOfJSDocComment(tag.comment) ?? "",
  }));
  const description = ts.getTextOfJSDocComment(block.comment);
  return description ? { description, tags } : { tags };
}

// ---- Unresolved.

async function parseErrorReason(
  reader: Reader,
  sourceFile: TypeScript.SourceFile,
): Promise<string | undefined> {
  const { ts, project } = reader;
  const file = sourceFile.fileName;
  if (!file.endsWith(".astro")) return undefined;
  const root: object | undefined = scriptOf(reader, file)?.generated?.root;
  const compiler = root && "compilerDiagnostics" in root ? root.compilerDiagnostics : undefined;
  const hasError =
    Array.isArray(compiler) &&
    compiler.some((diagnostic: { severity?: number }) => diagnostic.severity === 1);
  if (!hasError) return undefined;
  const syntactic = new Set(
    project
      .program()
      .getSyntacticDiagnostics(sourceFile)
      .map((diagnostic) => ts.flattenDiagnosticMessageText(diagnostic.messageText, "\n")),
  );
  const diagnostics = await project.checker.check(file);
  return diagnostics
    .filter(
      ({ severity, source, message }) =>
        (severity === undefined || severity === 1) &&
        (source === "astro" || (source === "ts" && syntactic.has(message))),
    )
    .sort(
      (a, b) =>
        a.range.start.line - b.range.start.line ||
        a.range.start.character - b.range.start.character,
    )
    .map(({ message }) => message)
    .join("; ");
}

// ---- Components.

async function componentsOfFile(reader: Reader, file: string): Promise<Component[]> {
  const { ts, tc, project } = reader;
  const sourceFile = project.program().getSourceFile(file);
  const moduleSymbol = sourceFile && tc.getSymbolAtLocation(sourceFile);
  if (!sourceFile || !moduleSymbol) return [];

  const exportNames = new Map<TypeScript.Symbol, string[]>();
  for (const exported of tc.getExportsOfModule(moduleSymbol)) {
    const symbol = exported.flags & ts.SymbolFlags.Alias ? tc.getAliasedSymbol(exported) : exported;
    if (symbol.flags & ts.SymbolFlags.Value) {
      exportNames.set(symbol, [...(exportNames.get(symbol) ?? []), exported.name]);
    }
  }

  const isAstro = file.endsWith(".astro");
  const relative = relativeFile(project.root, file);
  const { description, tags } = intelligence(reader, sourceFile);
  const parseReason = await parseErrorReason(reader, sourceFile);
  const components: Component[] = [];
  for (const [symbol, names] of exportNames) {
    const name = names.filter((n) => /^[A-Z]/.test(n)).sort(compareCodePoints)[0];
    const signature = tc.getTypeOfSymbol(symbol).getCallSignatures()[0];
    if (name === undefined || !signature) continue;

    const parameter = signature.parameters[0];
    let defaults = new Map<string, string>();
    if (!isAstro) defaults = tsxDefaults(reader, symbol.valueDeclaration);
    else if (names.includes("default")) defaults = astroDefaults(ts, sourceFile);
    const read: ReadProps = parameter
      ? readProps(reader, tc.getTypeOfSymbol(parameter), sourceFile, defaults)
      : { props: [], inherited: [], inheritedDefaults: {}, anyNames: [] };

    const component: Component = {
      name,
      file: relative,
      resolved: true,
      tags,
      props: read.props,
      inherited: read.inherited,
      inheritedDefaults: read.inheritedDefaults,
    };
    if (description !== undefined) component.description = description;
    if (parseReason !== undefined) {
      component.resolved = false;
      component.reason = parseReason;
    } else if (read.anyNames.length > 0) {
      component.resolved = false;
      component.reason = `props typed any: ${read.anyNames.join(", ")}`;
    }
    components.push(component);
  }
  return components;
}

export async function resolveComponents(
  project: Project,
  componentFolders: string[],
): Promise<{ components: Component[]; notices: Notice[] }> {
  const reader: Reader = { project, ts: project.ts, tc: project.program().getTypeChecker() };
  const files = new Set<string>();
  for (const folder of componentFolders) {
    await walkFolder(path.resolve(project.root, folder), files);
  }

  const rootFiles = new Set(project.checker.getRootFileNames().map(posix));
  const components: Component[] = [];
  const notices: Notice[] = [];
  for (const file of files) {
    if (rootFiles.has(file)) {
      components.push(...(await componentsOfFile(reader, file)));
    } else {
      notices.push({
        code: "outside-program",
        message: `${relativeFile(project.root, file)} is under componentsPath but not in the tsconfig's files`,
      });
    }
  }
  components.sort((a, b) => compareCodePoints(a.file, b.file) || compareCodePoints(a.name, b.name));
  return { components, notices };
}
