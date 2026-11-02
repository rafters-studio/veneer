// Checks the measured runs did real work: root files, docs lines, and that a page's unsaved error shows.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { check, openProject } from "/Volumes/store/projects/rafters-studio/veneer/.claude/worktrees/agent-a587cd7142077d7ad/packages/veneer/dist/index.mjs";

const SITE =
  "/Volumes/store/tmp/claude-501/-Volumes-store-projects-rafters-studio-veneer/45c3ba21-7f79-4f87-9f0c-d82bf7328a8d/scratchpad/shingle-pin/sites/rafters.studio";
const PAGE = "src/pages/docs/index.astro";

const project = await openProject(SITE);
const roots = project.checker.getRootFileNames();
console.log("roots", roots.length, "astro/tsx", roots.filter((f) => f.endsWith(".tsx") || f.endsWith(".astro")).length);
project.close();

const lines = readFileSync(join(SITE, ".rafters/veneer/docs.jsonl"), "utf8").trimEnd().split("\n");
console.log("docs lines", lines.length, lines[0], lines[1].slice(0, 120));

const result = await check(SITE, {
  paths: [PAGE],
  unsaved: { [PAGE]: `<div class="p-[1px]">a</div>` },
});
console.log(result.ok, result.report);
