import { createHash } from "node:crypto";
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
// matchFiles is internal to TypeScript, which FR-VEN-064 pins exactly.
const ts: typeof TypeScript & { matchFiles(...args: unknown[]): string[] } = require(
  typescriptPath,
);

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
  kit.createTypeScriptChecker = (languages, services, tsconfig, references, setup) =>
    original(languages, [...services, capturePlugin], tsconfig, references, (args) => {
      setup?.(args);
      versionByContent(args.project.typescript.languageServiceHost);
    });
}

// Every language service shares one TypeScript DocumentRegistry, which reuses a stored file when
// the version string matches and never compares text. Each open restarts its per-file counters
// at "0", so a later open would answer from the first text the process saw (#164).
const contentVersions = new WeakMap<TypeScript.IScriptSnapshot, string>();
function versionByContent(host: TypeScript.LanguageServiceHost): void {
  const versionOf = host.getScriptVersion.bind(host);
  host.getScriptVersion = (file) => {
    const version = versionOf(file);
    const snapshot = version === "" ? undefined : host.getScriptSnapshot(file);
    if (!snapshot) return version;
    let hash = contentVersions.get(snapshot);
    if (hash === undefined) {
      hash = createHash("sha1").update(snapshot.getText(0, snapshot.getLength())).digest("base64");
      contentVersions.set(snapshot, hash);
    }
    return hash;
  };
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

// A file system holding only the file, for ts.matchFiles.
function holdingOnly(file: string) {
  return (directory: string): { files: string[]; directories: string[] } => {
    const prefix = directory.endsWith("/") ? directory : `${directory}/`;
    if (!file.startsWith(prefix)) return { files: [], directories: [] };
    const [next = "", ...below] = file.slice(prefix.length).split("/");
    return below.length === 0
      ? { files: [next], directories: [] }
      : { files: [], directories: [next] };
  };
}

function overlaySys(unsaved: Map<string, { text: string; modified: Date }>): () => void {
  const { sys } = ts;
  const { readFile, getModifiedTime, fileExists, readDirectory } = sys;
  sys.readFile = (file, encoding) => unsaved.get(file)?.text ?? readFile.call(sys, file, encoding);
  sys.getModifiedTime = (file) => unsaved.get(file)?.modified ?? getModifiedTime?.call(sys, file);
  sys.fileExists = (file) => unsaved.has(file) || fileExists.call(sys, file);
  sys.readDirectory = (directory, extensions, excludes, includes, depth) => {
    const found = readDirectory.call(sys, directory, extensions, excludes, includes, depth);
    const taken = [...unsaved.keys()].filter(
      (file) =>
        !found.includes(file) &&
        ts.matchFiles(
          directory,
          extensions,
          excludes,
          includes,
          sys.useCaseSensitiveFileNames,
          sys.getCurrentDirectory(),
          depth,
          holdingOnly(file),
          (name: string) => name,
        ).length > 0,
    );
    return [...found, ...taken];
  };
  return () => {
    sys.readFile = readFile;
    sys.getModifiedTime = getModifiedTime;
    sys.fileExists = fileExists;
    sys.readDirectory = readDirectory;
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

function badPath(file: string): ProjectError {
  return new ProjectError("bad-path", `${file} is not a project-relative path with / separators`);
}

function unsavedEntry(text: string): { text: string; modified: Date } {
  unsavedChanges += 1;
  return { text, modified: new Date(-unsavedChanges) };
}

export async function openProject(
  projectRoot: string,
  options: { unsaved?: Record<string, string> } = {},
): Promise<Project> {
  if (aProjectIsOpen) {
    throw new ProjectError("already-open", "a veneer project is already open in this process");
  }
  const root = path.resolve(projectRoot);
  if (!existsSync(path.join(root, "tsconfig.json"))) {
    throw new ProjectError("tsconfig-missing", `tsconfig.json not found at ${root}`);
  }
  const unsaved = new Map<string, { text: string; modified: Date }>();
  for (const [file, text] of Object.entries(options.unsaved ?? {})) {
    if (!isProjectRelative(file)) throw badPath(file);
    unsaved.set(path.join(root, file), unsavedEntry(text));
  }

  aProjectIsOpen = true;
  const consoleInfo = console.info;
  console.info = (...args: unknown[]) => {
    if (!format(...args).startsWith(NO_INTEGRATION_LINE)) consoleInfo(...args);
  };
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
      if (!isProjectRelative(file)) throw badPath(file);
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
        unsaved.set(absolute, unsavedEntry(text));
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
