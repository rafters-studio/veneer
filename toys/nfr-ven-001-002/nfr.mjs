// Each run is a fresh process; time counts from process start, so module load is inside the bound.
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const INDEX =
  "/Volumes/store/projects/rafters-studio/veneer/.claude/worktrees/agent-a587cd7142077d7ad/packages/veneer/dist/index.mjs";
const SITE =
  "/Volumes/store/tmp/claude-501/-Volumes-store-projects-rafters-studio-veneer/45c3ba21-7f79-4f87-9f0c-d82bf7328a8d/scratchpad/shingle-pin/sites/rafters.studio";
const PAGE = "src/pages/docs/index.astro";

const child = (call) => `
import { performance } from "node:perf_hooks";
import { check } from ${JSON.stringify(INDEX)};
const before = performance.now();
const result = await ${call};
const after = performance.now();
process.stdout.write(JSON.stringify({
  ok: result.ok, errors: result.errors, count: result.diagnostics?.length, notices: result.notices,
  docsPath: result.docsPath, sinceStart: after, inCall: after - before,
}));
`;

function once(call) {
  const started = performance.now();
  const { status, stdout, stderr } = spawnSync(
    process.execPath,
    ["--input-type=module", "--eval", child(call)],
    { encoding: "utf8", maxBuffer: 1 << 28 },
  );
  const wall = performance.now() - started;
  if (status !== 0) throw new Error(stderr);
  return { ...JSON.parse(stdout), wall };
}

const text = readFileSync(join(SITE, PAGE), "utf8");
const unsaved = JSON.stringify({ [PAGE]: text });

const preEdit = once(
  `check(${JSON.stringify(SITE)}, { paths: [${JSON.stringify(PAGE)}], unsaved: ${unsaved} })`,
);
console.log("NFR-VEN-001 first run", JSON.stringify(preEdit));

const runs = [];
for (let run = 0; run < 20; run += 1) runs.push(once(`check(${JSON.stringify(SITE)}, { save: true })`));
console.log("NFR-VEN-002 first", JSON.stringify(runs[0]));
console.log("NFR-VEN-002 sinceStart ms", runs.map((r) => Math.round(r.sinceStart)).join(" "));
console.log("NFR-VEN-002 inCall ms", runs.map((r) => Math.round(r.inCall)).join(" "));
console.log("NFR-VEN-002 wall ms", runs.map((r) => Math.round(r.wall)).join(" "));
console.log(
  "slowest sinceStart",
  Math.round(Math.max(...runs.map((r) => r.sinceStart))),
  "slowest wall",
  Math.round(Math.max(...runs.map((r) => r.wall))),
);
