// node measure.mjs <label> <runs> <cwd> <stdin file or -> <command> [args...]
// One warm-up run, then the median, min and max wall clock of <runs> cold processes.
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";

const [label, runs, cwd, stdinFile, command, ...args] = process.argv.slice(2);
const input = stdinFile === "-" ? undefined : readFileSync(stdinFile);

const once = () => {
  const started = process.hrtime.bigint();
  const result = spawnSync(command, args, { cwd, input, encoding: "utf8" });
  if (result.status !== 0 && result.status !== 1) throw new Error(`${label}: exit ${result.status}: ${result.stderr}`);
  return Number(process.hrtime.bigint() - started) / 1e6;
};

const first = once();
const times = Array.from({ length: Number(runs) }, once).sort((a, b) => a - b);
const median = times[Math.floor(times.length / 2)];
const fmt = (ms) => (ms < 100 ? ms.toFixed(1) : ms.toFixed(0));
console.log(
  `${label}: median ${fmt(median)} ms, min ${fmt(times[0])}, max ${fmt(times.at(-1))}, first ${fmt(first)} (n=${runs})`,
);
