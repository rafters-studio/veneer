// node dist/worker.mjs <project root>: the worker behind veneer-lsp. JSON lines on stdin and stdout.
import { execFile } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import { createInterface } from "node:readline";
import { promisify } from "node:util";
import type { CheckResult } from "./check.ts";
import { readConfig, type VeneerError } from "./config.ts";
import { lint } from "./lint.ts";
import { openProject } from "./read.ts";

type Line =
  | { seq: number; file: string; text: string }
  | { seq: number; close: string }
  | { seq: number; save: true };

// stdout carries only the protocol's lines.
const SILENCE = "console.log = console.info = console.debug = () => {};";
console.log = console.info = console.debug = () => {};

const run = promisify(execFile);

const write = (message: object): void => {
  process.stdout.write(`${JSON.stringify(message)}\n`);
};

const messages = (errors: VeneerError[]): string => errors.map((e) => e.message).join("; ");

// A child process, because one project is open per process. A save reads the disk.
async function save(root: string): Promise<CheckResult & { ok: true }> {
  const index = new URL("./index.mjs", import.meta.url).href;
  const script = `${SILENCE}
const { check } = await import(${JSON.stringify(index)});
process.stdout.write(JSON.stringify(await check(${JSON.stringify(root)}, { save: true })));
`;
  const { stdout } = await run(process.execPath, ["--input-type=module", "--eval", script], {
    maxBuffer: Number.POSITIVE_INFINITY,
  });
  const result: CheckResult = JSON.parse(stdout);
  if (!result.ok) throw new Error(messages(result.errors));
  return result;
}

async function serve(root: string): Promise<void> {
  const config = await readConfig(root);
  if (!config.ok) throw new Error(messages(config.errors));
  const { componentFolders, scan } = config;
  let project = await openProject(root);
  const unsaved = new Map<string, string>();
  // Files the tsconfig does not take. Remembered, so typing in one does not reopen the project.
  const outside = new Set<string>();
  const isRoot = (file: string): boolean =>
    project.checker.getRootFileNames().includes(path.join(project.root, file));

  // One project opens per process, so a reopen closes first. If the open then fails, the next
  // line tries again rather than using the closed project.
  let isOpen = true;
  const reopen = async (): Promise<void> => {
    if (isOpen) project.close();
    isOpen = false;
    project = await openProject(root, { unsaved: Object.fromEntries(unsaved) });
    isOpen = true;
  };

  const lintText = async (file: string, text: string) => {
    if (outside.has(file)) return [];
    unsaved.set(file, text);
    if (!isOpen) await reopen();
    if (isRoot(file)) {
      project.setUnsaved(file, text);
    } else {
      await reopen();
      if (!isRoot(file)) {
        outside.add(file);
        unsaved.delete(file);
        return [];
      }
    }
    return lint(project, { componentFolders, scan, files: [file] });
  };

  write({ ready: true });
  for await (const raw of createInterface({ input: process.stdin })) {
    const line: Line = JSON.parse(raw);
    const { seq } = line;
    // One line failing, such as a save with a config typo, must not stop reporting on the rest.
    try {
      if ("save" in line) {
        const { diagnostics } = await save(root);
        write({ seq, files: Object.groupBy(diagnostics, ({ file }) => file) });
      } else if ("close" in line) {
        const file = line.close;
        unsaved.delete(file);
        outside.delete(file);
        if (!isOpen) await reopen();
        if (isRoot(file)) {
          // A root with no text left on disk would answer nothing if it were opened again.
          if (existsSync(path.join(project.root, file))) project.clearUnsaved(file);
          else await reopen();
        }
        write({ seq, file, diagnostics: [] });
      } else {
        write({ seq, file: line.file, diagnostics: await lintText(line.file, line.text) });
      }
    } catch (error) {
      write({ seq, error: error instanceof Error ? error.message : String(error) });
    }
  }
}

try {
  await serve(path.resolve(process.argv[2] ?? ""));
  process.exit(0);
} catch (error) {
  // The server warns once and keeps answering, the same whether this is the start or a later line.
  write({ ready: false, message: error instanceof Error ? error.message : String(error) });
  process.exit(1);
}
