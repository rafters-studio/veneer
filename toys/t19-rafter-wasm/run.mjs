// Run a t19 wasm once on a .rafter file: node run.mjs <wasm> <file.rafter> [sheet.css]
import { readFileSync } from 'node:fs';

const [wasmPath, rafterPath, cssPath] = process.argv.slice(2);
const { instance } = await WebAssembly.instantiate(readFileSync(wasmPath), {});
const x = instance.exports;
const enc = new TextEncoder();

function put(bytes) {
  const p = x.alloc(bytes.length);
  new Uint8Array(x.memory.buffer, p, bytes.length).set(bytes);
  return p;
}

const rafter = enc.encode(readFileSync(rafterPath, 'utf8'));
const rp = put(rafter);
let out;
if (cssPath) {
  const css = enc.encode(readFileSync(cssPath, 'utf8'));
  out = x.compile_rafter_css(rp, rafter.length, put(css), css.length);
} else {
  out = x.compile_rafter(rp, rafter.length);
}
const len = new DataView(x.memory.buffer).getUint32(out, true);
const json = new TextDecoder().decode(new Uint8Array(x.memory.buffer, out + 4, len));
x.dealloc(out, 4 + len);
const result = JSON.parse(json);
if (result.css !== undefined) {
  result.css = `${result.css.slice(0, 200)}... (${result.css.length} bytes)`;
}
console.log(JSON.stringify(result, null, 2));
