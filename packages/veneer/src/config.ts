import { readFile, stat } from "node:fs/promises";
import path from "node:path";

const CONFIG_FILE = ".rafters/config.rafters.json";

export interface Notice {
  code: string;
  message: string;
}

export interface VeneerError {
  code: string;
  message: string;
  field?: string;
  value?: unknown;
}

export interface DocsConfig {
  intent: string | null;
  // The entries that exist, each a path as the config wrote it.
  componentFolders: string[];
  notices: Notice[];
}

export type ConfigResult = ({ ok: true } & DocsConfig) | { ok: false; errors: VeneerError[] };

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function fieldError(field: string, problem: string, ...value: [unknown?]): VeneerError {
  const error: VeneerError = {
    code: "config-field",
    message: `${field}: ${problem}`,
    field,
  };
  if (value.length > 0) error.value = value[0];
  return error;
}

function errnoOf(error: unknown): string | undefined {
  return error instanceof Error && "code" in error && typeof error.code === "string"
    ? error.code
    : undefined;
}

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

async function readConfigObject(
  root: string,
): Promise<{ config: Record<string, unknown> } | { error: VeneerError }> {
  const unreadable = (problem: string) => ({
    error: { code: "config-unreadable", message: `${CONFIG_FILE}: ${problem}` },
  });
  let bytes: Buffer;
  try {
    bytes = await readFile(path.join(root, CONFIG_FILE));
  } catch (error) {
    const errno = errnoOf(error);
    if (errno === "ENOENT" || errno === "ENOTDIR") {
      return { error: { code: "config-missing", message: `${CONFIG_FILE} not found` } };
    }
    return unreadable(messageOf(error));
  }
  try {
    // A fatal decoder rejects bytes that are not UTF-8 and drops a leading U+FEFF.
    const parsed: unknown = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes));
    return isObject(parsed) ? { config: parsed } : unreadable("the text is not a JSON object");
  } catch (error) {
    return unreadable(messageOf(error));
  }
}

function entryPath(entry: unknown): string | undefined {
  if (typeof entry === "string") return entry;
  if (!isObject(entry) || typeof entry.path !== "string") return undefined;
  return !("root" in entry) || entry.root === true ? entry.path : undefined;
}

// What is wrong with the path, or whether the folder it names exists.
async function checkEntryPath(root: string, entry: string): Promise<string | boolean> {
  if (entry === "") return "the path is empty";
  if (path.isAbsolute(entry)) return "the path is absolute";
  const fromRoot = path.relative(root, path.resolve(root, entry));
  if (fromRoot === ".." || fromRoot.startsWith(`..${path.sep}`) || path.isAbsolute(fromRoot)) {
    return "the path is outside the project root";
  }
  try {
    return (await stat(path.resolve(root, entry))).isDirectory() || "the path is not a folder";
  } catch (error) {
    const errno = errnoOf(error);
    if (errno === "ENOENT" || errno === "ENOTDIR") return false;
    throw error;
  }
}

export async function readConfig(root: string): Promise<ConfigResult> {
  const read = await readConfigObject(root);
  if ("error" in read) return { ok: false, errors: [read.error] };
  const { config } = read;

  const errors: VeneerError[] = [];
  const componentFolders: string[] = [];
  const notices: Notice[] = [];
  const { componentsPath, intent, veneer } = config;

  if (componentsPath === undefined) {
    errors.push(fieldError("componentsPath", "the field is missing"));
  } else if (
    typeof componentsPath !== "string" &&
    !(Array.isArray(componentsPath) && componentsPath.length > 0)
  ) {
    errors.push(
      fieldError("componentsPath", "expected a string or a non-empty list", componentsPath),
    );
  } else {
    const entries: unknown[] = Array.isArray(componentsPath) ? componentsPath : [componentsPath];
    for (const [index, entry] of entries.entries()) {
      const field = Array.isArray(componentsPath) ? `componentsPath[${index}]` : "componentsPath";
      const written = entryPath(entry);
      if (written === undefined) {
        errors.push(fieldError(field, "expected a string or { path, root: true }", entry));
        continue;
      }
      const checked = await checkEntryPath(root, written);
      if (typeof checked === "string") {
        errors.push(fieldError(field, checked, entry));
      } else if (checked) {
        componentFolders.push(written);
      } else {
        notices.push({
          code: "components-folder-missing",
          message: `${written} named by componentsPath does not exist`,
        });
      }
    }
  }

  if (intent !== undefined && typeof intent !== "string") {
    errors.push(fieldError("intent", "expected a string", intent));
  }

  if (veneer !== undefined && !isObject(veneer)) {
    errors.push(fieldError("veneer", "expected an object", veneer));
  } else if (veneer?.mdxPath !== undefined && typeof veneer.mdxPath !== "string") {
    errors.push(fieldError("veneer.mdxPath", "expected a string", veneer.mdxPath));
  }

  if (errors.length > 0) return { ok: false, errors };
  return {
    ok: true,
    intent: typeof intent === "string" ? intent : null,
    componentFolders,
    notices,
  };
}
