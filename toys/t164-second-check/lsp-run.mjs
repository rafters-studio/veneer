// node lsp-run.mjs <veneer-lsp binary> <installed copy of the read fixture>: W2 and W3b end to end through the real server.
import { spawn } from "node:child_process";
import { cpSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = fileURLToPath(new URL(".", import.meta.url));
const pkg = path.join(here, "../../packages/veneer");
const fixture = path.join(pkg, "test/fixtures/read");
const [binary, R] = process.argv.slice(2);
const CLEAN = `---\n---\n<div class="p-4"></div>\n`;
const BRACKET = `---\n---\n\n<div class="p-[3px]"></div>\n`;
const USES = `---\nimport { box } from "../box.ts";\n---\n<div class={box}></div>\n`;

function restore() {
  for (const item of ["src", "tsconfig.json"]) {
    rmSync(path.join(R, item), { recursive: true, force: true });
    cpSync(path.join(fixture, item), path.join(R, item), { recursive: true });
  }
  rmSync(path.join(R, ".rafters"), { recursive: true, force: true });
  mkdirSync(path.join(R, ".rafters"));
  writeFileSync(
    path.join(R, ".rafters/config.rafters.json"),
    JSON.stringify({ framework: "astro", componentsPath: "src/components/ui" }),
  );
}

async function session(name, run) {
  restore();
  const server = spawn(binary, ["--worker", path.join(pkg, "dist/worker.mjs")], {
    stdio: ["pipe", "pipe", "inherit"],
  });
  const publishes = [];
  let answered = false;
  let buffer = Buffer.alloc(0);
  server.stdout.on("data", (chunk) => {
    buffer = Buffer.concat([buffer, chunk]);
    for (;;) {
      const end = buffer.indexOf("\r\n\r\n");
      if (end < 0) return;
      const length = Number(/Content-Length: (\d+)/i.exec(buffer.toString("utf8", 0, end))?.[1]);
      if (buffer.length < end + 4 + length) return;
      const message = JSON.parse(buffer.toString("utf8", end + 4, end + 4 + length));
      buffer = buffer.subarray(end + 4 + length);
      if (message.id === 1) answered = true;
      if (message.method === "textDocument/publishDiagnostics") publishes.push(message.params);
    }
  });
  const send = (message) => {
    const body = JSON.stringify({ jsonrpc: "2.0", ...message });
    server.stdin.write(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
  };
  const until = async (test, what) => {
    const started = Date.now();
    while (!test()) {
      if (Date.now() - started > 30000) throw new Error(`timed out: ${what}`);
      await new Promise((done) => setTimeout(done, 20));
    }
  };
  const uri = (file) => `file://${path.join(R, file)}`;
  send({
    id: 1,
    method: "initialize",
    params: { processId: null, rootUri: `file://${R}`, capabilities: {} },
  });
  await until(() => answered, "initialize");
  send({ method: "initialized", params: {} });
  const open = async (file, text) => {
    const seen = publishes.filter((p) => p.uri === uri(file)).length;
    send({
      method: "textDocument/didOpen",
      params: { textDocument: { uri: uri(file), languageId: "astro", version: 1, text } },
    });
    await until(() => publishes.filter((p) => p.uri === uri(file)).length > seen, `publish ${file}`);
    const found = publishes.filter((p) => p.uri === uri(file)).at(-1).diagnostics;
    console.log(`  ${name}: open ${file}: [${found.map((d) => d.message).join("; ")}]`);
    return found;
  };
  const close = async (file) => {
    const seen = publishes.filter((p) => p.uri === uri(file)).length;
    send({ method: "textDocument/didClose", params: { textDocument: { uri: uri(file) } } });
    await until(() => publishes.filter((p) => p.uri === uri(file)).length > seen, `close ${file}`);
  };
  await run({ open, close });
  server.kill();
}

await session("W2 through veneer-lsp", async ({ open, close }) => {
  await open("src/pages/n.astro", CLEAN);
  await close("src/pages/n.astro");
  await open("src/pages/n.astro", BRACKET);
});
await session("W2 fresh server", async ({ open }) => {
  await open("src/pages/n.astro", BRACKET);
});
await session("W3b through veneer-lsp", async ({ open }) => {
  writeFileSync(path.join(R, "src/box.ts"), `export const box = "p-4";\n`);
  writeFileSync(path.join(R, "src/pages/uses.astro"), USES);
  await open("src/pages/uses.astro", USES);
  writeFileSync(path.join(R, "src/box.ts"), `export const box = "p-[3px]";\n`);
  await open("src/pages/n.astro", CLEAN);
  await open("src/pages/uses.astro", `${USES}\n`);
});
restore();
process.exit(0);
