//! FR-VEN-012/026/030: watch mode (and every other veneer code path) opens
//! no server, no HMR channel, no network listener. Asserted structurally,
//! mirroring `veneer-adapters/tests/no_toolchain_spawn.rs`'s approach for
//! the toolchain-spawn contract: production source across every veneer
//! crate contains no network-listening call site.

use std::fs;
use std::path::{Path, PathBuf};

const NETWORK_MARKERS: &[&str] = &[
    "TcpListener",
    "UdpSocket",
    "tokio::net::",
    "::bind(",
    "hyper::Server",
    "axum::serve",
    "warp::serve",
];

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
fn no_production_code_path_opens_a_network_listener() {
    let root = crates_root();
    let mut sources = Vec::new();
    for krate in ["veneer", "veneer-adapters", "veneer-docs"] {
        // src/ only: this test file itself (and any future test helper)
        // is not bound by the production contract.
        rust_sources(&root.join(krate).join("src"), &mut sources);
    }
    assert!(
        sources.len() > 10,
        "source scan found too few files -- wrong root?"
    );

    let mut violations = Vec::new();
    for path in sources {
        // Normalized to `/` so this reads correctly if ever compared
        // against a hardcoded separator on Windows.
        let rel = path
            .strip_prefix(&root)
            .expect("under crates root")
            .display()
            .to_string()
            .replace(std::path::MAIN_SEPARATOR, "/");
        let text = fs::read_to_string(&path).expect("read source");
        for marker in NETWORK_MARKERS {
            if text.contains(marker) {
                violations.push(format!("{rel}: {marker}"));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "network listener outside veneer's contract (FR-VEN-012/026/030):\n{}",
        violations.join("\n")
    );
}
