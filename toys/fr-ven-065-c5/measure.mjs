// FR-VEN-065-c5: node measure.mjs <path to dist/index.mjs> <project>
// Runs writeDocs in fresh processes: twice, then once with the tsconfig's files reversed. Prints
// the counts, the outputs' hashes and whether they are byte-identical, and restores the tsconfig.
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";

const [dist, project] = process.argv.slice(2);
const DOCS = path.join(project, ".rafters/veneer/docs.jsonl");
const TSCONFIG = path.join(project, "tsconfig.json");

function compareCodePoints(a, b) {
  const [left, right] = [[...a], [...b]];
  for (let i = 0; i < Math.min(left.length, right.length); i += 1) {
    const difference = left[i].codePointAt(0) - right[i].codePointAt(0);
    if (difference !== 0) return difference;
  }
  return left.length - right.length;
}

function runInChild(script) {
  const { status, stdout, stderr } = spawnSync(
    process.execPath,
    ["--input-type=module", "--eval", script],
    { encoding: "utf8", maxBuffer: 2 ** 28 },
  );
  if (status !== 0) throw new Error(stderr);
  return stdout;
}

function writeDocsFresh() {
  const result = JSON.parse(
    runInChild(`import { writeDocs } from ${JSON.stringify(dist)};
process.stdout.write(JSON.stringify(await writeDocs(${JSON.stringify(project)})));`),
  );
  if (!result.ok) throw new Error(JSON.stringify(result.errors));
  return { bytes: readFileSync(DOCS), notices: result.notices };
}

// The first run's root files, each relative with / separators, from the program writeDocs reads.
function rootFiles() {
  return JSON.parse(
    runInChild(`import path from "node:path";
import { openProject } from ${JSON.stringify(dist)};
const project = await openProject(${JSON.stringify(project)});
const files = project.checker.getRootFileNames().map((file) => path.relative(project.root, file).split(path.sep).join("/"));
project.close();
process.stdout.write(JSON.stringify(files));`),
  );
}

function counts(bytes) {
  const lines = bytes.toString().trimEnd().split("\n").map(JSON.parse).slice(1);
  const out = {};
  for (const ext of [".tsx", ".astro"]) {
    const components = lines.filter((line) => line.file.endsWith(ext));
    const props = components.flatMap((line) => line.props);
    out[ext] = {
      components: components.length,
      ownProps: props.length,
      withValues: props.filter((prop) => prop.values).length,
      withDefault: props.filter((prop) => prop.default !== undefined).length,
      withDoc: props.filter((prop) => prop.doc !== undefined).length,
      inheritedDefaults: components.reduce(
        (total, line) => total + Object.keys(line.inheritedDefaults).length,
        0,
      ),
    };
  }
  return out;
}

const sha = (bytes) => createHash("sha256").update(bytes).digest("hex").slice(0, 16);

const first = writeDocsFresh();
const second = writeDocsFresh();

const original = readFileSync(TSCONFIG);
// tsconfig.json is JSONC; TypeScript parses it, as the program does.
const ts = (await import("node:module")).createRequire(dist)("typescript");
const { config } = ts.parseConfigFileTextToJson(TSCONFIG, original.toString());
const files = rootFiles().sort(compareCodePoints).reverse();
let reversed;
try {
  writeFileSync(
    TSCONFIG,
    JSON.stringify({
      ...(config.extends !== undefined && { extends: config.extends }),
      ...(config.compilerOptions !== undefined && { compilerOptions: config.compilerOptions }),
      include: [],
      exclude: [],
      files,
    }),
  );
  reversed = writeDocsFresh();
} finally {
  writeFileSync(TSCONFIG, original);
}

console.log(
  JSON.stringify(
    {
      project,
      rootFiles: files.length,
      counts: counts(first.bytes),
      notices: {
        first: first.notices.length,
        second: second.notices.length,
        reversed: reversed.notices.length,
      },
      sha256: { first: sha(first.bytes), second: sha(second.bytes), reversed: sha(reversed.bytes) },
      identical: first.bytes.equals(second.bytes) && first.bytes.equals(reversed.bytes),
    },
    null,
    2,
  ),
);
