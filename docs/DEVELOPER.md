# Developer Guide

## Prerequisites

- Node 24 or later, with the pnpm version `packageManager` names
- Rust on the stable toolchain

## Building and testing

```bash
pnpm install
pnpm -r build
pnpm bdd
pnpm exec toolchain drift
pnpm --filter @rafters/veneer check
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Scenarios

Each requirement's scenarios run as a Gherkin feature file under Cucumber. Generate the file from the requirement, and never edit it by hand:

```bash
pnpm exec toolchain features FR-VEN-064 packages/veneer/tests/FR-VEN-064.feature
```

Steps live in `packages/veneer/tests/*.steps.ts`. A scenario with no `@criterion-` tag fails the run.

`FR-VEN-075-c2` runs Cucumber inside Cucumber, and that inner run loads every steps file. It skips tagged scenarios by setting `VENEER_UNTAGGED_PROBE`, so an `After` hook has to tolerate a skipped scenario, for example by not assuming a Project was opened.

## Release

A pushed `v*` tag publishes `@rafters/veneer` to npm with the tag's version, using the `NPM_TOKEN` secret. See #128 before the first publish.

## CI

`.github/workflows/ci.yml` runs on push to main and every pull request: the Rust check, format, lint and tests, and a `node` job on macOS with Node 24 that runs the install, build, drift and scenarios.

## Style

- No emoji in code, comments or docs.
- No `any` in TypeScript; narrow from `unknown`.
- pnpm only.
- No `unsafe` in Rust.
