//! Mode detection and framework dispatch (FR-VEN-033).
//!
//! Veneer NAMES which mode it runs in, detected from what the pointed-at
//! project root contains -- never a flag, never guessed:
//!
//! - [`Mode::Default`]: pointed at the rafters monorepo. Rich source is
//!   present: a component matrix plus `.behavior.ts` files. Builds official
//!   docs.
//! - [`Mode::Sidecar`]: pointed at an installed consumer project. No matrix,
//!   no behavior files -- only the installed tree (framework component
//!   files plus `*.classes.ts`).
//!
//! [`detect_mode`] elevates the same per-component signal
//! `intelligence.rs`/`config_interface.rs` already use to distinguish the
//! `Config`-interface path from the `*Props` fallback --
//! [`crate::intelligence::is_behavior_file`] -- to a project-level
//! determination, so the two never fork.
//!
//! Separately, veneer DISPATCHES the source reader by the framework a
//! project's `.rafters/config.rafters.json` declares
//! (`crate::rafters_source::read_framework_declaration`). `componentTarget`
//! is an input fact, acted on only to select an adapter
//! ([`dispatch_framework`]); an unsupported value is an observation, never
//! an inference -- no props or intelligence are read from a source shape
//! veneer has no adapter for.

use std::path::Path;

use walkdir::WalkDir;

use crate::intelligence::is_behavior_file;
use crate::matrix::default_matrix_path;
use crate::react::ReactAdapter;
use crate::registry::is_walkable_entry;
use crate::traits::FrameworkAdapter;

/// Which mode veneer runs in for a given project root (FR-VEN-033).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The rafters monorepo: a component matrix
    /// (`docs/spec/matrix/components.jsonl`) and at least one `.behavior.ts`
    /// file are present.
    Default,
    /// An installed consumer project: no matrix, no behavior files.
    Sidecar,
}

/// Detect which mode `project_root` is running in.
///
/// [`Mode::Default`] requires both signals present: the component matrix
/// ([`crate::matrix::default_matrix_path`]) and at least one `.behavior.ts`
/// file anywhere under the project ([`is_behavior_file`], the same
/// predicate `crate::intelligence::read_component_config` uses
/// per-component -- not forked here). Either signal absent -- including the
/// installed-tree shape of framework component files plus `*.classes.ts`
/// with neither present -- is [`Mode::Sidecar`].
pub fn detect_mode(project_root: &Path) -> Mode {
    let matrix_present = default_matrix_path(project_root).is_file();
    if matrix_present && has_behavior_file(project_root) {
        Mode::Default
    } else {
        Mode::Sidecar
    }
}

/// True when any file under `project_root` is a `.behavior.ts` file. Walks
/// with the same directory filter discovery uses
/// ([`crate::registry::is_walkable_entry`]) so the two walks agree on what
/// counts as project source.
fn has_behavior_file(project_root: &Path) -> bool {
    if !project_root.is_dir() {
        return false;
    }
    WalkDir::new(project_root)
        .follow_links(true)
        .into_iter()
        .filter_entry(is_walkable_entry)
        .filter_map(Result::ok)
        .any(|entry| entry.file_type().is_file() && is_behavior_file(entry.path()))
}

/// The outcome of dispatching on a project's declared `componentTarget`
/// (FR-VEN-033).
pub enum FrameworkDispatch {
    /// `componentTarget` names a framework veneer has an adapter for.
    Supported(Box<dyn FrameworkAdapter>),
    /// `componentTarget` names a framework veneer has no adapter for. An
    /// observation, not an error: discovery still runs and the item still
    /// surfaces, but no props or intelligence are read from its source --
    /// veneer never infers from a shape it cannot parse.
    Unsupported {
        /// The framework exactly as declared.
        declared: String,
    },
}

/// Select the framework adapter for a declared `componentTarget`
/// (FR-VEN-033). This is the single construction site for a framework
/// adapter driven by that declaration.
///
/// `None` -- the field absent, or `.rafters/config.rafters.json` itself
/// absent -- is the project's silence, not a declaration of "no framework":
/// every project without this declaration already ran on veneer's one
/// adapter (React), so silence resolves to it rather than becoming a
/// spurious [`FrameworkDispatch::Unsupported`]. An explicit, non-`"react"`
/// declaration is never inferred as React: it is
/// [`FrameworkDispatch::Unsupported`], naming exactly what was declared.
pub fn dispatch_framework(component_target: Option<&str>) -> FrameworkDispatch {
    match component_target {
        None | Some("react") => FrameworkDispatch::Supported(Box::new(ReactAdapter::new())),
        Some(other) => FrameworkDispatch::Unsupported {
            declared: other.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, contents).expect("write");
    }

    // AC: default mode detected when matrix + behavior present.
    #[test]
    fn default_mode_detected_when_matrix_and_behavior_present() {
        let temp = tempfile::tempdir().expect("tempdir");
        write(
            &temp.path().join("docs/spec/matrix/components.jsonl"),
            r#"{"schema":"rafters.component-line/1","name":"button"}"#,
        );
        write(
            &temp.path().join("components/button.behavior.ts"),
            "export interface Config { variant: string }",
        );
        assert_eq!(detect_mode(temp.path()), Mode::Default);
    }

    // AC: sidecar mode detected for an installed tree (tsx + classes.ts, no
    // matrix/behavior).
    #[test]
    fn sidecar_mode_detected_for_installed_tree_with_no_matrix_or_behavior() {
        let temp = tempfile::tempdir().expect("tempdir");
        write(
            &temp.path().join("components/button.tsx"),
            "export function Button() { return null; }",
        );
        write(
            &temp.path().join("components/button.classes.ts"),
            "export const buttonBase = 'inline-flex';",
        );
        assert_eq!(detect_mode(temp.path()), Mode::Sidecar);
    }

    // Matrix alone, without a behavior file, is not the rich-source shape
    // FR-VEN-033 names as Default.
    #[test]
    fn matrix_without_behavior_is_sidecar() {
        let temp = tempfile::tempdir().expect("tempdir");
        write(
            &temp.path().join("docs/spec/matrix/components.jsonl"),
            r#"{"schema":"rafters.component-line/1","name":"button"}"#,
        );
        assert_eq!(detect_mode(temp.path()), Mode::Sidecar);
    }

    // Behavior alone, without a matrix, is not the rich-source shape either.
    #[test]
    fn behavior_without_matrix_is_sidecar() {
        let temp = tempfile::tempdir().expect("tempdir");
        write(
            &temp.path().join("components/button.behavior.ts"),
            "export interface Config { variant: string }",
        );
        assert_eq!(detect_mode(temp.path()), Mode::Sidecar);
    }

    #[test]
    fn a_nonexistent_root_is_sidecar_never_a_guess() {
        assert_eq!(
            detect_mode(Path::new("/nonexistent/veneer-mode-test-root")),
            Mode::Sidecar
        );
    }

    // AC: componentTarget "react" dispatches ReactAdapter.
    #[test]
    fn component_target_react_dispatches_react_adapter() {
        match dispatch_framework(Some("react")) {
            FrameworkDispatch::Supported(adapter) => assert_eq!(adapter.name(), "react"),
            FrameworkDispatch::Unsupported { declared } => {
                panic!("react must be supported, got Unsupported({declared})")
            }
        }
    }

    // An undeclared componentTarget preserves the single-adapter behavior
    // every project without the declaration already had -- silence is not
    // an unsupported declaration.
    #[test]
    fn an_undeclared_component_target_still_dispatches_react_adapter() {
        match dispatch_framework(None) {
            FrameworkDispatch::Supported(adapter) => assert_eq!(adapter.name(), "react"),
            FrameworkDispatch::Unsupported { declared } => {
                panic!("undeclared must default to react, got Unsupported({declared})")
            }
        }
    }

    // AC: an unsupported framework yields Unsupported { declared }
    // surfaced as an observation, not an inference.
    #[test]
    fn an_unsupported_framework_is_named_never_inferred() {
        match dispatch_framework(Some("vue")) {
            FrameworkDispatch::Unsupported { declared } => assert_eq!(declared, "vue"),
            FrameworkDispatch::Supported(_) => {
                panic!("vue has no adapter and must never be inferred as react")
            }
        }
    }
}
