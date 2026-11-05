// node probes.mjs <rafters-check> <t18-oxc-check> <probe project>
// Each probe sends a proposed text for a real huttspawn path on stdin to both checks, and compares
// their rafters(...) lines: the report line and its help line.
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";

const [tsCheck, rustCheck, root] = process.argv.slice(2);
const read = (file) => readFileSync(path.join(root, file), "utf8");

const page = "src/pages/index.astro";
const header = "src/components/hs-header.classes.ts";
const panel = "src/components/hs-panel.astro";
const front = (imports, body) =>
  `---\nimport Base from "@/layouts/Base.astro";\nimport Container from "@/components/ui/container.astro";\nimport Button from "@/components/ui/button.astro";\nimport classy from "@/lib/primitives/classy";\n${imports}\n---\n<Base>\n<Container>\n${body}\n</Container>\n</Base>\n`;

const probes = [
  ["unchanged page", page, read(page)],
  ["unchanged classes file", header, read(header)],
  ["unchanged component", panel, read(panel)],
  ["class attribute", page, front("", `<p class="p-[3px]">a</p>`)],
  ["classy in a class", page, front("const cond = true;", `<p class={classy("p-[3px]", cond && "m-[1px]")}>a</p>`)],
  ["classy outside JSX", page, front(`const box = classy("p-[3px]");`, `<p>a</p>`)],
  ["const string", page, front(`const box = "p-[3px]";`, `<p class={box}>a</p>`)],
  ["const object", page, front(`const box = { text: "p-[3px]" };`, `<p class={box.text}>a</p>`)],
  ["const object as const", page, front(`const box = { text: "p-[3px]" } as const;`, `<p class={box.text}>a</p>`)],
  ["const object in classy", page, front(`const box = { text: "text-[0.625rem]" };`, `<p class={classy(box.text)}>a</p>`)],
  ["let object", page, front(`let box = { text: "p-[3px]" };`, `<p class={box.text}>a</p>`)],
  ["function object", page, front(`function make() { return { text: "p-[3px]" }; }\nconst box = make();`, `<p class={box.text}>a</p>`)],
  ["own-source const", page, front(`import { boxText } from "@/lib/probe-boxes";`, `<p class={boxText}>a</p>`)],
  ["own-source object", page, front(`import { box } from "@/lib/probe-boxes";`, `<p class={box.text}>a</p>`)],
  ["dependency const", page, front(`import { depBox, depObj } from "fakedep";`, `<p class={depBox}>a</p>\n<p class={depObj.text}>b</p>`)],
  ["modifier bracket", page, front("", `<p class="data-[state=open]:p-2 aria-[current=page]:text-primary">a</p>`)],
  ["class:list", page, front("", `<p class:list={["p-[1px]", { "m-[2px]": true }]}>a</p>`)],
  ["multi-line, unicode before", page, front("", `<p title="é\u{1F600}" class="flex\n  w-[10px]">a</p>`)],
  ["template literal", page, front("const n = 1;", `<p class={\`h-[3px] \${n}\`}>a</p>\n<p class={\`w-[2px]\`}>b</p>`)],
  ["freehand div", page, front("", `<div class="flex"><Button>a</Button></div>`)],
  ["retired fill", page, front("", `<Container fill="background">a</Container>`)],
  ["composed fill", page, front("", `<Container fill="primary-to-accent/50">a</Container>`)],
  ["invalid fill", page, front("", `<Container fill="bg-fill-glass extra">a</Container>\n<Container fill="BG-FILL-X">b</Container>`)],
  ["good fill", page, front("", `<Container fill="bg-fill-brand">a</Container>\n<Container fill="">b</Container>`)],
  ["surfaces", page, front("", `<Container surface="card">a</Container>\n<Container surface="nosuch">b</Container>\n<Container surface="">c</Container>`)],
  ["classes file with a new bracket", header, `${read(header)}\nexport const probe = "p-[7px] aria-[current=page]:p-2";\n`],
  ["component with a bracket", panel, read(panel).replace("classy(c.root)", `classy(c.root, "p-[8px]")`)],
  ["file outside the program", "dist/x.astro", front("", `<p class="p-[3px]">a</p>`)],
  ["a plain .ts file", "src/lib/x.ts", `export const x = "p-[3px]";\n`],
  ["a classes file under componentsPath", "src/components/ui/x.classes.ts", `export const x = "p-[3px]";\n`],
];

function run(bin, file, text) {
  const started = process.hrtime.bigint();
  const result = spawnSync(bin, ["--stdin", file], { cwd: root, input: text, encoding: "utf8" });
  const ms = Number(process.hrtime.bigint() - started) / 1e6;
  const lines = [];
  const others = [];
  let keep = false;
  for (const line of result.stdout.split("\n")) {
    if (/^\S+:\d+:\d+ /.test(line)) {
      keep = line.includes(" rafters(");
      if (!keep) others.push(line.split(" ").slice(0, 3).join(" "));
    }
    else if (!line.startsWith("  help:")) keep = false;
    if (keep && line !== "") lines.push(line);
  }
  return { lines, others, ms, status: result.status, stderr: result.stderr.trim() };
}

let agree = 0;
for (const [name, file, text] of probes) {
  const ts = run(tsCheck, file, text);
  const rust = run(rustCheck, file, text);
  const same = ts.lines.join("\n") === rust.lines.join("\n");
  if (same) agree += 1;
  console.log(`## ${name} (${file}): ${same ? "same" : "DIFFERENT"}  ts ${ts.ms.toFixed(0)} ms exit ${ts.status}, rust ${rust.ms.toFixed(1)} ms exit ${rust.status}`);
  for (const line of ts.lines) console.log(`  ts   ${line}`);
  if (!same) for (const line of rust.lines) console.log(`  rust ${line}`);
  for (const line of ts.others) console.log(`  ts, not ported: ${line}`);
  if (ts.stderr) console.log(`  ts stderr: ${ts.stderr.split("\n")[0]}`);
  if (rust.stderr) console.log(`  rust stderr: ${rust.stderr.split("\n")[0]}`);
}
console.log(`${agree} of ${probes.length} probes agree`);
