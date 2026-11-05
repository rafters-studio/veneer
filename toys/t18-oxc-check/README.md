# T18: the rafters check's own rules, in Rust on oxc

`PREDICTIONS.md` was written and committed (77d795b) before anything was built or timed.

## The design finding first

Rule one, the `ts(...)` diagnostics on a use of a rafters component (`ts(2322)`, `ts(2741)`,
`ts(6385)`), needs a type checker, and oxc has none. That is 2 of huttspawn/site's 3 findings and
13 of the 27 lines in `lint.expected.jsonl`. So a check on oxc alone answers for the rules ported
here, and not for rule one. Every other rule in `lint.ts` ports, matches 0.0.6 everywhere it was
compared, and runs about 130x faster on the whole project and 700x to 1000x faster on one file.

## What it is

A binary, `t18-oxc-check [root] [--stdin <path>] [--jsonl]`, with `rafters-check`'s report, exit
codes and outside-program notice. It ports, from `packages/veneer/src/lint.ts`:

- `rafters(arbitrary-value)`. It covers class attributes (`class`, `className`, `class:list`),
  `classy(...)` arguments anywhere, and const strings and const object properties (through
  `as` / `satisfies`) used in either, in the same file or imported from the project's own source.
  It also covers every literal in an own `.classes.ts` file, and the modifier exception: the
  utility follows the last colon outside brackets and parentheses.
- `rafters(retired-fill)` and `rafters(invalid-fill)`, with the replacement help.
- `rafters(unknown-surface)`, read from `.rafters/tokens/*.rafters.json`, namespace `fill`.
- `rafters(freehand-div)`.
- The exemptions: files under `componentsPath`, and a qualified component's own file (sibling
  `.behavior.ts` and `.classes.ts`, and one `/** */` block holding the six intelligence tags).

Parsing and scopes:

- `.astro`: the withastro/oxc fork at rev `e5e100abf1afb9a79ae22f97de32befe4b925d5d`, the rev
  withastro/compiler-rs pins. The toy calls `Parser::parse_astro` (crate feature `astro`), then
  `SemanticBuilderAstroExt::build_astro` (oxc_semantic feature `astro`, on by default in the fork).
  The fork is git only; it is not on crates.io.
- `.tsx` and `.ts` parse with the same fork. One `oxc_ast_visit::Visit` walker serves both.
  `visit_astro_script` is skipped, because the language server keeps `<script>` out of the
  page's TSX.
- `class:list` comes out as a plain JSX identifier named `class:list`. Both that form and the
  namespaced form are matched.
- Import and export following: `oxc_resolver` 11 from crates.io, with the project's
  `tsconfig.json` for `paths`. It shares no AST with the fork, so the two coexist. Exports are
  followed by hand, up to 8 steps, through:
  - named and default imports
  - `export { a as b } from`
  - `export *`
  - `export default`
  - `const A = B` chains
  - `<NS.X>` on a namespace import
- Positions: oxc's spans in `.astro` are absolute byte offsets into the file, so no source map is
  needed. They convert to 1-based UTF-16 columns, as the language server counts.

Files: tsconfig `include` and `exclude`, expanded the way TypeScript does. A wildcard never
enters `node_modules` or a dot folder. Only `.tsx`, `.astro` and `.classes.ts` files are checked.

## How to run

```sh
cargo build --release
sh run.sh /Volumes/store/projects/huttspawn/site <clone with @rafters/veneer@0.0.6> <probe project> ../../packages/veneer/test/fixtures/lint
```

The measurements:

- `run.sh` runs every measurement. Results are in `out/`.
- `measure.mjs` does one warm-up run, then 10 cold processes, and reports the median.
- `fixture.mjs` diffs the check against `lint.expected.jsonl`.
- `probes.mjs` sends proposed texts on `--stdin` to both checks and compares their
  `rafters(...)` lines.

The three projects:

- **The clone** is the scratch copy of huttspawn/site with 0.0.6 installed. `diff -rq` on `src`,
  `.rafters` and `tsconfig.json` against the real site: no differences.
- **The probe project** is a copy of the clone with the clone's `node_modules` linked. It adds
  two files: `src/lib/probe-boxes.ts` (own-source consts) and
  `deps/node_modules/fakedep/index.ts` (a dependency), plus a tsconfig `paths` entry for
  `fakedep`.
- **The real site** was only read.

Machine: this Mac. Times are wall clock for a whole process. Both checks had a warm file cache.
The first-run column is the warm-up run.

## Results against predictions

### Time (`out/times.txt`)

| file, `--stdin` with its own text | Rust, site | Rust, clone | 0.0.6, clone |
|---|---|---|---|
| page `src/pages/index.astro` | 2.8 ms (first 15.7) | 2.3 ms | 2280 ms |
| `src/components/hs-header.classes.ts` | 2.2 ms (first 28.1) | 2.1 ms | 1667 ms |
| component `src/components/hs-panel.astro` | 2.3 ms (first 15.9) | 2.4 ms | 1704 ms |
| whole project (212 files checked, 215 parsed) | 32.2 ms (first 186) | 33.0 ms | 4195 ms |
| proposed `index.astro` text, not on disk | 2.3 ms | 2.6 ms | 1536 ms |

`hs-panel.astro` is a component the check does not exempt. It has the six tags but no
`.behavior.ts`, so the time is real work. Its `classy(c.root)` is followed into
`hs-panel.classes.ts`.

The comparison is not like for like. 0.0.6's time includes opening the TypeScript program and the
type checker for rule one. The Rust time includes reading the config and the tsconfig, resolving
imports and parsing what they reach, and none of rule one.

1. One file under 15 / 10 / 15 ms: **held** on the median (2.8, 2.2, 2.3 ms). The very first
   run, with the file cache cold, was 15.7, 28.1 and 15.9 ms: over for the page and the classes
   file.
2. Each 0.0.6 run over 2 s, and Rust at least 100x faster: **partly held**. The page took 2280 ms,
   but the classes file took 1667 ms and the component 1704 ms, both under 2 s. Rust, clone against clone, was
   990x (page), 790x (classes file) and 710x (component) faster.
3. Whole project under 150 ms for Rust and over 4 s for 0.0.6: **held** (32 ms; 4195 ms). The
   first run, with the file cache cold, was 186 ms.
4. A proposed text within 5 ms of item 1: **held** (2.3 ms against 2.8 ms).

### Parity, ported rules only

5. huttspawn/site: **held**. Both report exactly
   `src/components/hs-header.classes.ts:264:24 error rafters(arbitrary-value) text-[0.625rem] is an arbitrary value`.
   0.0.6 also reports two `ts(2322)` findings in `src/pages/lab/surfaces.astro` (rule one, not
   ported). The Rust check gives the same report on the real site and on the clone. See
   `out/hs-*.txt`.
6. The lint fixture: **held on substance; the count in the prediction was wrong**. The golden has
   14 `rafters(...)` lines, not 13: 12 arbitrary-value (4 in `brackets-tsx.tsx`, 8 in
   `brackets.astro`) and 2 freehand-div. The Rust check matches all 14 on file, line, column,
   endLine, endColumn, severity, code and message or help. Nothing is missing and nothing is
   extra (`out/fixture.txt`). Both freehand-div lines depend on resolving tags through
   `@/components/ui/...` paths, and they came out right.
7. `--stdin` probes: **held, 32 of 32 agree** (`out/probes.txt`). The probes cover:
   - unchanged page, classes file and component
   - a class attribute
   - `classy` in a class, and `classy` outside JSX
   - a const string, a const object, a const object `as const`, and a const object in `classy`
   - a `let` object, and an object returned by a function (both report nothing)
   - an own-source const and an own-source object, both reported at the use
   - a dependency const and object (nothing)
   - modifier brackets (nothing)
   - `class:list` with an object key
   - a multi-line attribute after a combining mark and an astral character
   - a combining mark and an astral character before the bracket class on the same line, in a
     page (10:29) and in a classes file (333:28): UTF-16 columns agree with 0.0.6's source map
   - a template literal with and without substitutions
   - a freehand div
   - retired bare and composed fills, invalid fills, good and empty fills
   - known, unknown and empty surfaces
   - a classes file with a new bracket
   - a component with a bracket
   - a path outside the program (the same notice)
   - a plain `.ts` path
   - a `.classes.ts` under `componentsPath`

   Every probe agreed on the report lines, the help lines and the exit code, with one exception.
   On the freehand-div probe, 0.0.6 exits 1 because of a `ts(2322)` on `<Button>` (rule one); the
   Rust check exits 0.

### Shape

8. Under 900 lines: **missed**. `src/main.rs` is 1370 lines after `cargo fmt`, 1254 of them
   neither blank nor comment. Most of the surplus is the import and export following and the
   tsconfig include expansion, which the prediction did not count.
9. Release binary under 10 MB: **held**, 2.0 MB (strip, lto).
10. No type checker needed for the ported rules: **held**. oxc_semantic plus oxc_resolver plus
    hand-written export following answered every case in the fixture, the site and the probes.

## Differences found

None on the ported rules. Every line both sides produced matched.

## What oxc covered, and what it did not

Covered:

- parsing `.astro`, `.tsx` and `.ts` into one AST
- scopes and references in `.astro`, across the frontmatter and the template
- absolute source positions in `.astro`
- module resolution with tsconfig `paths`

Not covered, and written by hand:

- following exports across files, which TypeScript's `getAliasedSymbol` does for free
- tsconfig `include` and `exclude` expansion: `oxc_resolver` reads `paths`, not the program's
  files
- the token read for surfaces
- type information, for rule one

## Not done, and what could still differ from TypeScript

These are not exercised by anything here:

- tsconfig `extends` supplying `include` or `exclude`.
- JSONC with trailing commas. Comments are stripped; trailing commas are not.
- `export =` and `declare module`.
- An `export default` of an expression that is not an identifier, which is treated as declared in
  that file.
- TypeScript counts alias resolution as one step where the toy counts each hop.
- TypeScript attaches JSDoc only to nodes. The toy accepts any `/** */` block in the file, or in
  the frontmatter for `.astro`.
- Parse errors. The fork's `panicked` drops a file, with a line on stderr; TypeScript would still
  check what it parsed.
- `veneer.scan` `system` and `all`, which are read but never exercised.
- Dropping `<script>` contents in `.astro` follows the language server's split, but no probe
  compares them.
- The token read is a direct JSON read of `*.rafters.json`, not rafters' registry with its
  plugins. It agreed on huttspawn's tokens.

## What would change the design

- **Rule one cannot come from oxc.** Either the Rust check keeps a TypeScript program for rule
  one, the way T14 used a Node worker, or rule one is restated as a comparison against the props
  each component's source declares (the 2026-10-28 reflection). That would be read from the AST,
  without a type checker. The ported rules no longer need TypeScript at all.
- **The fork is a git dependency.** A crate that depends on it cannot be published to crates.io
  as is. A napi package, the way @astrojs/compiler-rs ships, builds from git without trouble.
  The rev must move in step with compiler-rs.
- **Cost moves to the TypeScript side.** At 2 to 3 ms a file and 32 ms for the whole site, the
  pre-edit hook's `--stdin` path is free on the Rust side. Whatever stays on TypeScript for rule
  one is the whole cost: 1.5 to 2.3 s here.
