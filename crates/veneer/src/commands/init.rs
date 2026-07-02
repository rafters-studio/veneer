//! Additive init: wire veneer into a project by creating only veneer-owned files.
//!
//! `veneer init` never edits an authored file (`vite.config.*`, `package.json`,
//! source files). It creates `.veneer/config.toml` and prints the one line the
//! author adds to their own Vite config. Removing the `.veneer/` directory
//! uninstalls veneer and leaves the project byte-for-byte as it was.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;

/// Relative path of the veneer-owned config file inside the project root.
const CONFIG_RELATIVE_PATH: &str = ".veneer/config.toml";

/// Contents written to `.veneer/config.toml`. Kept static so repeated inits
/// are byte-identical and uninstall/reinstall is deterministic.
const CONFIG_CONTENTS: &str = "\
# Veneer configuration.
# This file is owned by veneer. Removing the .veneer/ directory uninstalls
# veneer and leaves every authored file byte-for-byte unchanged.

[veneer]
config_version = 1
";

#[derive(Args)]
pub struct InitArgs {
    /// Target project root (defaults to cwd)
    #[arg(long)]
    pub path: Option<PathBuf>,
}

/// The Vite config flavor found in the project root.
///
/// Variants are ordered to match Vite's own config resolution order
/// (`vite.config.js`, then `.mjs`, then `.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViteConfigFlavor {
    Js,
    Mjs,
    Ts,
}

impl ViteConfigFlavor {
    const ALL: [ViteConfigFlavor; 3] = [
        ViteConfigFlavor::Js,
        ViteConfigFlavor::Mjs,
        ViteConfigFlavor::Ts,
    ];

    fn file_name(self) -> &'static str {
        match self {
            ViteConfigFlavor::Js => "vite.config.js",
            ViteConfigFlavor::Mjs => "vite.config.mjs",
            ViteConfigFlavor::Ts => "vite.config.ts",
        }
    }
}

/// What `init` did to the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InitOutcome {
    /// `.veneer/config.toml` was created.
    Created,
    /// `.veneer/config.toml` already existed; nothing was written.
    AlreadyInitialized,
}

/// Detect (never modify) an existing Vite config in the project root.
fn detect_vite_config(root: &Path) -> Option<ViteConfigFlavor> {
    ViteConfigFlavor::ALL
        .into_iter()
        .find(|flavor| root.join(flavor.file_name()).is_file())
}

/// Create the veneer-owned files under `root`. Never touches authored files.
fn init_project(root: &Path) -> Result<InitOutcome> {
    if !root.is_dir() {
        anyhow::bail!("Project root does not exist: {}", root.display());
    }

    let config_path = root.join(CONFIG_RELATIVE_PATH);
    if config_path.is_file() {
        return Ok(InitOutcome::AlreadyInitialized);
    }

    let veneer_dir = root.join(".veneer");
    std::fs::create_dir_all(&veneer_dir)
        .with_context(|| format!("Failed to create {}", veneer_dir.display()))?;
    std::fs::write(&config_path, CONFIG_CONTENTS)
        .with_context(|| format!("Failed to write {}", config_path.display()))?;

    Ok(InitOutcome::Created)
}

/// The instruction printed for the author. Veneer never performs this edit.
fn integration_instruction(flavor: Option<ViteConfigFlavor>) -> String {
    match flavor {
        Some(flavor) => format!(
            "Detected {name} (veneer will never modify it).\n\
             To integrate, add this one line to the plugins array in {name}:\n\n    \
             veneer(),",
            name = flavor.file_name()
        ),
        None => "No Vite config detected (vite.config.ts / .js / .mjs).\n\
             When you add one, include this one line in its plugins array:\n\n    \
             veneer(),"
            .to_string(),
    }
}

pub async fn run(args: InitArgs) -> Result<()> {
    let root: PathBuf = match args.path {
        Some(path) => path,
        None => std::env::current_dir().context("Failed to resolve current directory")?,
    };

    let outcome = init_project(&root)?;
    match outcome {
        InitOutcome::Created => println!("Created {CONFIG_RELATIVE_PATH}"),
        InitOutcome::AlreadyInitialized => {
            println!("Already initialized ({CONFIG_RELATIVE_PATH} exists); nothing to do")
        }
    }

    println!("{}", integration_instruction(detect_vite_config(&root)));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// Snapshot every file under `root` as relative path -> exact bytes.
    fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in std::fs::read_dir(dir).expect("read_dir") {
                let path = entry.expect("dir entry").path();
                if path.is_dir() {
                    walk(root, &path, out);
                } else {
                    let rel = path.strip_prefix(root).expect("strip_prefix").to_path_buf();
                    out.insert(rel, std::fs::read(&path).expect("read file"));
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(root, root, &mut out);
        out
    }

    fn fixture_project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("vite.config.ts"),
            "export default { plugins: [] };\n",
        )
        .expect("write vite config");
        std::fs::write(dir.path().join("package.json"), "{\"name\":\"fixture\"}\n")
            .expect("write package.json");
        std::fs::create_dir(dir.path().join("src")).expect("mkdir src");
        std::fs::write(dir.path().join("src/main.ts"), "console.log(1);\n")
            .expect("write src file");
        dir
    }

    #[test]
    fn init_creates_only_veneer_owned_files() {
        let dir = fixture_project();
        let before = snapshot(dir.path());

        let outcome = init_project(dir.path()).expect("init");
        assert_eq!(outcome, InitOutcome::Created);

        let after = snapshot(dir.path());
        for (path, bytes) in &after {
            if path.starts_with(".veneer") {
                continue;
            }
            assert_eq!(
                before.get(path),
                Some(bytes),
                "authored file changed: {}",
                path.display()
            );
        }
        let added: Vec<&PathBuf> = after.keys().filter(|p| !before.contains_key(*p)).collect();
        assert_eq!(added, [&PathBuf::from(CONFIG_RELATIVE_PATH)]);
    }

    #[test]
    fn uninstall_restores_project_byte_for_byte() {
        let dir = fixture_project();
        let before = snapshot(dir.path());

        init_project(dir.path()).expect("init");
        std::fs::remove_dir_all(dir.path().join(".veneer")).expect("remove .veneer");

        assert_eq!(before, snapshot(dir.path()));
    }

    #[test]
    fn second_init_is_a_noop() {
        let dir = fixture_project();

        assert_eq!(
            init_project(dir.path()).expect("init"),
            InitOutcome::Created
        );
        let after_first = snapshot(dir.path());

        assert_eq!(
            init_project(dir.path()).expect("re-init"),
            InitOutcome::AlreadyInitialized
        );
        assert_eq!(after_first, snapshot(dir.path()));
    }

    #[test]
    fn missing_project_root_errors_with_resolved_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("does-not-exist");

        let err = init_project(&missing).expect_err("should fail");
        assert!(
            err.to_string().contains(&missing.display().to_string()),
            "error should name the resolved path: {err}"
        );
    }

    #[test]
    fn detects_each_vite_config_flavor() {
        for flavor in ViteConfigFlavor::ALL {
            let dir = tempfile::tempdir().expect("tempdir");
            std::fs::write(dir.path().join(flavor.file_name()), "export default {};\n")
                .expect("write config");
            assert_eq!(detect_vite_config(dir.path()), Some(flavor));
        }

        let empty = tempfile::tempdir().expect("tempdir");
        assert_eq!(detect_vite_config(empty.path()), None);
    }

    #[test]
    fn detection_follows_vite_resolution_order() {
        let dir = tempfile::tempdir().expect("tempdir");
        for flavor in ViteConfigFlavor::ALL {
            std::fs::write(dir.path().join(flavor.file_name()), "export default {};\n")
                .expect("write config");
        }
        assert_eq!(detect_vite_config(dir.path()), Some(ViteConfigFlavor::Js));
    }

    #[test]
    fn instruction_names_the_detected_flavor() {
        for flavor in ViteConfigFlavor::ALL {
            let text = integration_instruction(Some(flavor));
            assert!(text.contains(flavor.file_name()), "missing name in: {text}");
            assert!(text.contains("veneer(),"));
        }

        let none = integration_instruction(None);
        assert!(none.contains("No Vite config detected"));
        assert!(none.contains("veneer(),"));
    }

    #[test]
    fn vite_config_never_modified_by_init() {
        let dir = fixture_project();
        let config_path = dir.path().join("vite.config.ts");
        let before = std::fs::read(&config_path).expect("read before");

        init_project(dir.path()).expect("init");
        init_project(dir.path()).expect("re-init");

        assert_eq!(before, std::fs::read(&config_path).expect("read after"));
    }
}
