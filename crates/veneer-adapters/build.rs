//! Rebuild the scenario harness when a `.feature` file is added under
//! `tests/`. `scenarios!` tracks the files it found, not the directory, so a
//! newly generated feature would otherwise not run until something else
//! changed.

fn main() {
    println!("cargo:rerun-if-changed=tests");
}
