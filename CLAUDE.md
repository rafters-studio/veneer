# Veneer — Claude Configuration

## Core Invariants

### Component Previews Are the Product
Framework-less component previews are veneer's entire reason to exist. Without them, veneer has no purpose. Always prioritize getting component previews rendering over any other work — JSDoc extraction, nav polish, styling improvements are all secondary to the preview pipeline working for all components.

### Just Do the Work
Don't ask "want me to start?" or "should I proceed?" — just do the work. Git can always rollback. When the plan is clear and issues are filed, start executing immediately. Only pause for genuine design ambiguity, not for permission to begin.

### Acceptance Scenarios
Acceptance criteria run as Gherkin scenarios through rstest-bdd. A `.feature` file lives in its crate's `tests/`, mirroring `src/`: behaviour whose entry point is `src/a/b.rs` has its scenarios in `tests/a/b.feature`. Feature files are generated from the legion requirement and never hand-edited. Step code lives in `tests/steps/` and is compiled into the crate's `tests/bdd.rs` harness, which runs every `.feature` under `tests/` with `cargo test`. A scenario with no matching step fails the run.
