# T15: the built server in real VS Code, and what its copy costs

`PREDICTIONS.md` was written before it ran. A small extension starts the built `veneer-lsp` with `--worker`, on files selected by pattern. `@vscode/test-electron` launches VS Code 1.100.0 on the lint fixture, and the suite reads `vscode.languages.getDiagnostics`.

```sh
pnpm install --ignore-workspace
T15_USER_DATA=<a short path> node test/run.cjs <veneer-lsp> <dist/worker.mjs> <project> <out.json> [--astro]
```

`T15_USER_DATA` must be short, or VS Code's socket path is too long and the run fails to connect.

| | Measured | Predicted |
|---|---|---|
| Diagnostic with source `veneer` after opening a new page | 751 ms alone, 715 ms with Astro's extension | within 15 s |
| Another source's diagnostic on the same file | yes, `ts` 2322 at 4:9 | yes |
| Shows without Astro's extension | yes; the file's language is `plaintext` | yes |
| Removing the misuse clears it | 352 ms alone, 335 ms with Astro's | within 2 s |
| `@rafters/veneer` installed clean with production dependencies | 69 MB | under 100 MB |
| The client library | under 1 MB | under 5 MB |

All six held.

## What it showed that nobody predicted

**The same error shows twice.** With Astro's extension installed, a type error on a rafters component appears once from `ts` and once from `veneer`, at the same place with the same message. T4 counted two servers side by side as success. In use, this is a duplicate for every undeclared prop. Veneer's own additions are the help text and `rafters(arbitrary-value)`, which Astro does not report.

**69 MB, of which TypeScript is 24 MB.** The next largest are prettier at 9.6 MB, yaml-language-server at 6.9 MB and vscode-css-languageservice at 4.6 MB. They come with `@astrojs/language-server`, and veneer's read uses none of them. `pnpm deploy --prod --legacy` reported 137 MB because it stores copies twice, so do not use that number. Astro's own extension is 17 MB on disk, bundled and without TypeScript, which shows what bundling can reach. Bundling veneer's worker was not tried.

**VS Code already has a Node.** The extension host can run the worker, and the extension could be the language server itself. Then the Rust binary and a `node` on `PATH` would not be needed for VS Code. Zed needs a binary or its own Node handling. Not tried.

## Limits

- One fixture, one machine, one VS Code version.
- `--disable-workspace-trust` is on, so how the extension behaves in an untrusted folder was not tested. VS Code's trust prompt is separate from FR-VEN-064's rule that opening a project runs none of its code.
- The server and worker paths come from environment variables. Finding them inside a packaged extension is the part left to design.
