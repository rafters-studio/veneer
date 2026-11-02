// node client.mjs <server binary> <dist/index.mjs> <project> <component folder> <page>: drives the server as an editor would.
import { spawn } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";
const [bin, dist, root, folder, page] = process.argv.slice(2);
const started = performance.now();
const server = spawn(bin, [dist, root, folder, join(import.meta.dirname, "worker.mjs")], { stdio: ["pipe", "pipe", "inherit"] });
let buffer = Buffer.alloc(0);
const waiting = [];
const inbox = [];
server.stdout.on("data", (chunk) => {
  buffer = Buffer.concat([buffer, chunk]);
  for (;;) {
    const end = buffer.indexOf("\r\n\r\n");
    if (end < 0) return;
    const length = +/Content-Length: (\d+)/i.exec(buffer.subarray(0, end).toString())[1];
    if (buffer.length < end + 4 + length) return;
    const message = JSON.parse(buffer.subarray(end + 4, end + 4 + length).toString());
    buffer = buffer.subarray(end + 4 + length);
    if (message.method === "textDocument/publishDiagnostics") { const w = waiting.shift(); if (w) w(message.params); else inbox.push(message.params); }
    else if (message.id !== undefined) { const w = pendingIds.get(message.id); if (w) w(message); }
  }
});
const pendingIds = new Map();
const send = (m) => { const s = JSON.stringify({ jsonrpc: "2.0", ...m }); server.stdin.write(`Content-Length: ${Buffer.byteLength(s)}\r\n\r\n${s}`); };
const request = (id, method, params) => new Promise((r) => { pendingIds.set(id, r); send({ id, method, params }); });
const next = () => inbox.length ? Promise.resolve(inbox.shift()) : new Promise((r) => waiting.push(r));
const uriOf = (f) => `file://${join(root, f)}`;
const median = (x) => [...x].sort((a, b) => a - b)[Math.floor(x.length / 2)];
const short = (p) => ({ n: p.diagnostics.length, codes: p.diagnostics.map((d) => `${d.range.start.line + 1}:${d.range.start.character + 1} ${d.code}`), ...p._t14 });
await request(1, "initialize", { processId: null, rootUri: `file://${root}`, capabilities: {} });
send({ method: "initialized", params: {} });
const base = readFileSync(join(root, page), "utf8");
const planted = (i) => base.replace("\n---", `\nimport Button from '@/components/ui/button.astro';\n---`).replace("<H1 size=", `<Button tone="x${i}">a</Button>\n      <H1 size=`);
const out = {};
let v = 1;
send({ method: "textDocument/didOpen", params: { textDocument: { uri: uriOf(page), languageId: "astro", version: v, text: planted(0) } } });
const first = await next();
out.firstPublishMs = Math.round(performance.now() - started);
out.first = short(first);
const change = (file, text) => send({ method: "textDocument/didChange", params: { textDocument: { uri: uriOf(file), version: ++v }, contentChanges: [{ text }] } });
const totals = [], worker = [], rust = [];
for (let i = 1; i <= 20; i++) {
  const t = performance.now();
  change(page, planted(i));
  const p = await next();
  totals.push(performance.now() - t); worker.push(p._t14.workerMs); rust.push(p._t14.serverMs - p._t14.workerMs);
}
out.warm = { medianMs: +median(totals).toFixed(1), maxMs: +Math.max(...totals).toFixed(1), workerMedianMs: median(worker), rustLayerMedianMs: +median(rust).toFixed(2) };
// A burst: only the newest text's result may arrive last, and none after it.
for (let i = 100; i < 110; i++) change(page, planted(i));
const burst = [];
const settle = () => new Promise((r) => setTimeout(r, 1200));
const collector = (async () => { for (;;) burst.push(await next()); })();
await settle();
out.burst = { published: burst.length, lastSeq: burst.at(-1)?._t14?.seq, sentLastSeq: v - 1 + 0, n: burst.at(-1)?.diagnostics.length };
// A page not in the program.
const fresh = "src/pages/t14-brand-new.astro";
const before = burst.length;
const t1 = performance.now();
send({ method: "textDocument/didOpen", params: { textDocument: { uri: uriOf(fresh), languageId: "astro", version: ++v, text: `---\nimport Button from '@/components/ui/button.astro';\n---\n<Button tone="x">a</Button>\n` } } });
while (burst.length === before) await new Promise((r) => setTimeout(r, 5));
out.newPage = { ms: Math.round(performance.now() - t1), ...short(burst.at(-1)) };
const t2 = performance.now();
const b2 = burst.length;
change(fresh, `---\nimport Button from '@/components/ui/button.astro';\n---\n<Button variant="nope">a</Button>\n`);
while (burst.length === b2) await new Promise((r) => setTimeout(r, 2));
out.newPageNextChange = { ms: Math.round(performance.now() - t2), ...short(burst.at(-1)) };
console.log(JSON.stringify(out, null, 1));
const pid = server.pid;
server.kill("SIGKILL");
await new Promise((r) => setTimeout(r, 800));
process.stdout.write(`server killed, pid ${pid}\n`);
process.exit(0);
