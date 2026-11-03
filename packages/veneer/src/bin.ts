#!/usr/bin/env node
// rafters-check [root] [--stdin <path>]: the report on stdout, everything else on stderr, nothing
// written. Exit 0 for no error diagnostic, 1 for an error diagnostic, 2 when the check could not run.
import { existsSync, realpathSync } from "node:fs";
import path from "node:path";
import { text } from "node:stream/consumers";
import { parseArgs } from "node:util";
import { check } from "./check.ts";
import { relativeFile } from "./resolve.ts";

const COULD_NOT_RUN = 2;

function parsedArguments(): { root?: string; stdinPath?: string } | undefined {
  try {
    const { values, positionals } = parseArgs({
      allowPositionals: true,
      options: { stdin: { type: "string" } },
    });
    const [root, ...rest] = positionals;
    return rest.length > 0 ? undefined : { root, stdinPath: values.stdin };
  } catch {
    return undefined;
  }
}

// The current directory comes back with its links resolved, and a proposed file need not exist.
function resolvedLinks(file: string): string {
  if (existsSync(file)) return realpathSync(file);
  const parent = path.dirname(file);
  return parent === file ? file : path.join(resolvedLinks(parent), path.basename(file));
}

async function run(root: string, stdinPath: string | undefined): Promise<number> {
  const file =
    stdinPath && relativeFile(resolvedLinks(root), resolvedLinks(path.resolve(stdinPath)));
  const unsaved = file === undefined ? undefined : { [file]: await text(process.stdin) };
  const result = await check(root, { unsaved });
  if (!result.ok) {
    for (const { message } of result.errors) process.stderr.write(`${message}\n`);
    return COULD_NOT_RUN;
  }
  process.stdout.write(result.report);
  for (const { message } of result.notices) process.stderr.write(`${message}\n`);
  return result.diagnostics.some(({ severity }) => severity === "error") ? 1 : 0;
}

const parsed = parsedArguments();
if (parsed) {
  try {
    process.exitCode = await run(parsed.root ?? process.cwd(), parsed.stdinPath);
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = COULD_NOT_RUN;
  }
} else {
  process.stderr.write("usage: rafters-check [root] [--stdin <path>]\n");
  process.exitCode = COULD_NOT_RUN;
}
