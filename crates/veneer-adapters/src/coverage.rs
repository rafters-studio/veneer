//! Coverage tracking (FR-VEN-009): which discovered components are
//! documented versus not yet -- honest numbers, never a blank or a 404.
//!
//! The denominator is the discovered set (FR-VEN-017): an item veneer
//! never discovered cannot be reported missing, and an item veneer did
//! discover can never silently vanish from the numbers. An item counts as
//! documented only when discovery marked it generatable
//! ([`DiscoveredItem::generated`]) AND the render pipeline
//! ([`render_component`]) actually produced its preview -- a render
//! failure counts as not-yet-documented, with its failure state, never as
//! documented. No partial success is rounded up to "covered".
//!
//! Every uncovered item gets an explicit "not yet documented" placeholder
//! artifact ([`not_yet_documented_placeholder`]): a real MDX page naming
//! the item and the honest reason, instead of a blank page or an error.
//!
//! Out of scope here, by requirement: closing the coverage gap itself
//! (FR-VEN-003 per component) and staleness marking (FR-VEN-014).

use std::path::Path;

use crate::intelligence::render_component;
use crate::rafters_source::IntelligenceSource;
use crate::registry::{ComponentRegistry, DiscoveredItem, DiscoveredKind, RegistryError};
use crate::ts_helpers::kebab_case;

/// Coverage of a discovered set: documented versus not-yet-documented,
/// with the discovered set as the denominator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageReport {
    /// Names of discovered items whose preview rendered.
    pub documented: Vec<String>,
    /// Names of discovered items not yet documented (nothing generatable,
    /// or generation/render failed).
    pub not_yet_documented: Vec<String>,
    /// Size of the discovered set: always
    /// `documented.len() + not_yet_documented.len()`.
    pub total: usize,
}

impl CoverageReport {
    /// Fold assessed items into the report. Every assessed item lands in
    /// exactly one bucket, so the totals cannot drift from the item states.
    pub fn from_assessed(assessed: &[AssessedItem]) -> Self {
        let mut documented: Vec<String> = Vec::new();
        let mut not_yet_documented: Vec<String> = Vec::new();
        for entry in assessed {
            match &entry.state {
                CoverageState::Documented => documented.push(entry.item.name.clone()),
                CoverageState::NotYetDocumented { .. } => {
                    not_yet_documented.push(entry.item.name.clone())
                }
            }
        }
        Self {
            total: documented.len() + not_yet_documented.len(),
            documented,
            not_yet_documented,
        }
    }
}

/// The coverage state of one discovered item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoverageState {
    /// Discovery marked the item generatable and its preview rendered.
    Documented,
    /// Not documented yet, with the honest reason: what discovery or the
    /// render pipeline reported for this item.
    NotYetDocumented { reason: String },
}

/// One discovered item together with its assessed coverage state.
#[derive(Debug, Clone)]
pub struct AssessedItem {
    pub item: DiscoveredItem,
    pub state: CoverageState,
}

/// Assess the coverage of every discovered item, in the discovered order.
///
/// An item is [`CoverageState::Documented`] only when both gates pass:
/// discovery marked it generatable and [`render_component`] produced its
/// preview. Everything else is [`CoverageState::NotYetDocumented`] with
/// the failure state as the reason -- a failed generation is never
/// counted as documented.
pub fn assess_coverage(items: &[DiscoveredItem], source: &IntelligenceSource) -> Vec<AssessedItem> {
    items
        .iter()
        .map(|item| AssessedItem {
            item: item.clone(),
            state: assess_item(item, source),
        })
        .collect()
}

fn assess_item(item: &DiscoveredItem, source: &IntelligenceSource) -> CoverageState {
    if !item.generated {
        return CoverageState::NotYetDocumented {
            reason: format!(
                "discovered at {} but nothing is generatable from it yet",
                item.source_path.display()
            ),
        };
    }
    match render_component(item, source) {
        Ok(_) => CoverageState::Documented,
        Err(error) => CoverageState::NotYetDocumented {
            reason: error.to_string(),
        },
    }
}

impl ComponentRegistry {
    /// Query coverage for a project: discover the set, assess every item,
    /// and fold the states into a [`CoverageReport`].
    ///
    /// This is an associated function taking the project root, not
    /// `&self`: the registry's scanned cache silently drops items that
    /// fail extraction, so measuring coverage against it would violate
    /// the discovered-set denominator. Like
    /// [`ComponentRegistry::discover`], coverage works from the project
    /// source directly.
    pub fn coverage(
        project_root: &Path,
        source: &IntelligenceSource,
    ) -> Result<CoverageReport, RegistryError> {
        let items = Self::discover(project_root, source)?;
        Ok(CoverageReport::from_assessed(&assess_coverage(
            &items, source,
        )))
    }
}

/// A placeholder artifact ready to be written alongside generator output:
/// the file name it should be emitted under and its full content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceholderArtifact {
    /// Kebab-cased file name, for example `ghost-widget.mdx`.
    pub file_name: String,
    /// Complete MDX page content -- a real artifact, never blank.
    pub content: String,
}

/// Build the explicit "not yet documented" placeholder artifact for one
/// uncovered item: an MDX page (matching the extract output family) that
/// names the item, where it was discovered, and the honest reason it is
/// not documented yet. Emitting this page is what turns a coverage gap
/// into a visible state instead of a blank page or a 404.
pub fn not_yet_documented_placeholder(
    item: &DiscoveredItem,
    reason: &str,
    layout: Option<&str>,
) -> PlaceholderArtifact {
    let layout_line = layout
        .map(|value| format!("layout: {value}\n"))
        .unwrap_or_default();
    let kind = kind_label(item.kind);
    let name = &item.name;
    let source_path = item.source_path.display();

    let content = format!(
        "\
---
{layout_line}title: {name}
description: {name} is not yet documented.
status: not-yet-documented
---

# {name}

## Not yet documented

Veneer discovered this {kind} at `{source_path}` but has not documented
it yet.

Reason: {reason}

This page exists so the gap is explicit: an uncovered {kind} surfaces as
this placeholder, never as a blank page or a 404.
"
    );

    PlaceholderArtifact {
        file_name: format!("{}.mdx", kebab_case(name)),
        content,
    }
}

fn kind_label(kind: DiscoveredKind) -> &'static str {
    match kind {
        DiscoveredKind::Component => "component",
        DiscoveredKind::Composite => "composite",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rafters_source::read_rafters_namespace;
    use std::path::PathBuf;

    fn fixture_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/coverage/partial")
    }

    fn assessed_fixture() -> Vec<AssessedItem> {
        let root = fixture_root();
        let source = read_rafters_namespace(&root).expect("fixture namespace must read");
        let items = ComponentRegistry::discover(&root, &source).expect("fixture must discover");
        assess_coverage(&items, &source)
    }

    // AC: coverage numbers are exact against a fixture with known partial
    // coverage. The fixture discovers exactly three items: Button
    // (renderable), Broken (unparseable source), ghost-widget (installed
    // in the rafters config with no source file).
    #[test]
    fn coverage_numbers_are_exact_against_partial_fixture() {
        let report = CoverageReport::from_assessed(&assessed_fixture());
        assert_eq!(report.total, 3, "denominator is the full discovered set");
        assert_eq!(report.documented, ["Button"]);
        assert_eq!(report.not_yet_documented, ["Broken", "ghost-widget"]);
    }

    // AC: coverage status is queryable, with the discovered set as the
    // denominator.
    #[test]
    fn coverage_is_queryable_from_the_registry() {
        let root = fixture_root();
        let source = read_rafters_namespace(&root).expect("fixture namespace must read");
        let report = ComponentRegistry::coverage(&root, &source).expect("coverage must compute");
        assert_eq!(report.total, 3);
        assert_eq!(
            report.total,
            report.documented.len() + report.not_yet_documented.len(),
            "every discovered item lands in exactly one bucket"
        );
    }

    // AC: a component that failed generation counts as not-yet-documented
    // (with its failure state), never as documented.
    #[test]
    fn failed_generation_is_not_yet_documented_with_its_failure_state() {
        let assessed = assessed_fixture();
        let broken = assessed
            .iter()
            .find(|entry| entry.item.name == "Broken")
            .expect("Broken must be assessed");
        match &broken.state {
            CoverageState::NotYetDocumented { reason } => {
                assert!(!reason.is_empty(), "the failure state must be carried");
                assert!(
                    reason.contains("broken.tsx"),
                    "reason names the source: {reason}"
                );
            }
            CoverageState::Documented => panic!("a failed generation must never be documented"),
        }
    }

    // AC: a render failure on a discovery-generatable item is still
    // not-yet-documented -- generation success at discovery time is not
    // rounded up to covered.
    #[test]
    fn render_failure_is_never_documented() {
        let item = DiscoveredItem {
            name: "Vanished".to_string(),
            kind: DiscoveredKind::Component,
            source_path: PathBuf::from("/nonexistent/vanished.tsx"),
            generated: true,
        };
        let assessed = assess_coverage(&[item], &IntelligenceSource::NoSource);
        assert_eq!(assessed.len(), 1);
        match &assessed[0].state {
            CoverageState::NotYetDocumented { reason } => {
                assert!(
                    reason.contains("Vanished"),
                    "failure names the item: {reason}"
                );
            }
            CoverageState::Documented => panic!("a render failure must never be documented"),
        }
    }

    // AC: an undocumented component surfaces an explicit "not yet
    // documented" artifact -- a real page, not a blank.
    #[test]
    fn placeholder_is_a_real_explicit_artifact() {
        let assessed = assessed_fixture();
        let ghost = assessed
            .iter()
            .find(|entry| entry.item.name == "ghost-widget")
            .expect("ghost-widget must be assessed");
        let CoverageState::NotYetDocumented { reason } = &ghost.state else {
            panic!("an installed name with no source cannot be documented");
        };

        let artifact = not_yet_documented_placeholder(&ghost.item, reason, None);
        assert_eq!(artifact.file_name, "ghost-widget.mdx");
        assert!(artifact.content.starts_with("---\n"), "real frontmatter");
        assert!(artifact.content.contains("status: not-yet-documented"));
        assert!(artifact.content.contains("# ghost-widget"));
        assert!(artifact.content.contains("Not yet documented"));
        assert!(
            artifact.content.contains("config.rafters.json"),
            "the honest reason names where the item is declared"
        );
        assert!(
            artifact.content.trim().lines().count() > 5,
            "a placeholder is a page, never a blank"
        );
    }

    #[test]
    fn placeholder_carries_the_layout_when_given() {
        let item = DiscoveredItem {
            name: "HeroBanner".to_string(),
            kind: DiscoveredKind::Composite,
            source_path: PathBuf::from("composites/hero-banner.composite.json"),
            generated: false,
        };
        let artifact =
            not_yet_documented_placeholder(&item, "no generation pipeline", Some("../Docs.astro"));
        assert_eq!(artifact.file_name, "hero-banner.mdx");
        assert!(artifact.content.contains("layout: ../Docs.astro"));
        assert!(artifact.content.contains("this composite"));
    }
}
