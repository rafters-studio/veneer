use crate::model::ServiceDesignArtifact;
use crate::validation::{ArtifactValidator, Severity, ValidationContext, ValidationIssue};

pub struct InternalValidator;

fn check_non_empty(
    issues: &mut Vec<ValidationIssue>,
    artifact_title: &str,
    field: &str,
    value: &str,
    severity: Severity,
) {
    if value.trim().is_empty() {
        issues.push(ValidationIssue {
            severity,
            artifact_title: artifact_title.to_string(),
            field: field.to_string(),
            message: format!("{field} must not be empty"),
        });
    }
}

impl ArtifactValidator for InternalValidator {
    fn validate(
        &self,
        artifact: &ServiceDesignArtifact,
        _context: &ValidationContext,
    ) -> Vec<ValidationIssue> {
        match artifact {
            ServiceDesignArtifact::Blueprint(bp) => validate_blueprint(bp),
            ServiceDesignArtifact::JourneyMap(jm) => validate_journey_map(jm),
            ServiceDesignArtifact::EcosystemMap(em) => validate_ecosystem_map(em),
            ServiceDesignArtifact::Persona(p) => validate_persona(p),
            ServiceDesignArtifact::PainPointMatrix(pm) => validate_pain_point_matrix(pm),
        }
    }
}

fn validate_blueprint(bp: &crate::model::Blueprint) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let title = &bp.meta.title;

    check_non_empty(
        &mut issues,
        title,
        "meta.title",
        &bp.meta.title,
        Severity::Error,
    );
    check_non_empty(
        &mut issues,
        title,
        "meta.primary_persona",
        &bp.meta.primary_persona,
        Severity::Error,
    );
    check_non_empty(
        &mut issues,
        title,
        "meta.trigger",
        &bp.meta.trigger,
        Severity::Error,
    );
    check_non_empty(
        &mut issues,
        title,
        "meta.scope",
        &bp.meta.scope,
        Severity::Error,
    );

    if bp.steps.is_empty() {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            artifact_title: title.clone(),
            field: "steps".into(),
            message: "Blueprint must have at least 1 step".into(),
        });
    }

    for (i, step) in bp.steps.iter().enumerate() {
        let prefix = format!("steps[{i}]");
        check_non_empty(
            &mut issues,
            title,
            &format!("{prefix}.evidence"),
            &step.evidence,
            Severity::Error,
        );
        check_non_empty(
            &mut issues,
            title,
            &format!("{prefix}.customer_actions"),
            &step.customer_actions,
            Severity::Error,
        );
        check_non_empty(
            &mut issues,
            title,
            &format!("{prefix}.frontstage"),
            &step.frontstage,
            Severity::Error,
        );
        check_non_empty(
            &mut issues,
            title,
            &format!("{prefix}.backstage"),
            &step.backstage,
            Severity::Error,
        );
        check_non_empty(
            &mut issues,
            title,
            &format!("{prefix}.support_processes"),
            &step.support_processes,
            Severity::Error,
        );
        check_non_empty(
            &mut issues,
            title,
            &format!("{prefix}.emotional_label"),
            &step.emotional_label,
            Severity::Error,
        );
    }

    issues
}

fn validate_journey_map(jm: &crate::model::JourneyMap) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let title = &jm.meta.title;

    check_non_empty(
        &mut issues,
        title,
        "meta.title",
        &jm.meta.title,
        Severity::Error,
    );
    check_non_empty(
        &mut issues,
        title,
        "meta.persona",
        &jm.meta.persona,
        Severity::Error,
    );
    check_non_empty(
        &mut issues,
        title,
        "meta.scenario",
        &jm.meta.scenario,
        Severity::Error,
    );
    check_non_empty(
        &mut issues,
        title,
        "meta.goal",
        &jm.meta.goal,
        Severity::Error,
    );

    if jm.phases.is_empty() {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            artifact_title: title.clone(),
            field: "phases".into(),
            message: "JourneyMap must have at least 1 phase".into(),
        });
    }

    for (i, phase) in jm.phases.iter().enumerate() {
        let prefix = format!("phases[{i}]");
        check_non_empty(
            &mut issues,
            title,
            &format!("{prefix}.name"),
            &phase.name,
            Severity::Error,
        );
        check_non_empty(
            &mut issues,
            title,
            &format!("{prefix}.emotions"),
            &phase.emotions,
            Severity::Error,
        );
    }

    issues
}

fn validate_ecosystem_map(em: &crate::model::EcosystemMap) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let title = &em.title;

    check_non_empty(&mut issues, title, "title", &em.title, Severity::Error);
    check_non_empty(
        &mut issues,
        title,
        "core_service",
        &em.core_service,
        Severity::Error,
    );

    if em.actors.is_empty() {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            artifact_title: title.clone(),
            field: "actors".into(),
            message: "EcosystemMap must have at least 1 actor".into(),
        });
    }

    for (i, mot) in em.moments_of_truth.iter().enumerate() {
        let prefix = format!("moments_of_truth[{i}]");
        check_non_empty(
            &mut issues,
            title,
            &format!("{prefix}.success_state"),
            &mot.success_state,
            Severity::Error,
        );
        check_non_empty(
            &mut issues,
            title,
            &format!("{prefix}.failure_state"),
            &mot.failure_state,
            Severity::Error,
        );
    }

    for (i, fm) in em.failure_modes.iter().enumerate() {
        let prefix = format!("failure_modes[{i}]");
        check_non_empty(
            &mut issues,
            title,
            &format!("{prefix}.mode"),
            &fm.mode,
            Severity::Error,
        );
        check_non_empty(
            &mut issues,
            title,
            &format!("{prefix}.impact"),
            &fm.impact,
            Severity::Error,
        );
        check_non_empty(
            &mut issues,
            title,
            &format!("{prefix}.recovery"),
            &fm.recovery,
            Severity::Error,
        );
    }

    issues
}

fn validate_persona(p: &crate::model::Persona) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let title = &p.overview.name;

    check_non_empty(
        &mut issues,
        title,
        "overview.name",
        &p.overview.name,
        Severity::Error,
    );
    check_non_empty(
        &mut issues,
        title,
        "overview.role",
        &p.overview.role,
        Severity::Error,
    );
    check_non_empty(
        &mut issues,
        title,
        "background",
        &p.background,
        Severity::Error,
    );

    if p.goals.is_empty() {
        issues.push(ValidationIssue {
            severity: Severity::Warning,
            artifact_title: title.clone(),
            field: "goals".into(),
            message: "Persona should have at least 1 goal".into(),
        });
    }

    issues
}

fn validate_pain_point_matrix(pm: &crate::model::PainPointMatrix) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let title = "PainPointMatrix";

    for (i, theme) in pm.themes.iter().enumerate() {
        if theme.composite_score < 0.0 {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                artifact_title: title.into(),
                field: format!("themes[{i}].composite_score"),
                message: format!(
                    "composite_score must be >= 0.0, got {}",
                    theme.composite_score
                ),
            });
        }
    }

    for (i, dim) in pm.rubric.dimensions.iter().enumerate() {
        if dim.weight <= 0.0 {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                artifact_title: title.into(),
                field: format!("rubric.dimensions[{i}].weight"),
                message: format!("weight must be > 0.0, got {}", dim.weight),
            });
        }
    }

    if !pm.rubric.dimensions.is_empty() {
        let total_weight: f32 = pm.rubric.dimensions.iter().map(|d| d.weight).sum();
        if (total_weight - 1.0).abs() > 0.05 {
            issues.push(ValidationIssue {
                severity: Severity::Warning,
                artifact_title: title.into(),
                field: "rubric.dimensions".into(),
                message: format!(
                    "Dimension weights should sum to approximately 1.0, got {total_weight:.2}"
                ),
            });
        }
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use std::collections::HashMap;

    fn empty_context() -> ValidationContext {
        ValidationContext::default()
    }

    #[test]
    fn blueprint_with_empty_evidence_produces_error() {
        let bp = ServiceDesignArtifact::Blueprint(Blueprint {
            meta: BlueprintMeta {
                title: "Test".into(),
                primary_persona: "Alex".into(),
                secondary_persona: None,
                trigger: "trigger".into(),
                scope: "scope".into(),
                channels: vec![],
            },
            steps: vec![BlueprintStep {
                name: "Step 1".into(),
                evidence: "".into(), // empty
                customer_actions: "action".into(),
                frontstage: "front".into(),
                backstage: "back".into(),
                support_processes: "support".into(),
                pain_points: vec![],
                emotional_state: EmotionalScore::new(0).expect("valid score"),
                emotional_label: "Neutral".into(),
                metrics: vec![],
                moments_of_truth: vec![],
            }],
            dependency_map: vec![],
            design_decisions: vec![],
            open_questions: vec![],
        });

        let validator = InternalValidator;
        let issues = validator.validate(&bp, &empty_context());

        let evidence_errors: Vec<_> = issues
            .iter()
            .filter(|i| i.field.contains("evidence") && i.severity == Severity::Error)
            .collect();
        assert_eq!(evidence_errors.len(), 1);
    }

    #[test]
    fn blueprint_with_no_steps_produces_error() {
        let bp = ServiceDesignArtifact::Blueprint(Blueprint {
            meta: BlueprintMeta {
                title: "Test".into(),
                primary_persona: "Alex".into(),
                secondary_persona: None,
                trigger: "trigger".into(),
                scope: "scope".into(),
                channels: vec![],
            },
            steps: vec![],
            dependency_map: vec![],
            design_decisions: vec![],
            open_questions: vec![],
        });

        let validator = InternalValidator;
        let issues = validator.validate(&bp, &empty_context());

        let step_errors: Vec<_> = issues
            .iter()
            .filter(|i| i.field == "steps" && i.severity == Severity::Error)
            .collect();
        assert_eq!(step_errors.len(), 1);
    }

    #[test]
    fn moment_of_truth_with_empty_failure_state_produces_error() {
        let em = ServiceDesignArtifact::EcosystemMap(EcosystemMap {
            title: "Test Ecosystem".into(),
            core_service: "Service".into(),
            actors: vec![Actor {
                name: "Dev".into(),
                actor_type: ActorType::Primary,
                description: "A developer".into(),
            }],
            channels: vec![],
            value_exchanges: vec![],
            moments_of_truth: vec![MomentOfTruth {
                moment: "First use".into(),
                success_state: "Works great".into(),
                failure_state: "".into(), // empty
                why_it_matters: None,
            }],
            failure_modes: vec![],
        });

        let validator = InternalValidator;
        let issues = validator.validate(&em, &empty_context());

        let mot_errors: Vec<_> = issues
            .iter()
            .filter(|i| i.field.contains("failure_state") && i.severity == Severity::Error)
            .collect();
        assert_eq!(mot_errors.len(), 1);
    }

    #[test]
    fn persona_with_empty_name_produces_error() {
        let persona = ServiceDesignArtifact::Persona(Persona {
            overview: PersonaOverview {
                name: "".into(), // empty
                age: None,
                role: "Developer".into(),
                location: None,
                experience: None,
            },
            background: "Some background".into(),
            goals: vec!["A goal".into()],
            pain_points: vec![],
            current_tools: vec![],
            behavioral_patterns: vec![],
            technology_expertise: vec![],
            success_metrics: vec![],
            quotes: vec![],
        });

        let validator = InternalValidator;
        let issues = validator.validate(&persona, &empty_context());

        let name_errors: Vec<_> = issues
            .iter()
            .filter(|i| i.field == "overview.name" && i.severity == Severity::Error)
            .collect();
        assert_eq!(name_errors.len(), 1);
    }

    #[test]
    fn pain_point_matrix_negative_composite_score_produces_error() {
        let matrix = ServiceDesignArtifact::PainPointMatrix(PainPointMatrix {
            rubric: ScoringRubric {
                dimensions: vec![ScoringDimension {
                    name: "Freq".into(),
                    weight: 1.0,
                    scale_max: 5,
                }],
            },
            themes: vec![PainTheme {
                name: "Bad theme".into(),
                scores: HashMap::new(),
                composite_score: -1.0, // negative
                evidence: "evidence".into(),
                monthly_cost: None,
            }],
            ranked_priorities: vec![],
            disconfirmation_log: vec![],
            probe_backlog: vec![],
        });

        let validator = InternalValidator;
        let issues = validator.validate(&matrix, &empty_context());

        let score_errors: Vec<_> = issues
            .iter()
            .filter(|i| i.field.contains("composite_score") && i.severity == Severity::Error)
            .collect();
        assert_eq!(score_errors.len(), 1);
    }

    #[test]
    fn dimension_weights_not_summing_to_one_produces_warning() {
        let matrix = ServiceDesignArtifact::PainPointMatrix(PainPointMatrix {
            rubric: ScoringRubric {
                dimensions: vec![
                    ScoringDimension {
                        name: "A".into(),
                        weight: 0.2,
                        scale_max: 5,
                    },
                    ScoringDimension {
                        name: "B".into(),
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

        let validator = InternalValidator;
        let issues = validator.validate(&matrix, &empty_context());

        let weight_warnings: Vec<_> = issues
            .iter()
            .filter(|i| i.field == "rubric.dimensions" && i.severity == Severity::Warning)
            .collect();
        assert_eq!(weight_warnings.len(), 1);
    }

    #[test]
    fn persona_with_no_goals_produces_warning() {
        let persona = ServiceDesignArtifact::Persona(Persona {
            overview: PersonaOverview {
                name: "Alex".into(),
                age: None,
                role: "Dev".into(),
                location: None,
                experience: None,
            },
            background: "Background".into(),
            goals: vec![], // empty
            pain_points: vec![],
            current_tools: vec![],
            behavioral_patterns: vec![],
            technology_expertise: vec![],
            success_metrics: vec![],
            quotes: vec![],
        });

        let validator = InternalValidator;
        let issues = validator.validate(&persona, &empty_context());

        let goal_warnings: Vec<_> = issues
            .iter()
            .filter(|i| i.field == "goals" && i.severity == Severity::Warning)
            .collect();
        assert_eq!(goal_warnings.len(), 1);
    }
}
