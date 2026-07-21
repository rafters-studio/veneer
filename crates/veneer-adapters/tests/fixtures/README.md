# Test fixtures: parser-level scope only (FR-VEN-021)

Every fixture tree under this directory is **parser-level**: it exercises
veneer's own reading, assessment, and emission logic against known input
shapes. None of these fixtures is integration truth about what rafters
actually generates — several predate the consumer-project contract and model
`.rafters` state by hand (the wave-3 habit FR-VEN-021 retired; the measured
failure was fixtures modeling `@utility` blocks real rafters never emits).

Integration truth is the rebuilt apps/demo consumer project, reached through
the env-gated tests (`VENEER_REAL_RAFTERS_ROOT`, see
`src/intelligence.rs` and `src/artifact.rs` `#[ignore]` tests). Those run
against a real rafters checkout and are the only tests that may claim
veneer works against what rafters ships.

Rules for this directory:

- Do not add new hand-modeled `.rafters` integration fixtures. A new fixture
  is legitimate only when it pins a *parser-level* behavior (a line shape, an
  error path, a determinism property).
- When a fixture's shape is contradicted by real rafters output, the fixture
  is wrong — fix it from a real checkout, never from memory.
