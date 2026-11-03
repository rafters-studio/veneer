#!/usr/bin/env node
// rafters-check [root]: the report on stdout, everything else on stderr, nothing written.
import { check } from "./check.ts";

const [root, ...rest] = process.argv.slice(2);

if (root?.startsWith("-") || rest.length > 0) {
  process.stderr.write("usage: rafters-check [root]\n");
  process.exitCode = 1;
} else {
  const result = await check(root ?? process.cwd());
  if (result.ok) {
    process.stdout.write(result.report);
    for (const { message } of result.notices) process.stderr.write(`${message}\n`);
    process.exitCode = result.diagnostics.some(({ severity }) => severity === "error") ? 1 : 0;
  } else {
    for (const { message } of result.errors) process.stderr.write(`${message}\n`);
    process.exitCode = 1;
  }
}
