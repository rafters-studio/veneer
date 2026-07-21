//! FR-VEN-021: no code path in veneer spawns or shells any rafters or
//! tailwind tooling -- the consumer project owns its rafters lifecycle
//! entirely. Asserted structurally: production source across every veneer
//! crate contains no process-spawning call site outside the one allowlisted
//! seam.
//!
//! The allowlisted seam is veneer-docs' CLI-help parser, which runs the
//! PROJECT'S OWN binary with `--help` -- the opt-in `--binary` CLI-docs
//! feature. It invokes the documented binary itself, never rafters or
//! tailwind tooling, and never runs unless the operator passes `--binary`.

use std::fs;
use std::path::{Path, PathBuf};

const SPAWN_MARKERS: &[&str] = &["process::Command", "Command::new(", "std::process::"];

/// Files permitted to spawn, with the reason recorded here so the exemption
/// is a decision, not a leak.
const ALLOWLIST: &[&str] = &["veneer-docs/src/cli_parser.rs"];

fn crates_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .to_path_buf()
}

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("readdir") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn no_production_code_path_spawns_a_process() {
    let root = crates_root();
    let mut sources = Vec::new();
    for krate in ["veneer", "veneer-adapters", "veneer-docs"] {
        // src/ only: tests (including this one) legitimately run the veneer
        // binary itself; the contract binds production code.
        rust_sources(&root.join(krate).join("src"), &mut sources);
    }
    assert!(
        sources.len() > 10,
        "source scan found too few files -- wrong root?"
    );

    let mut violations = Vec::new();
    for path in sources {
        // Normalized to `/` so the allowlist matches on Windows.
        let rel = path
            .strip_prefix(&root)
            .expect("under crates root")
            .display()
            .to_string()
            .replace(std::path::MAIN_SEPARATOR, "/");
        if ALLOWLIST.contains(&rel.as_str()) {
            continue;
        }
        let text = fs::read_to_string(&path).expect("read source");
        for marker in SPAWN_MARKERS {
            if text.contains(marker) {
                violations.push(format!("{rel}: {marker}"));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "process spawn outside the allowlisted CLI-help seam (FR-VEN-021):\n{}",
        violations.join("\n")
    );
}
