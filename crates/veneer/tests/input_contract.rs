//! FR-VEN-021 input-contract acceptance, proven against the real binary:
//! writes confined to declared outputs, and two projects yielding docs
//! traceable only to their own state. These run the shipped `veneer extract`,
//! not internal functions, because the contract is about what the tool does
//! to someone else's project.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../veneer-adapters/tests/fixtures/coverage/partial")
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &target)?;
        } else {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

/// Every file under `root`, relative path -> content bytes.
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("readdir") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .expect("under root")
                    .display()
                    .to_string();
                files.insert(rel, fs::read(&path).expect("read"));
            }
        }
    }
    files
}

fn run_extract(project: &Path) {
    let status = Command::new(env!("CARGO_BIN_EXE_veneer"))
        .args(["extract", "--project"])
        .arg(project)
        .status()
        .expect("veneer binary runs");
    assert!(status.success(), "extract exits zero");
}

// AC (FR-VEN-021): veneer performs no writes outside its declared output
// locations. The project's rafters state is read-only to veneer.
#[test]
fn extract_writes_only_inside_the_veneer_namespace() {
    let tmp = tempfile::tempdir().expect("tempdir");
    copy_dir_all(&fixture_root(), tmp.path()).expect("fixture copy");

    let before = snapshot(tmp.path());
    run_extract(tmp.path());
    let after = snapshot(tmp.path());

    // Nothing pre-existing was modified or deleted.
    for (rel, content) in &before {
        assert_eq!(
            after.get(rel).map(Vec::as_slice),
            Some(content.as_slice()),
            "pre-existing file modified or deleted: {rel}"
        );
    }
    // Everything created lives under .rafters/veneer/ (and no temp litter).
    for rel in after.keys().filter(|rel| !before.contains_key(*rel)) {
        assert!(
            rel.starts_with(".rafters/veneer/"),
            "write outside the declared output location: {rel}"
        );
        assert!(!rel.ends_with(".tmp"), "temp file left behind: {rel}");
    }
}

// AC (FR-VEN-021): two projects with different rafters state produce docs
// reflecting each project's own state, with no content traceable to anything
// but that project.
#[test]
fn two_projects_yield_docs_traceable_only_to_their_own_state() {
    let a = tempfile::tempdir().expect("tempdir a");
    let b = tempfile::tempdir().expect("tempdir b");
    copy_dir_all(&fixture_root(), a.path()).expect("fixture copy a");
    copy_dir_all(&fixture_root(), b.path()).expect("fixture copy b");

    // Project B declares one component project A does not have.
    let button = fs::read_to_string(b.path().join("components/button.tsx")).expect("button");
    fs::write(
        b.path().join("components/zenith.tsx"),
        button.replace("Button", "Zenith"),
    )
    .expect("write zenith");

    run_extract(a.path());
    run_extract(b.path());

    let docs_a = fs::read_to_string(a.path().join(".rafters/veneer/docs.jsonl")).expect("docs a");
    let docs_b = fs::read_to_string(b.path().join(".rafters/veneer/docs.jsonl")).expect("docs b");

    assert!(
        docs_b.contains("Zenith"),
        "B documents its own component set"
    );
    assert!(
        !docs_a.contains("Zenith"),
        "A carries nothing from B's state"
    );
    // Portability: neither substrate embeds the other project's absolute
    // root (paths are project-relative, so outputs carry no machine paths).
    assert!(!docs_a.contains(&b.path().display().to_string()));
    assert!(!docs_b.contains(&a.path().display().to_string()));
}
