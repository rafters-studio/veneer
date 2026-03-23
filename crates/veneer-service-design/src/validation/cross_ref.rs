use crate::model::ServiceDesignArtifact;
use crate::validation::{ArtifactValidator, Severity, ValidationContext, ValidationIssue};

pub struct CrossReferenceValidator;

impl ArtifactValidator for CrossReferenceValidator {
    fn validate(
        &self,
        artifact: &ServiceDesignArtifact,
        context: &ValidationContext,
    ) -> Vec<ValidationIssue> {
        match artifact {
            ServiceDesignArtifact::Blueprint(bp) => validate_blueprint_refs(bp, context),
            ServiceDesignArtifact::JourneyMap(jm) => validate_journey_refs(jm, context),
            ServiceDesignArtifact::EcosystemMap(em) => validate_ecosystem_refs(em),
            ServiceDesignArtifact::Persona(_) | ServiceDesignArtifact::PainPointMatrix(_) => {
                vec![]
            }
        }
    }
}

fn check_persona_exists(
    issues: &mut Vec<ValidationIssue>,
    artifact_title: &str,
    field: &str,
    persona_name: &str,
    context: &ValidationContext,
) {
    if !persona_name.is_empty() && !context.known_personas.contains(persona_name) {
        issues.push(ValidationIssue {
            severity: Severity::Warning,
            artifact_title: artifact_title.to_string(),
            field: field.to_string(),
            message: format!(
                "References persona '{persona_name}' which does not exist in the artifact set"
            ),
        });
    }
}

fn validate_blueprint_refs(
    bp: &crate::model::Blueprint,
    context: &ValidationContext,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let title = &bp.meta.title;

    check_persona_exists(
        &mut issues,
        title,
        "meta.primary_persona",
        &bp.meta.primary_persona,
        context,
    );

    if let Some(ref secondary) = bp.meta.secondary_persona {
        check_persona_exists(
            &mut issues,
            title,
            "meta.secondary_persona",
            secondary,
            context,
        );
    }

    issues
}

fn validate_journey_refs(
    jm: &crate::model::JourneyMap,
    context: &ValidationContext,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    check_persona_exists(
        &mut issues,
        &jm.meta.title,
        "meta.persona",
        &jm.meta.persona,
        context,
    );

    issues
}

fn validate_ecosystem_refs(em: &crate::model::EcosystemMap) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    let actor_names: std::collections::HashSet<&str> =
        em.actors.iter().map(|a| a.name.as_str()).collect();

    for (i, ve) in em.value_exchanges.iter().enumerate() {
        if !actor_names.contains(ve.actor.as_str()) {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                artifact_title: em.title.clone(),
                field: format!("value_exchanges[{i}].actor"),
                message: format!(
                    "References actor '{}' which does not exist in the ecosystem's actors list",
                    ve.actor
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

    fn context_with_persona(name: &str) -> ValidationContext {
        let mut ctx = ValidationContext::default();
        ctx.known_personas.insert(name.to_string());
        ctx
    }

    #[test]
    fn blueprint_referencing_unknown_persona_produces_warning() {
        let bp = ServiceDesignArtifact::Blueprint(Blueprint {
            meta: BlueprintMeta {
                title: "Test".into(),
                primary_persona: "Unknown Person".into(),
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

        let validator = CrossReferenceValidator;
        let issues = validator.validate(&bp, &ValidationContext::default());

        let persona_warnings: Vec<_> = issues
            .iter()
            .filter(|i| i.field == "meta.primary_persona" && i.severity == Severity::Warning)
            .collect();
        assert_eq!(persona_warnings.len(), 1);
    }

    #[test]
    fn blueprint_referencing_known_persona_produces_no_warning() {
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

        let ctx = context_with_persona("Alex");
        let validator = CrossReferenceValidator;
        let issues = validator.validate(&bp, &ctx);

        assert!(issues.is_empty());
    }

    #[test]
    fn blueprint_secondary_persona_unknown_produces_warning() {
        let bp = ServiceDesignArtifact::Blueprint(Blueprint {
            meta: BlueprintMeta {
                title: "Test".into(),
                primary_persona: "Alex".into(),
                secondary_persona: Some("Unknown".into()),
                trigger: "trigger".into(),
                scope: "scope".into(),
                channels: vec![],
            },
            steps: vec![],
            dependency_map: vec![],
            design_decisions: vec![],
            open_questions: vec![],
        });

        let ctx = context_with_persona("Alex");
        let validator = CrossReferenceValidator;
        let issues = validator.validate(&bp, &ctx);

        let secondary_warnings: Vec<_> = issues
            .iter()
            .filter(|i| i.field == "meta.secondary_persona")
            .collect();
        assert_eq!(secondary_warnings.len(), 1);
    }

    #[test]
    fn journey_referencing_unknown_persona_produces_warning() {
        let jm = ServiceDesignArtifact::JourneyMap(JourneyMap {
            meta: JourneyMeta {
                title: "Test Journey".into(),
                persona: "Ghost".into(),
                scenario: "scenario".into(),
                goal: "goal".into(),
            },
            phases: vec![],
        });

        let validator = CrossReferenceValidator;
        let issues = validator.validate(&jm, &ValidationContext::default());

        let warnings: Vec<_> = issues
            .iter()
            .filter(|i| i.field == "meta.persona" && i.severity == Severity::Warning)
            .collect();
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn value_exchange_referencing_nonexistent_actor_produces_error() {
        let em = ServiceDesignArtifact::EcosystemMap(EcosystemMap {
            title: "Test Ecosystem".into(),
            core_service: "Service".into(),
            actors: vec![Actor {
                name: "Developer".into(),
                actor_type: ActorType::Primary,
                description: "A developer".into(),
            }],
            channels: vec![],
            value_exchanges: vec![ValueExchange {
                actor: "NonExistent".into(), // not in actors
                gives: vec!["data".into()],
                gets: vec!["value".into()],
            }],
            moments_of_truth: vec![],
            failure_modes: vec![],
        });

        let validator = CrossReferenceValidator;
        let issues = validator.validate(&em, &ValidationContext::default());

        let actor_errors: Vec<_> = issues
            .iter()
            .filter(|i| i.field.contains("value_exchanges") && i.severity == Severity::Error)
            .collect();
        assert_eq!(actor_errors.len(), 1);
    }

    #[test]
    fn value_exchange_referencing_existing_actor_produces_no_error() {
        let em = ServiceDesignArtifact::EcosystemMap(EcosystemMap {
            title: "Test Ecosystem".into(),
            core_service: "Service".into(),
            actors: vec![Actor {
                name: "Developer".into(),
                actor_type: ActorType::Primary,
                description: "A developer".into(),
            }],
            channels: vec![],
            value_exchanges: vec![ValueExchange {
                actor: "Developer".into(),
                gives: vec!["data".into()],
                gets: vec!["value".into()],
            }],
            moments_of_truth: vec![],
            failure_modes: vec![],
        });

        let validator = CrossReferenceValidator;
        let issues = validator.validate(&em, &ValidationContext::default());

        assert!(issues.is_empty());
    }
}
