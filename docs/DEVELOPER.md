# Developer Guide

Guide for contributors to veneer.

## Prerequisites

- Rust 1.75 or later
- Cargo (comes with Rust)
- Git

## Project Structure

```
veneer/
├── Cargo.toml              # Workspace manifest
├── Cargo.lock              # Dependency lock file
├── rust-toolchain.toml     # Rust version pinning
├── .cargo/
│   └── config.toml         # Cargo configuration
├── crates/
│   ├── veneer-adapters/   # JSX → Web Component, scope_css
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── react.rs     # React adapter
│   │       ├── registry.rs  # Component discovery
│   │       ├── scope.rs     # CSS scoping
│   │       └── ts_helpers.rs
│   ├── veneer-docs/       # CLI help parser, MDX skeletons, sidebar
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── cli_parser.rs
│   │       ├── reference.rs
│   │       ├── skeleton.rs
│   │       └── sidebar.rs
│   └── veneer/             # CLI
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           └── commands/
│               └── extract.rs
└── docs/
    ├── ARCHITECTURE.md
    └── DEVELOPER.md
```

## Building

```bash
cargo build                      # debug
cargo build --release            # release → target/release/veneer
cargo build -p veneer-adapters   # one crate
cargo check                      # no binary
```

## Testing

```bash
cargo test                                            # all
cargo test -p veneer-adapters                         # one crate
cargo test -p veneer-adapters scope_extracts_rules    # one test
cargo test -- --nocapture                             # with stdout
```

## Adding Features

### New framework adapter

1. Create `crates/veneer-adapters/src/<framework>.rs`
2. Implement the extraction functions (see `react.rs` as the reference)
3. Re-export from `lib.rs`
4. Add tests at the bottom of the file under `#[cfg(test)] mod tests`

### New CLI command

1. Create `crates/veneer/src/commands/<name>.rs` with a `run` function and an `Args` struct derived from `clap::Args`
2. Add the variant to the `Commands` enum in `main.rs` and dispatch in the `match`
3. Export from `commands/mod.rs`

## Code Style

Formatting and linting:

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
```

Guidelines:

- **Error handling** — `thiserror` for library errors, `anyhow` for the CLI, `.context("…")` when the call site adds meaning.
- **Docs** — `///` on public items.
- **Naming** — types `PascalCase`, functions `snake_case`, constants `SCREAMING_SNAKE_CASE`.
- **Imports** — std, external, internal, local — grouped and explicit. No globs.
- **No emoji** in code, comments, or docs (project policy).
- **No `unsafe`** (project policy).

## Debugging

```bash
RUST_LOG=debug cargo run -- extract --project ./some-cli --binary ./some-cli/target/debug/some-cli
RUST_LOG=veneer_adapters=debug,veneer_docs=info cargo run -- extract ...
```

Levels: `error`, `warn`, `info`, `debug`, `trace`.

VS Code launch config (CodeLLDB):

```json
{
  "type": "lldb",
  "request": "launch",
  "name": "Debug veneer extract",
  "program": "${workspaceFolder}/target/debug/veneer",
  "args": ["extract", "--project", "./some-cli", "--binary", "./some-cli/target/debug/some-cli"],
  "cwd": "${workspaceFolder}"
}
```

## Release Process

Tag-driven. GitHub Actions (`.github/workflows/release.yml`) builds binaries for x86_64-linux, x86_64-macos, aarch64-macos, and x86_64-windows on any `v*` tag push, uploads artifacts, and generates release notes.

```bash
# 1. Bump versions in root Cargo.toml [workspace.package] if needed
# 2. Commit & push
# 3. Tag
git tag v0.2.0
git push origin v0.2.0
```

## CI

`.github/workflows/ci.yml` runs on push to main and every PR:

- `cargo check`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `shellcheck install.sh`
- `cargo test` on ubuntu, macos, windows

All five jobs must pass before merge.
