# Architecture

Veneer turns a rafters project's components into documentation that any site can render: framework-less Web Component previews (Shadow DOM with adopted style sheets) and structured records describing each component. It is being rebuilt, and nothing here is fixed before 1.0.

## The rebuild

On 2026-10-09 every open issue was closed as not planned, because veneer is being retooled from a fresh intent: watch a project, map it, run checks on every save, and compile composites to Astro, React 19 and Web Components. That intent is not written yet. Until it lands, treat the code below as the previous build. It will be reused only where the new spec calls for it.

## What exists

A Rust workspace with two crates, on the stable toolchain:

- `crates/veneer`: the CLI, with two commands.
  - `veneer extract --project <path>` reads a rafters project and writes its documentation.
  - `veneer watch --project <path>` runs in the foreground and, after a debounced file change (`--debounce-ms`, default 300), rewrites the `.rafters/veneer/` jsonl files. It does not write pages or re-read `veneer.json`; only `extract` does. It has no server, no hot reload and no daemon.
- `crates/veneer-adapters`: the library the CLI calls. It reads component source with oxc (`react.rs`, `registry.rs`, `rafters_source.rs`), turns each component into a Web Component class (`generator.rs`), compiles its intelligence and constraints (`intelligence.rs`, `artifact.rs`), assesses coverage (`coverage.rs`), and generates page text (`mdx.rs`). `extract.rs` writes the files.

### What `extract` does

1. Names the mode from what the project root contains, never from a flag (`mode.rs`). A component matrix plus `.behavior.ts` files means the rafters monorepo (default mode); their absence means an installed consumer project (sidecar mode).
2. Reads the optional `veneer.json` at the project root (`veneer_config.rs`). A missing file means defaults; a malformed one is an error naming the file and field.
3. Discovers components and writes `docs.jsonl` and `index.jsonl` to `<project>/.rafters/veneer/`, from one assessment pass. Discovery picks the source reader from `componentTarget` in `.rafters/config.rafters.json`; a framework with no adapter is reported, not guessed at.
4. Writes one page per component to `<project>/<outputDir>/components/` (`outputDir` defaults to `docs`). A component that renders gets an MDX page plus a preview file. One that does not gets a page marked `status: not-yet-documented` with the reason, never a blank.

## Operations

| Command                                           | What it does                   |
| ------------------------------------------------- | ------------------------------ |
| `cargo check`                                     | Type-checks the workspace      |
| `cargo test`                                      | Runs the tests                 |
| `cargo fmt --all -- --check`                      | Format check                   |
| `cargo clippy --all-targets -- -D warnings`       | Lint; warnings fail            |
| `cargo run -p veneer -- extract --project <path>` | Runs extract against a project |

CI (`.github/workflows/ci.yml`) runs check, fmt, clippy, tests and `shellcheck install.sh`.

## Rules that hold across the rebuild

- **Read, never infer.** When rafters publishes a fact as data (tokens, intelligence, DO/NEVER constraints, prop domains), read it. Never derive meaning from names or source shape.
- **Run the system's code, never reimplement it.** When veneer needs a project's logic, it calls that logic rather than copying it.
- **Web Components are the preview contract.** A site renders a preview without React, Vue or Solid at runtime.
- Conventions: no emoji in code, comments, commits or docs; pnpm only for JS/TS; no `any` in TypeScript; UUIDv7 for IDs. Branch first and never commit to `main`.

## Outside this repo

- **rafters** owns tokens, components, intelligence and lint rules. Veneer reads them as input.
- **shingle** hosts the sites that render veneer's output (rafters.studio and others).

## The previous version

`../veneer-archive` is a copy of veneer as it stood before the rebuild. Read it to see how something worked before. Its `README.md` and `docs/` describe an earlier version still (the MDX-from-`--help` engine and a `veneer-docs` crate), so check the code itself.
