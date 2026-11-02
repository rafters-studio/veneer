import assert from "node:assert/strict";
import { spawn, spawnSync, type ChildProcess } from "node:child_process";
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { setTimeout as sleep } from "node:timers/promises";
import { After, Given, Then, When } from "@cucumber/cucumber";
import {
  DOCS,
  INSTALL_TIMEOUT_MS,
  installOnce,
  pkg,
  type DocsWorld,
} from "./support/docs-fixture.ts";

const SERVER_MS = 60 * 1000;
const WAIT_MS = 30 * 1000;
const WORKER = join(pkg, "dist/worker.mjs");
const PLANTED_WORKER = "node_modules/@rafters/veneer/dist/worker.mjs";
const MARKER = "planted-worker-ran";

interface LspDiagnostic {
  range: { start: { line: number; character: number }; end: { line: number; character: number } };
  severity: number;
  code: string;
  source: string;
  message: string;
}

interface Publish {
  uri: string;
  diagnostics: LspDiagnostic[];
}

interface LspMessage {
  id?: number;
  method?: string;
  params?: unknown;
}

// FR-VEN-064's After removes the folders.
interface LspWorld extends DocsWorld {
  copy?: string;
  // FR-VEN-064's "does not exist on disk" reads this.
  fixture?: string;
  server?: ChildProcess;
  publishes?: Publish[];
  shown?: { type: number; message: string }[];
  answers?: Set<number>;
  version?: number;
  page?: string;
}

function send(world: LspWorld, message: object): void {
  assert.ok(world.server?.stdin, "the server has not started");
  const body = JSON.stringify({ jsonrpc: "2.0", ...message });
  world.server.stdin.write(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
}

async function until(what: string, holds: () => boolean, ms = WAIT_MS): Promise<void> {
  const deadline = performance.now() + ms;
  while (!holds()) {
    assert.ok(performance.now() < deadline, `timed out waiting until ${what}`);
    await sleep(10);
  }
}

function onMessage(world: LspWorld, message: LspMessage): void {
  if (message.method === "textDocument/publishDiagnostics") {
    (world.publishes ??= []).push(message.params as Publish);
  } else if (message.method === "window/showMessage") {
    (world.shown ??= []).push(message.params as { type: number; message: string });
  } else if (message.id !== undefined) {
    (world.answers ??= new Set()).add(message.id);
  }
}

// The Rust build's executable, wherever the target folder is.
let binary: string | undefined;
function buildOnce(): string {
  if (binary) return binary;
  const { status, stdout, stderr } = spawnSync(
    "cargo",
    ["build", "-p", "veneer-lsp", "--message-format=json"],
    { cwd: pkg, encoding: "utf8", timeout: INSTALL_TIMEOUT_MS, maxBuffer: 64 * 1024 * 1024 },
  );
  assert.equal(status, 0, stderr);
  const artifacts = stdout
    .trimEnd()
    .split("\n")
    .map((line): { executable?: string | null } => JSON.parse(line));
  binary = artifacts.findLast(({ executable }) => executable)?.executable ?? undefined;
  assert.ok(binary, "cargo built no executable");
  return binary;
}

function makeCopy(world: LspWorld): void {
  const copy = mkdtempSync(join(tmpdir(), "veneer-lsp-"));
  (world.folders ??= []).push(copy);
  cpSync(join(pkg, "test/fixtures/lint"), copy, { recursive: true });
  installOnce(world, copy);
  world.copy = copy;
  world.fixture = copy;
}

async function start(world: LspWorld, option: string[]): Promise<void> {
  if (!world.copy) makeCopy(world);
  const server = spawn(buildOnce(), option, { stdio: ["pipe", "pipe", "inherit"] });
  world.server = server;
  let buffer = Buffer.alloc(0);
  server.stdout.on("data", (chunk: Buffer) => {
    buffer = Buffer.concat([buffer, chunk]);
    for (;;) {
      const end = buffer.indexOf("\r\n\r\n");
      if (end < 0) return;
      const length = Number(/Content-Length: (\d+)/i.exec(buffer.toString("utf8", 0, end))?.[1]);
      if (buffer.length < end + 4 + length) return;
      onMessage(world, JSON.parse(buffer.toString("utf8", end + 4, end + 4 + length)));
      buffer = buffer.subarray(end + 4 + length);
    }
  });
  send(world, {
    id: 1,
    method: "initialize",
    params: { processId: null, rootUri: uriOf(world, ""), capabilities: {} },
  });
  await until("the server answers initialize", () => world.answers?.has(1) === true);
  send(world, { method: "initialized", params: {} });
}

function copyOf(world: LspWorld): string {
  assert.ok(world.copy, "the server has not started");
  return world.copy;
}

function uriOf(world: LspWorld, file: string): string {
  return `file://${join(copyOf(world), file)}`.replace(/\/$/, "");
}

function open(world: LspWorld, file: string, text: string): void {
  world.page = file;
  world.version = 1;
  send(world, {
    method: "textDocument/didOpen",
    params: { textDocument: { uri: uriOf(world, file), languageId: "astro", version: 1, text } },
  });
}

function change(world: LspWorld, text: string): void {
  assert.ok(world.page, "no file is open");
  world.version = (world.version ?? 1) + 1;
  send(world, {
    method: "textDocument/didChange",
    params: {
      textDocument: { uri: uriOf(world, world.page), version: world.version },
      contentChanges: [{ text }],
    },
  });
}

function publishesOf(world: LspWorld, file: string): Publish[] {
  return (world.publishes ?? []).filter(({ uri }) => uri === uriOf(world, file));
}

async function nextPublish(world: LspWorld, file: string, seen: number): Promise<Publish> {
  await until(`${file} is published`, () => publishesOf(world, file).length > seen);
  const publish = publishesOf(world, file).at(-1);
  assert.ok(publish);
  return publish;
}

// Feature text writes its quotes and line breaks escaped.
const unescaped = (text: string): string => text.replaceAll("\\n", "\n").replaceAll('\\"', '"');

function workerProcesses(world: LspWorld): string[] {
  const { stdout } = spawnSync("ps", ["-axww", "-o", "command="], { encoding: "utf8" });
  return stdout.split("\n").filter((line) => line.includes(WORKER) && line.includes(copyOf(world)));
}

After(async function (this: LspWorld) {
  const { server } = this;
  if (!server || server.exitCode !== null || server.signalCode !== null) return;
  const exited = new Promise((resolve) => server.once("exit", resolve));
  server.kill("SIGKILL");
  await exited;
});

Given(
  /^the lint fixture and veneer-lsp started on it with --worker naming packages\/veneer\/dist\/worker\.mjs$/,
  { timeout: INSTALL_TIMEOUT_MS + SERVER_MS },
  async function (this: LspWorld) {
    await start(this, ["--worker", WORKER]);
  },
);

Given(
  /^the lint fixture and veneer-lsp started on it$/,
  { timeout: INSTALL_TIMEOUT_MS + SERVER_MS },
  async function (this: LspWorld) {
    await start(this, ["--worker", WORKER]);
  },
);

Given(
  /^the lint fixture and veneer-lsp started on it, with (?:src\/pages\/brackets\.astro open and its|a) first publish received$/,
  { timeout: INSTALL_TIMEOUT_MS + SERVER_MS },
  async function (this: LspWorld) {
    await start(this, ["--worker", WORKER]);
    const file = "src/pages/brackets.astro";
    open(this, file, readFileSync(join(copyOf(this), file), "utf8"));
    await nextPublish(this, file, 0);
  },
);

Given(
  /^the lint fixture and veneer-lsp started on it with --worker naming a path that does not exist$/,
  { timeout: INSTALL_TIMEOUT_MS + SERVER_MS },
  async function (this: LspWorld) {
    await start(this, ["--worker", join(tmpdir(), "veneer-no-such-worker.mjs")]);
  },
);

Given(
  /^the lint fixture, with node_modules\/@rafters\/veneer\/dist\/worker\.mjs in the copy writing a marker file when run$/,
  { timeout: INSTALL_TIMEOUT_MS },
  function (this: LspWorld) {
    makeCopy(this);
    const planted = join(copyOf(this), PLANTED_WORKER);
    mkdirSync(dirname(planted), { recursive: true });
    writeFileSync(
      planted,
      `import { writeFileSync } from "node:fs";
writeFileSync(${JSON.stringify(join(copyOf(this), MARKER))}, "ran");
`,
    );
  },
);

Given(
  /^veneer-lsp started on it with no --worker$/,
  { timeout: INSTALL_TIMEOUT_MS + SERVER_MS },
  async function (this: LspWorld) {
    await start(this, []);
  },
);

When(
  /^the client opens (\S+) with the text on disk and sends didSave$/,
  function (this: LspWorld, file: string) {
    open(this, file, readFileSync(join(copyOf(this), file), "utf8"));
    send(this, {
      method: "textDocument/didSave",
      params: { textDocument: { uri: uriOf(this, file) } },
    });
  },
);

When(
  /^the client opens (\S+) with the text "(.*)"$/,
  function (this: LspWorld, file: string, text: string) {
    open(this, file, unescaped(text));
  },
);

When(/^the client opens (\S+)$/, function (this: LspWorld, file: string) {
  open(this, file, "");
});

When(
  /^the client sends 10 changes in a row, the last with the text "(.*)"$/,
  function (this: LspWorld, text: string) {
    for (let at = 1; at < 10; at += 1) change(this, `<div class="p-[${at}px] m-[${at}px]">a</div>`);
    change(this, unescaped(text));
  },
);

When(/^the server is killed with SIGKILL$/, async function (this: LspWorld) {
  assert.ok(this.server);
  assert.ok(workerProcesses(this).length > 0, "the worker is not running");
  const exited = new Promise((resolve) => this.server?.once("exit", resolve));
  this.server.kill("SIGKILL");
  await exited;
});

Then(
  /^the server publishes one diagnostic for it, with source "([^"]+)", code "([^"]+)", severity (\d+), range (\d+):(\d+) to (\d+):(\d+)$/,
  { timeout: SERVER_MS },
  async function (
    this: LspWorld,
    source: string,
    code: string,
    severity: string,
    startLine: string,
    startCharacter: string,
    endLine: string,
    endCharacter: string,
  ) {
    assert.ok(this.page);
    const { diagnostics } = await nextPublish(this, this.page, 0);
    assert.equal(diagnostics.length, 1, JSON.stringify(diagnostics));
    const [found] = diagnostics;
    assert.deepEqual(
      [found?.source, found?.code, found?.severity, found?.range],
      [
        source,
        code,
        Number(severity),
        {
          start: { line: Number(startLine), character: Number(startCharacter) },
          end: { line: Number(endLine), character: Number(endCharacter) },
        },
      ],
    );
  },
);

Then(
  /^when the client changes the text to "(.*)" the server publishes an empty list for it$/,
  { timeout: SERVER_MS },
  async function (this: LspWorld, text: string) {
    assert.ok(this.page);
    const seen = publishesOf(this, this.page).length;
    change(this, unescaped(text));
    const { diagnostics } = await nextPublish(this, this.page, seen);
    assert.deepEqual(diagnostics, []);
  },
);

Then(
  /^the server publishes for (\S+) and (\S+) as many diagnostics in total as lint\.expected\.jsonl has lines for them$/,
  { timeout: SERVER_MS },
  async function (this: LspWorld, first: string, second: string) {
    const expected = readFileSync(join(pkg, "test/fixtures/lint.expected.jsonl"), "utf8")
      .trimEnd()
      .split("\n")
      .map((line): { file: string } => JSON.parse(line))
      .filter(({ file }) => file === first || file === second).length;
    // The first file is published only by the save. A save publishes its files together.
    await until(`${first} is published`, () => publishesOf(this, first).length > 0, SERVER_MS / 2);
    await sleep(200);
    const lastCount = (file: string): number =>
      publishesOf(this, file).at(-1)?.diagnostics.length ?? 0;
    assert.equal(lastCount(first) + lastCount(second), expected);
    assert.equal(publishesOf(this, first).length, 1, "the save published more than once");
  },
);

Then(/^\.rafters\/veneer\/docs\.jsonl exists in the copy$/, function (this: LspWorld) {
  const docs = join(copyOf(this), DOCS);
  assert.ok(existsSync(docs), `${docs} does not exist`);
});

Then(
  /^the server publishes one diagnostic for it at (\d+):(\d+) whose message ends "(.*)"$/,
  { timeout: SERVER_MS },
  async function (this: LspWorld, line: string, character: string, ending: string) {
    assert.ok(this.page);
    const { diagnostics } = await nextPublish(this, this.page, 0);
    assert.equal(diagnostics.length, 1, JSON.stringify(diagnostics));
    const [found] = diagnostics;
    assert.deepEqual(found?.range.start, { line: Number(line), character: Number(character) });
    assert.ok(found?.message.endsWith(unescaped(ending)), found?.message);
  },
);

Then(
  /^the last publish for that file holds one diagnostic ending at character (\d+)$/,
  { timeout: SERVER_MS },
  async function (this: LspWorld, character: string) {
    const file = this.page;
    assert.ok(file);
    const newest = (): LspDiagnostic[] => publishesOf(this, file).at(-1)?.diagnostics ?? [];
    await until(
      `the newest text is published`,
      () => newest().length === 1 && newest()[0]?.range.end.character === Number(character),
    );
  },
);

Then(
  /^no publish arrives after it within 1 second$/,
  { timeout: SERVER_MS },
  async function (this: LspWorld) {
    const seen = this.publishes?.length;
    await sleep(1000);
    assert.equal(this.publishes?.length, seen);
  },
);

Then(
  /^within 2 seconds no process with the worker path in its command line remains for this copy$/,
  { timeout: SERVER_MS },
  async function (this: LspWorld) {
    await until("the worker ends", () => workerProcesses(this).length === 0, 2000);
  },
);

Then(
  /^the server sends one window\/showMessage of type (\d+)$/,
  { timeout: SERVER_MS },
  async function (this: LspWorld, type: string) {
    await until("the server warns", () => (this.shown?.length ?? 0) > 0);
    await sleep(500);
    assert.equal(this.shown?.length, 1, JSON.stringify(this.shown));
    assert.equal(this.shown?.[0]?.type, Number(type));
  },
);

Then(/^the marker file does not exist$/, function (this: LspWorld) {
  assert.ok(!existsSync(join(copyOf(this), MARKER)), "the folder's worker ran");
});

Then(/^it answers shutdown$/, { timeout: SERVER_MS }, async function (this: LspWorld) {
  send(this, { id: 2, method: "shutdown", params: null });
  await until("the server answers shutdown", () => this.answers?.has(2) === true);
  const exited = new Promise((resolve) => this.server?.once("exit", resolve));
  send(this, { method: "exit", params: null });
  await exited;
});
