import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { cpSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import type { DocsResult } from "@rafters/veneer";

export const pkg = fileURLToPath(new URL("../../", import.meta.url));
export const GOLDEN = join(pkg, "test/fixtures/docs.expected.jsonl");
export const INSTALL_TIMEOUT_MS = 5 * 60 * 1000;
export const DOCS = ".rafters/veneer/docs.jsonl";
export const CONFIG = ".rafters/config.rafters.json";

export interface DocsLine {
  kind: string;
  intent?: string | null;
  name?: string;
  file?: string;
  tags?: unknown[];
  props?: { name: string; type: string[]; values?: unknown[]; default?: string }[];
  inherited?: unknown[];
  inheritedDefaults?: unknown;
}

export interface DocsWorld {
  // FR-VEN-064's After removes each of these folders.
  folders?: string[];
  docsCopy?: string;
  secondCopy?: string;
  installed?: Set<string>;
  result?: DocsResult;
  renames?: [string, string][];
  restoreRename?: () => void;
  earlier?: Buffer;
  outside?: string;
  children?: { status: number | null; exitedAfterMs: number; sawResolved: boolean }[];
}

export function copyDocsFixture(world: DocsWorld): string {
  const folder = mkdtempSync(join(tmpdir(), "veneer-docs-"));
  (world.folders ??= []).push(folder);
  cpSync(join(pkg, "test/fixtures/docs"), folder, { recursive: true });
  return folder;
}

export function docsCopy(world: DocsWorld): string {
  world.docsCopy ??= copyDocsFixture(world);
  return world.docsCopy;
}

export function write(folder: string, file: string, text: string): void {
  mkdirSync(dirname(join(folder, file)), { recursive: true });
  writeFileSync(join(folder, file), text);
}

// Scenarios install after making their changes to the copy.
export function installOnce(world: DocsWorld, folder: string): void {
  world.installed ??= new Set();
  if (world.installed.has(folder)) return;
  world.installed.add(folder);
  const { status, stderr, stdout } = spawnSync("pnpm", ["install", "--frozen-lockfile"], {
    cwd: folder,
    encoding: "utf8",
    timeout: INSTALL_TIMEOUT_MS,
    // pnpm fails an install that skips build scripts (esbuild, sharp); the read runs neither.
    env: { ...process.env, pnpm_config_strict_dep_builds: "false" },
  });
  assert.equal(status, 0, `${stdout}\n${stderr}`);
}

export function docsLines(world: DocsWorld): DocsLine[] {
  return readFileSync(join(docsCopy(world), DOCS), "utf8")
    .trimEnd()
    .split("\n")
    .map((line): DocsLine => JSON.parse(line));
}

export function componentLines(world: DocsWorld): DocsLine[] {
  return docsLines(world).filter((line) => line.kind === "component");
}

export function componentNamed(world: DocsWorld, name: string): DocsLine {
  const found = componentLines(world).find((line) => line.name === name);
  assert.ok(found, `no component line named ${name}`);
  return found;
}

export function okResult(world: DocsWorld): Extract<DocsResult, { ok: true }> {
  assert.ok(world.result, "writeDocs has not run");
  assert.ok(world.result.ok, JSON.stringify(world.result));
  return world.result;
}

export function errorsOf(world: DocsWorld): Extract<DocsResult, { ok: false }>["errors"] {
  assert.ok(world.result, "writeDocs has not run");
  assert.ok(!world.result.ok, JSON.stringify(world.result));
  return world.result.errors;
}
