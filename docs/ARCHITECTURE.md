# Architecture

Veneer reads a rafters project and turns it into that project's docs. The same reading lets it check every use of rafters and compile composites into the project's own framework. It is being rebuilt, and nothing here is fixed before 1.0.

## The rebuild

The intent is legion document `01a058f3`. The requirements are `FR-VEN-061` to `FR-VEN-075`, read with `legion document view <id>`. They are built in this order:

1. The package, its scenario runner, CI and release (`FR-VEN-075`).
2. Read: one TypeScript program across `.astro` and `.tsx`, through astro check's own checker (`FR-VEN-064`).
3. `writeDocs`: settings, resolving components and writing `docs.jsonl` (`FR-VEN-061`, `FR-VEN-065`, `FR-VEN-074`).
4. Lint, the composite, compile, the docs pages and the language server. Their requirements are still drafts.

## What exists

- `packages/veneer`: `@rafters/veneer`, an ES module compiled by `vp pack`. It holds no binary. Its entry exports nothing yet.
- `packages/veneer/tests`: each requirement's scenarios, in a feature file generated from the requirement.
- `crates/veneer-check` and `crates/veneer-lsp`: scaffolds.
- `editors/`: scaffolds.

## Operations

| Command | What it does |
| --- | --- |
| `pnpm install` | Installs the workspace |
| `pnpm -r build` | Builds the package |
| `pnpm bdd` | Runs every scenario |
| `pnpm exec toolchain drift` | Checks dependencies against the rafters toolchain |
| `pnpm --filter @rafters/veneer check` | Lints and formats the package |
| `cargo test` | Runs the Rust tests |
| `cargo clippy --all-targets -- -D warnings` | Lints the Rust; warnings fail |
| `cargo fmt --all -- --check` | Checks Rust formatting |

CI (`.github/workflows/ci.yml`) runs these. A pushed `v*` tag publishes `@rafters/veneer` to npm (`.github/workflows/release.yml`).

## Rules that hold across the rebuild

- **Read, never infer.** When rafters publishes a fact as data (tokens, intelligence, DO and NEVER constraints, prop domains), read it. Never derive meaning from names or source shape.
- **Run the system's code, never reimplement it.** When veneer needs a project's logic, it calls that logic rather than copying it.
- **One check, one answer.** The same check sits behind `rafters check`, the language server and the agent hook.
- Conventions: no emoji in code, comments, commits or docs; pnpm only for JS and TS; no `any` in TypeScript; UUIDv7 for IDs. Branch first and never commit to `main`.

## Outside this repo

- **rafters** owns tokens, components, intelligence, the `rafters` command and its config. Veneer reads them as input.
- **shingle** hosts the sites that render veneer's output.

## The previous version

The Rust CLI that came before this rebuild was removed in #125. Git history holds it, and `../veneer-archive` is an older copy.
