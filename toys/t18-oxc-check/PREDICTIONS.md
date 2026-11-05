# T18 predictions

Written before anything was built or timed. The toy is a Rust binary on oxc that ports the rafters
check's own rules from `packages/veneer/src/lint.ts`: arbitrary-value brackets (class attributes,
`classy(...)` arguments, const strings and const object properties used in either, own `.classes.ts`
files, the modifier exception), retired and invalid `fill`, unknown `surface`, and `freehand-div`.
It does not port rule one (the `ts(...)` diagnostics on a use of a rafters component): that needs a
type checker, and oxc has none.

`.astro` parses with the withastro/oxc fork at `e5e100ab` (`Parser::parse_astro`, feature `astro`),
the rev withastro/compiler-rs pins; `.tsx` and `.ts` parse with the same fork, and one oxc
`Visit` walker serves both. Imports resolve with `oxc_resolver` (tsconfig `paths`). Scopes come
from `oxc_semantic` (`build_astro` for `.astro`).

The baseline is `rafters-check` from `@rafters/veneer@0.0.6`, installed in the scratch clone of
huttspawn/site. Times are wall clock for a cold process, median of 10 runs, on this machine.

## Time

1. One file, cold process, `--stdin` with the file's own text, release build:
   a page under 15 ms, a `.classes.ts` file under 10 ms, a component under 15 ms. Confidence 0.7.
2. The same three files through `rafters-check --stdin`: each over 2 s, because it opens the whole
   TypeScript program. The Rust check is at least 100x faster on each. Confidence 0.75.
3. The whole huttspawn/site project, cold: under 150 ms for the Rust check, over 4 s for
   `rafters-check`. Confidence 0.65.
4. `--stdin` with a proposed text that differs from disk costs the same as item 1, within 5 ms.
   Confidence 0.8.

## Parity, ported rules only (codes `rafters(...)`)

5. huttspawn/site: the Rust check reports exactly what 0.0.6 reports for the ported rules: one
   finding, `src/components/hs-header.classes.ts:264:24 rafters(arbitrary-value)`. Confidence 0.75.
6. The lint fixture: the Rust check reproduces all 13 `rafters(...)` lines of
   `lint.expected.jsonl` (11 arbitrary-value, 2 freehand-div) at the same file, line, column,
   endLine and endColumn, with nothing extra. Confidence 0.6. The likeliest miss: an `.astro`
   position or a `freehand-div` that depends on how a tag resolves through a barrel.
7. `--stdin` probes on huttspawn files (a bracket in a class attribute, in `classy(...)`, in a const
   string, in a const object property, a modifier bracket, a retired fill, an unknown surface): the
   Rust check and 0.0.6 agree on every probe. Confidence 0.6.

## Shape

8. The toy is under 900 lines of Rust. Confidence 0.6.
9. The release binary is under 10 MB. Confidence 0.7.
10. Nothing in the ported rules needs a TypeScript type checker: const and import resolution
    through oxc_semantic and oxc_resolver is enough for every case the fixture and huttspawn hold.
    Confidence 0.7.
