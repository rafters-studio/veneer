# FR-VEN-065-c5: the T9 re-run

Runs `writeDocs` in fresh processes on each corpus: twice, then once with `tsconfig.json` replaced by a copy that keeps its `extends` and `compilerOptions`, sets `"include": []` and `"exclude": []`, and sets `files` to the root files in reverse code point order.

```sh
node measure.mjs <packages/veneer/dist/index.mjs> <project>
```

Apple M4, Node 26.13.0, at the #127 build. Each corpus is the tree of its commit, from GitHub's tarball for that commit, installed at the workspace root with `pnpm install --frozen-lockfile` and `pnpm_config_strict_dep_builds=false`.

| | shingle a5ae5dee, `sites/rafters.studio`, its own config | rafters 9c7b0d0a, `packages/ui`, `{"componentsPath":"src/components"}` |
|---|---|---|
| root files | 88 | 449 |
| .tsx components / own props | 0 / 0 | 291 / 856 |
| .tsx with values / default / doc | - | 96 / 180 / 320 |
| .astro components / own props | 42 / 160 | 121 / 606 |
| .astro with values / default | 32 / 23 | 82 / 171 |
| inheritedDefaults entries | 1 | 7 |
| sha256, run 1 | b64552caa5377fa5 | 19d71adcaf7c9d5b |
| sha256, run 2 | b64552caa5377fa5 | 19d71adcaf7c9d5b |
| sha256, reversed | b64552caa5377fa5 | 19d71adcaf7c9d5b |

All three outputs are byte-identical on each corpus, and every count the requirement expects holds.

Against T9's own output (`out4/*-run1/*/docs.source.jsonl` in the vault): rafters.studio is byte-equal. `packages/ui` differs on one line, Grid's `pattern`, whose `type` T9 wrote with a `never` entry; FR-VEN-065 drops it, which T9's `tool_limits` records.
