//! The one check behind `rafters check` and the veneer language server.
//!
//! Every caller -- the CLI, the Node package, the language server -- asks this
//! crate, so agents, editors, git hooks and CI all get the same answer.

/// The version of the check, shared by every caller.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_the_workspace() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }
}
