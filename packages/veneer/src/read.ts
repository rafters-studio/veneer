import { existsSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { format } from "node:util";
import { AstroCheck, type Diagnostic as LspDiagnostic } from "@astrojs/language-server";
import type * as ImportPackage from "@astrojs/language-server/dist/importPackage.js";
import type { LanguageServiceContext, LanguageServicePlugin } from "@volar/kit";
import type * as CreateChecker from "@volar/kit/lib/createChecker.js";
import type * as TypeScript from "typescript";

const NO_ASTRO_LINE =
  "seems to be an Astro project, but we couldn't find Astro or Astro is not installed";
const NO_INTEGRATION_LINE = "Couldn't find package @astrojs/";
const SEVERITIES = { 1: "error", 2: "warning", 3: "info", 4: "hint" } as const;

const require = createRequire(import.meta.url);
const typescriptPath = require.resolve("typescript");
// The same module AstroCheck loads from typescriptPath: its own `ts` member is private.
const ts: typeof TypeScript = require(typescriptPath);

export class ProjectError extends Error {
  readonly code: "already-open" | "tsconfig-missing" | "bad-path" | "not-root-file" | "closed";

  constructor(code: ProjectError["code"], message: string) {
    super(message);
    this.name = "ProjectError";
    this.code = code;
  }
}

export interface Diagnostic {
  file: string;
  line: number;
  column: number;
  endLine: number;
  endColumn: number;
  severity: "error" | "warning" | "info" | "hint";
  code: string;
  message: LspDiagnostic["message"];
}

export interface Project {
  readonly ts: typeof TypeScript;
  readonly checker: AstroCheck["linter"];
  readonly root: string;
  program(): TypeScript.Program;
  diagnostics(file: string): Promise<Diagnostic[]>;
  setUnsaved(file: string, text: string): void;
  clearUnsaved(file: string): void;
  close(): void;
}

let aProjectIsOpen = false;
let serverIsPrepared = false;
let constructedContext: LanguageServiceContext | undefined;
// Module-wide, because kit's snapshot cache outlives a Project and compares modified times.
let unsavedChanges = 0;

// Volar reaches the program only through a service plugin's context. A project with references
// creates its plugins once per tsconfig, the root's first, and the program is the root's.
const capturePlugin: LanguageServicePlugin = {
  name: "veneer-capture-context",
  capabilities: {},
  create(context) {
    constructedContext ??= context;
    return {};
  },
};

function prepareLanguageServer(): void {
  if (serverIsPrepared) return;
  serverIsPrepared = true;
  const serverRequire = createRequire(require.resolve("@astrojs/language-server/package.json"));
  // A project is untrusted: the server must not load an editor integration from it.
  const { setIsTrusted }: typeof ImportPackage = serverRequire("./dist/importPackage.js");
  setIsTrusted(false);
  const kit: typeof CreateChecker = serverRequire("@volar/kit/lib/createChecker.js");
  const original = kit.createTypeScriptChecker;
  kit.createTypeScriptChecker = (languages, services, ...rest) =>
    original(languages, [...services, capturePlugin], ...rest);
}

function constructAstroCheck(root: string): AstroCheck {
  const consoleError = console.error;
  console.error = (...args: unknown[]) => {
    if (!format(...args).endsWith(NO_ASTRO_LINE)) consoleError(...args);
  };
  try {
    return new AstroCheck(root, typescriptPath, undefined);
  } finally {
    console.error = consoleError;
  }
}

function overlaySys(unsaved: Map<string, { text: string; modified: Date }>): () => void {
  const { sys } = ts;
  const { readFile, getModifiedTime, fileExists } = sys;
  sys.readFile = (file, encoding) => unsaved.get(file)?.text ?? readFile.call(sys, file, encoding);
  sys.getModifiedTime = (file) => unsaved.get(file)?.modified ?? getModifiedTime?.call(sys, file);
  sys.fileExists = (file) => unsaved.has(file) || fileExists.call(sys, file);
  return () => {
    sys.readFile = readFile;
    sys.getModifiedTime = getModifiedTime;
    sys.fileExists = fileExists;
  };
}

function programOf(context: LanguageServiceContext | undefined): TypeScript.Program {
  const program = context
    ?.inject<{ "typescript/languageService": () => TypeScript.LanguageService }>(
      "typescript/languageService",
    )
    ?.getProgram();
  if (!program) throw new Error("the language service holds no program");
  return program;
}

function toDiagnostic(file: string, diagnostic: LspDiagnostic): Diagnostic {
  const { range, severity, source, code, message } = diagnostic;
  return {
    file,
    line: range.start.line + 1,
    column: range.start.character + 1,
    endLine: range.end.line + 1,
    endColumn: range.end.character + 1,
    severity: SEVERITIES[severity ?? 1],
    code: `${source ?? ""}(${code ?? ""})`,
    message,
  };
}

function isProjectRelative(file: string): boolean {
  return (
    !path.isAbsolute(file) &&
    !file.includes("\\") &&
    file === path.posix.normalize(file) &&
    !file.startsWith("../")
  );
}

export async function openProject(projectRoot: string): Promise<Project> {
  if (aProjectIsOpen) {
    throw new ProjectError("already-open", "a veneer project is already open in this process");
  }
  const root = path.resolve(projectRoot);
  if (!existsSync(path.join(root, "tsconfig.json"))) {
    throw new ProjectError("tsconfig-missing", `tsconfig.json not found at ${root}`);
  }

  aProjectIsOpen = true;
  const consoleInfo = console.info;
  console.info = (...args: unknown[]) => {
    if (!format(...args).startsWith(NO_INTEGRATION_LINE)) consoleInfo(...args);
  };
  const unsaved = new Map<string, { text: string; modified: Date }>();
  const restoreSys = overlaySys(unsaved);
  try {
    prepareLanguageServer();
    constructedContext = undefined;
    const astroCheck = constructAstroCheck(root);
    let context: LanguageServiceContext | undefined = constructedContext;
    constructedContext = undefined;
    const fileNames = astroCheck.linter.getRootFileNames().slice(0, 1);
    await astroCheck.lint({ fileNames });
    let program = programOf(context);
    let closed = false;

    const rootFile = (file: string): string => {
      if (closed) throw new ProjectError("closed", "the veneer project is closed");
      if (!isProjectRelative(file)) {
        throw new ProjectError(
          "bad-path",
          `${file} is not a project-relative path with / separators`,
        );
      }
      const absolute = path.join(root, file);
      if (!astroCheck.linter.getRootFileNames().includes(absolute)) {
        throw new ProjectError("not-root-file", `${file} is not a root file of the program`);
      }
      return absolute;
    };

    return {
      ts,
      checker: astroCheck.linter,
      root,
      program() {
        if (closed) throw new ProjectError("closed", "the veneer project is closed");
        return program;
      },
      async diagnostics(file) {
        const absolute = rootFile(file);
        const { fileResult } = await astroCheck.lint({ fileNames: [absolute] });
        program = programOf(context);
        return fileResult.flatMap(({ errors }) => errors.map((d) => toDiagnostic(file, d)));
      },
      setUnsaved(file, text) {
        const absolute = rootFile(file);
        unsavedChanges += 1;
        unsaved.set(absolute, { text, modified: new Date(-unsavedChanges) });
        astroCheck.linter.fileUpdated(absolute);
      },
      clearUnsaved(file) {
        const absolute = rootFile(file);
        unsaved.delete(absolute);
        astroCheck.linter.fileUpdated(absolute);
      },
      close() {
        if (closed) throw new ProjectError("closed", "the veneer project is closed");
        closed = true;
        unsaved.clear();
        restoreSys();
        context = undefined;
        aProjectIsOpen = false;
      },
    };
  } catch (error) {
    restoreSys();
    aProjectIsOpen = false;
    throw error;
  } finally {
    console.info = consoleInfo;
  }
}
