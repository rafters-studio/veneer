# FR-VEN-064-c4: the T8 re-run

Measures `openProject` and `diagnostics()` on shingle `sites/rafters.studio` at a5ae5dee, installed with `pnpm install --frozen-lockfile`.

```sh
node measure.mjs <packages/veneer/dist/index.mjs> <site>
```

Apple M4, Node 26.13.0, at the #126 build:

| | fresh install | after `astro sync` |
|---|---|---|
| files | 88 | 89 |
| errors / warnings / hints | 0 / 0 / 2 | 0 / 0 / 2 |
| module load | 134-144 ms | 135-140 ms |
| cold (limit 2.10 s) | 1083-1123 ms | 1074-1114 ms |
| warm median per process (limit 100 ms) | 85-90 ms | 88-93 ms |

The hints in both are ts(6385) at `src/components/ui/card.astro` 41:3 and `src/components/ui/container.astro` 48:3. The planted page gives ts(2322) at 23:15-23:19 and 24:15-24:22 in both.

The 89th root is `.astro/types.d.ts`, which the site's tsconfig includes, `.gitignore` excludes and only `astro sync` generates. The requirement says 89 after install alone, which is #130.
