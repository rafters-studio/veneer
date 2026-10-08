//! Steps for `tests/mode.feature` (`src/mode.rs`, FR-VEN-033).

use rstest::fixture;
use rstest_bdd::Slot;
use rstest_bdd_macros::{given, then, when};
use tempfile::TempDir;
use veneer_adapters::{detect_mode, Mode};

#[derive(Default)]
pub struct ModeWorld {
    root: Slot<TempDir>,
    mode: Slot<Mode>,
}

#[fixture]
pub fn mode_world() -> ModeWorld {
    ModeWorld::default()
}

#[given("a project root with no component matrix and no behavior files")]
fn empty_project_root(mode_world: &ModeWorld) {
    mode_world
        .root
        .set(TempDir::new().expect("create temp project root"));
}

#[when("veneer detects the mode")]
fn detect(mode_world: &ModeWorld) {
    let mode = mode_world
        .root
        .with_ref(|root| detect_mode(root.path()))
        .expect("project root set by a Given step");
    mode_world.mode.set(mode);
}

#[then("the mode is Sidecar")]
fn mode_is_sidecar(mode_world: &ModeWorld) {
    assert_eq!(mode_world.mode.get(), Some(Mode::Sidecar));
}
