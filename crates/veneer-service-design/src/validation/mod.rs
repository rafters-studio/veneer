mod cross_ref;
mod internal;

use std::collections::HashSet;

use crate::model::ServiceDesignArtifact;

pub use cross_ref::CrossReferenceValidator;
pub use internal::InternalValidator;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub artifact_title: String,
    pub field: String,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct ValidationContext {
    pub known_personas: HashSet<String>,
    pub known_blueprints: HashSet<String>,
    pub known_journeys: HashSet<String>,
}

#[derive(Debug, Default)]
pub struct ValidationResult {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationResult {
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.severity == Severity::Error)
    }

    pub fn errors(&self) -> impl Iterator<Item = &ValidationIssue> {
        self.issues.iter().filter(|i| i.severity == Severity::Error)
    }

    pub fn warnings(&self) -> impl Iterator<Item = &ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
    }
}

pub trait ArtifactValidator: Send + Sync {
    fn validate(
        &self,
        artifact: &ServiceDesignArtifact,
        context: &ValidationContext,
    ) -> Vec<ValidationIssue>;
}

pub struct ArtifactValidatorPipeline {
    validators: Vec<Box<dyn ArtifactValidator>>,
}

impl Default for ArtifactValidatorPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl ArtifactValidatorPipeline {
    pub fn new() -> Self {
        Self {
            validators: vec![
                Box::new(InternalValidator),
                Box::new(CrossReferenceValidator),
            ],
        }
    }

    pub fn validate_all(&self, artifacts: &[ServiceDesignArtifact]) -> ValidationResult {
        let context = Self::build_context(artifacts);

        let issues = artifacts
            .iter()
            .flat_map(|artifact| {
                self.validators
                    .iter()
                    .flat_map(|v| v.validate(artifact, &context))
                    .collect::<Vec<_>>()
            })
            .collect();

        ValidationResult { issues }
    }

    fn build_context(artifacts: &[ServiceDesignArtifact]) -> ValidationContext {
        let mut ctx = ValidationContext::default();

        for artifact in artifacts {
            match artifact {
                ServiceDesignArtifact::Persona(p) => {
                    ctx.known_personas.insert(p.overview.name.clone());
                }
                ServiceDesignArtifact::Blueprint(b) => {
                    ctx.known_blueprints.insert(b.meta.title.clone());
                }
                ServiceDesignArtifact::JourneyMap(j) => {
                    ctx.known_journeys.insert(j.meta.title.clone());
                }
                _ => {}
            }
        }

        ctx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn valid_blueprint() -> ServiceDesignArtifact {
        ServiceDesignArtifact::Blueprint(Blueprint {
            meta: BlueprintMeta {
                title: "Test Blueprint".into(),
                primary_persona: "Alex".into(),
                secondary_persona: None,
                trigger: "Discovers tool".into(),
                scope: "First run".into(),
                channels: vec![],
            },
            steps: vec![BlueprintStep {
                name: "Step 1".into(),
                evidence: "Some evidence".into(),
                customer_actions: "Does something".into(),
                frontstage: "Sees something".into(),
                backstage: "Processes something".into(),
                support_processes: "Needs something".into(),
                pain_points: vec![],
                emotional_state: EmotionalScore::new(1).expect("valid score"),
                emotional_label: "Happy".into(),
                metrics: vec![],
                moments_of_truth: vec![],
            }],
            dependency_map: vec![],
            design_decisions: vec![],
            open_questions: vec![],
        })
    }

    fn valid_persona() -> ServiceDesignArtifact {
        ServiceDesignArtifact::Persona(Persona {
            overview: PersonaOverview {
                name: "Alex".into(),
                age: None,
                role: "Developer".into(),
                location: None,
                experience: None,
            },
            background: "5 years of experience".into(),
            goals: vec!["Ship fast".into()],
            pain_points: vec![],
            current_tools: vec![],
            behavioral_patterns: vec![],
            technology_expertise: vec![],
            success_metrics: vec![],
            quotes: vec![],
        })
    }

    #[test]
    fn pipeline_collects_issues_from_both_validators() {
        // Blueprint referencing unknown persona triggers cross-ref warning,
        // and we can also trigger internal errors by making it invalid.
        let bp = ServiceDesignArtifact::Blueprint(Blueprint {
            meta: BlueprintMeta {
                title: "Test".into(),
                primary_persona: "Unknown Person".into(),
                secondary_persona: None,
                trigger: "trigger".into(),
                scope: "scope".into(),
                channels: vec![],
            },
            steps: vec![], // empty steps -> internal error
            dependency_map: vec![],
            design_decisions: vec![],
            open_questions: vec![],
        });

        let pipeline = ArtifactValidatorPipeline::new();
        let result = pipeline.validate_all(&[bp]);

        // Should have at least one error (no steps) and one warning (unknown persona)
        assert!(result.has_errors());
        assert!(result.errors().count() >= 1);
        assert!(result.warnings().count() >= 1);
    }

    #[test]
    fn pipeline_builds_context_from_artifacts() {
        let persona = valid_persona();
        let bp = valid_blueprint();

        let pipeline = ArtifactValidatorPipeline::new();
        let result = pipeline.validate_all(&[persona, bp]);

        // Blueprint references "Alex" which exists as a persona, so no cross-ref warning
        let persona_warnings: Vec<_> = result
            .warnings()
            .filter(|i| i.field.contains("primary_persona"))
            .collect();
        assert!(persona_warnings.is_empty());
    }

    #[test]
    fn valid_blueprint_produces_zero_issues() {
        let persona = valid_persona();
        let bp = valid_blueprint();

        let pipeline = ArtifactValidatorPipeline::new();
        let result = pipeline.validate_all(&[persona, bp]);

        // Filter to blueprint issues only
        let bp_issues: Vec<_> = result
            .issues
            .iter()
            .filter(|i| i.artifact_title == "Test Blueprint")
            .collect();
        assert!(
            bp_issues.is_empty(),
            "Expected no issues, got: {bp_issues:?}"
        );
    }

    #[test]
    fn pipeline_with_pain_point_matrix_weights() {
        let matrix = ServiceDesignArtifact::PainPointMatrix(PainPointMatrix {
            rubric: ScoringRubric {
                dimensions: vec![
                    ScoringDimension {
                        name: "Frequency".into(),
                        weight: 0.3,
                        scale_max: 5,
                    },
                    ScoringDimension {
                        name: "Severity".into(),
                        weight: 0.3,
                        scale_max: 5,
                    },
                ],
            },
            themes: vec![],
            ranked_priorities: vec![],
            disconfirmation_log: vec![],
            probe_backlog: vec![],
        });

        let pipeline = ArtifactValidatorPipeline::new();
        let result = pipeline.validate_all(&[matrix]);

        // Weights sum to 0.6, not ~1.0, so should produce a warning
        let weight_warnings: Vec<_> = result
            .warnings()
            .filter(|i| i.field == "rubric.dimensions")
            .collect();
        assert_eq!(weight_warnings.len(), 1);
    }
}
