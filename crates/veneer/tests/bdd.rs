//! Runs every `.feature` file under `tests/` as acceptance scenarios
//! (rstest-bdd). Feature files are generated from legion requirements and
//! mirror `src/`; step code lives in `tests/steps/`, compiled into this one
//! binary so every scenario can see every step.

rstest_bdd_macros::scenarios!("tests");
