# FR-VEN-066-c6: the T11 and T12 re-run

```sh
node c6-lint.mjs <packages/veneer/dist/index.mjs> <sites/rafters.studio>
```

shingle at a5ae5dee8e436c2006b2533a2afa99cab2ccc0c8, from the GitHub tarball, `pnpm install --frozen-lockfile`, `pnpm exec astro sync`, `sites/rafters.studio`. Unsaved text on `src/pages/kelex/index.astro` adds the Button import after the `typography-p.astro` line, `const planted: number = 'x';` as the last frontmatter line (line 10), and `<Button tone="x">` and `<Button variant="nope">` before the `<H1 size=` line (lines 24 and 25).

`project.diagnostics` across the program: `card.astro` 41:3 hint ts(6385), `container.astro` 48:3 hint ts(6385), `kelex/index.astro` 25:15 and 24:15 error ts(2322), 10:7 error ts(2322), 10:7 hint ts(6133).

`lint` with `componentFolders ["src/components/ui"]`: exactly `kelex/index.astro` 24:15 ts(2322) and 25:15 ts(2322), both with help `variant, size, loading`. The report ends `2 problems`.

Own time beyond `project.diagnostics`, 5 runs: 6.13, 3.51, 3.02, 3.00, 2.90 ms (limit 20 ms). The file on disk is unchanged.
