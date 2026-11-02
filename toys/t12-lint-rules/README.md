# T12: reading a project safely, and finding bracket values

`PREDICTIONS.md` was written before anything ran.

```sh
cd fixture && pnpm install --ignore-workspace --no-frozen-lockfile && cd ..
node trust.mjs <dist/index.mjs> "$PWD/fixture" trusted
node trust.mjs <dist/index.mjs> "$PWD/fixture" untrusted
T12_MARKER=$PWD/m.txt node brackets.mjs <dist/index.mjs> "$PWD/fixture"
```

## A. Project code runs on open, and one call stops it

The fixture ships a fake `@astrojs/svelte` whose `dist/editor.cjs` writes a marker file.

| | Project code ran | Rafters page | Page importing `.svelte` |
|---|---|---|---|
| AstroCheck as it comes | **yes** | ts(2322) at 5:9 | none |
| `setIsTrusted(false)` first | **no** | ts(2322) at 5:9 | ts(2306) at 2:17 |

- The loader is `importEditorIntegration` in the language server's `dist/importPackage.js`. It `require`s `dist/editor.cjs` from whatever package the project resolves.
- Untrusted, the only change is one ts(2306) on a page importing a `.svelte` file, and FR-VEN-066's rule leaves that out because it is not a use of a rafters component.
- Untrusted, the language server prints one `console.info` line saying it could not find the package. Left alone, it would land in `rafters check`'s report.

## B. The bracket rule

A class's utility is what follows the last colon outside brackets and parentheses. If that holds `[`, it is arbitrary. Class attributes are `class`, `className` and `class:list`. Files under `componentsPath` are skipped.

Fixture: 13 class literals read, 10 flagged, every one correct and at its exact source position, in `.astro` and `.tsx`: see `out-brackets-fixture.txt`. Not flagged, correctly: `data-[state=open]:p-4`, `group-data-[state=open]:p-4`, `p-4`, a bracket value in a file under `componentsPath`, and dynamic values.

rafters.studio: 206 class literals read, 0 flagged, 42 files skipped, 3.3 ms.

## Limits

Dynamic values such as `class={dynamic}` are invisible to this rule, by design. The rule looks at literals in class attributes only, so a value passed through a helper call is not seen. One real site was tried, and it has no bracket values to find, so true positives come only from the fixture.

## Predictions against results

All nine held. 1 and 2: code ran, then did not. 3: rafters page unchanged. 4: one new diagnostic. 5 and 6: all cases right at exact positions. 7: `class:list` literals reachable. 8: dynamic values gave nothing. 9: 3.3 ms and no false positives.
