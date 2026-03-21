use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::ServiceDesignError;

// -- Shared types --

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmotionalScore(i8);

impl EmotionalScore {
    pub fn new(value: i8) -> Result<Self, ServiceDesignError> {
        if (-2..=2).contains(&value) {
            Ok(Self(value))
        } else {
            Err(ServiceDesignError::ScoreOutOfRange {
                value,
                min: -2,
                max: 2,
            })
        }
    }

    pub fn value(&self) -> i8 {
        self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PainPoint {
    pub label: String,
    pub problem: String,
    #[serde(default)]
    pub workaround: Option<String>,
    #[serde(default)]
    pub cost: Option<String>,
    #[serde(default)]
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MomentOfTruth {
    pub moment: String,
    pub success_state: String,
    pub failure_state: String,
    #[serde(default)]
    pub why_it_matters: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActorType {
    Primary,
    Secondary,
    Tertiary,
    Future,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub name: String,
    pub actor_type: ActorType,
    pub description: String,
}

// -- Blueprint --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blueprint {
    pub meta: BlueprintMeta,
    pub steps: Vec<BlueprintStep>,
    #[serde(default)]
    pub dependency_map: Vec<Dependency>,
    #[serde(default)]
    pub design_decisions: Vec<String>,
    #[serde(default)]
    pub open_questions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintMeta {
    pub title: String,
    pub primary_persona: String,
    #[serde(default)]
    pub secondary_persona: Option<String>,
    pub trigger: String,
    pub scope: String,
    #[serde(default)]
    pub channels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintStep {
    pub name: String,
    pub evidence: String,
    pub customer_actions: String,
    pub frontstage: String,
    pub backstage: String,
    pub support_processes: String,
    #[serde(default)]
    pub pain_points: Vec<PainPoint>,
    pub emotional_state: EmotionalScore,
    pub emotional_label: String,
    #[serde(default)]
    pub metrics: Vec<String>,
    #[serde(default)]
    pub moments_of_truth: Vec<MomentOfTruth>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub description: Option<String>,
}

// -- Journey Map --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JourneyMap {
    pub meta: JourneyMeta,
    pub phases: Vec<JourneyPhase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JourneyMeta {
    pub title: String,
    pub persona: String,
    pub scenario: String,
    pub goal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JourneyPhase {
    pub name: String,
    #[serde(default)]
    pub time_range: Option<String>,
    #[serde(default)]
    pub actions: Vec<String>,
    #[serde(default)]
    pub thoughts: Vec<String>,
    pub emotions: String,
    #[serde(default)]
    pub touchpoints: Vec<String>,
    #[serde(default)]
    pub pain_points: Vec<String>,
    #[serde(default)]
    pub opportunities: Vec<String>,
    pub emotional_score: EmotionalScore,
}

// -- Ecosystem Map --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemMap {
    pub title: String,
    pub core_service: String,
    #[serde(default)]
    pub actors: Vec<Actor>,
    #[serde(default)]
    pub channels: Vec<Channel>,
    #[serde(default)]
    pub value_exchanges: Vec<ValueExchange>,
    #[serde(default)]
    pub moments_of_truth: Vec<MomentOfTruth>,
    #[serde(default)]
    pub failure_modes: Vec<FailureMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub name: String,
    pub channel_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueExchange {
    pub actor: String,
    #[serde(default)]
    pub gives: Vec<String>,
    #[serde(default)]
    pub gets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureMode {
    pub mode: String,
    pub impact: String,
    pub recovery: String,
}

// -- Persona --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persona {
    pub overview: PersonaOverview,
    pub background: String,
    #[serde(default)]
    pub goals: Vec<String>,
    #[serde(default)]
    pub pain_points: Vec<PainPoint>,
    #[serde(default)]
    pub current_tools: Vec<String>,
    #[serde(default)]
    pub behavioral_patterns: Vec<String>,
    #[serde(default)]
    pub technology_expertise: Vec<ExpertiseTier>,
    #[serde(default)]
    pub success_metrics: Vec<String>,
    #[serde(default)]
    pub quotes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaOverview {
    pub name: String,
    #[serde(default)]
    pub age: Option<String>,
    pub role: String,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub experience: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertiseTier {
    pub tier: String,
    #[serde(default)]
    pub skills: Vec<String>,
}

// -- Pain Point Matrix --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PainPointMatrix {
    pub rubric: ScoringRubric,
    #[serde(default)]
    pub themes: Vec<PainTheme>,
    #[serde(default)]
    pub ranked_priorities: Vec<String>,
    #[serde(default)]
    pub disconfirmation_log: Vec<String>,
    #[serde(default)]
    pub probe_backlog: Vec<Probe>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringRubric {
    #[serde(default)]
    pub dimensions: Vec<ScoringDimension>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringDimension {
    pub name: String,
    pub weight: f32,
    pub scale_max: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PainTheme {
    pub name: String,
    #[serde(default)]
    pub scores: HashMap<String, f32>,
    pub composite_score: f32,
    pub evidence: String,
    #[serde(default)]
    pub monthly_cost: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Probe {
    pub question: String,
    pub method: String,
    pub success_metric: String,
}

// -- Top-level artifact enum --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceDesignArtifact {
    Blueprint(Blueprint),
    JourneyMap(JourneyMap),
    EcosystemMap(EcosystemMap),
    Persona(Persona),
    PainPointMatrix(PainPointMatrix),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emotional_score_valid_range() {
        assert!(EmotionalScore::new(0).is_ok());
        assert!(EmotionalScore::new(-2).is_ok());
        assert!(EmotionalScore::new(2).is_ok());
        assert!(EmotionalScore::new(-1).is_ok());
        assert!(EmotionalScore::new(1).is_ok());
    }

    #[test]
    fn emotional_score_rejects_out_of_range() {
        assert!(EmotionalScore::new(3).is_err());
        assert!(EmotionalScore::new(-3).is_err());
        assert!(EmotionalScore::new(127).is_err());
        assert!(EmotionalScore::new(-128).is_err());
    }

    #[test]
    fn emotional_score_value_accessor() {
        let score = EmotionalScore::new(1).unwrap();
        assert_eq!(score.value(), 1);
    }

    #[test]
    fn blueprint_step_roundtrip_serde() {
        let step = BlueprintStep {
            name: "First Encounter".into(),
            evidence: "README, install output".into(),
            customer_actions: "Runs npx rafters init".into(),
            frontstage: "CLI responds with config".into(),
            backstage: "Detects framework, generates config".into(),
            support_processes: "Node.js, package manager".into(),
            pain_points: vec![],
            emotional_state: EmotionalScore::new(2).unwrap(),
            emotional_label: "Curiosity".into(),
            metrics: vec!["Time to first output < 30s".into()],
            moments_of_truth: vec![],
        };
        let json = serde_json::to_string(&step).unwrap();
        let roundtrip: BlueprintStep = serde_json::from_str(&json).unwrap();
        assert_eq!(step.name, roundtrip.name);
        assert_eq!(step.emotional_state, roundtrip.emotional_state);
    }

    #[test]
    fn service_design_artifact_enum_dispatches() {
        let bp = ServiceDesignArtifact::Blueprint(Blueprint {
            meta: BlueprintMeta {
                title: "Test Blueprint".into(),
                primary_persona: "Developer".into(),
                secondary_persona: None,
                trigger: "Discovers rafters".into(),
                scope: "First run to production".into(),
                channels: vec!["CLI".into()],
            },
            steps: vec![],
            dependency_map: vec![],
            design_decisions: vec![],
            open_questions: vec![],
        });
        assert!(matches!(bp, ServiceDesignArtifact::Blueprint(_)));
    }

    #[test]
    fn pain_point_optional_fields_default() {
        let json = r#"{"label":"test","problem":"something breaks"}"#;
        let pp: PainPoint = serde_json::from_str(json).unwrap();
        assert_eq!(pp.label, "test");
        assert!(pp.workaround.is_none());
        assert!(pp.cost.is_none());
        assert!(pp.evidence.is_none());
    }

    #[test]
    fn moment_of_truth_roundtrip() {
        let mot = MomentOfTruth {
            moment: "First output".into(),
            success_state: "User sees working component".into(),
            failure_state: "Blank page, no error".into(),
            why_it_matters: Some("Determines whether user continues".into()),
        };
        let json = serde_json::to_string(&mot).unwrap();
        let roundtrip: MomentOfTruth = serde_json::from_str(&json).unwrap();
        assert_eq!(mot.moment, roundtrip.moment);
        assert_eq!(mot.why_it_matters, roundtrip.why_it_matters);
    }

    #[test]
    fn actor_type_equality() {
        assert_eq!(ActorType::Primary, ActorType::Primary);
        assert_ne!(ActorType::Primary, ActorType::Secondary);
    }

    #[test]
    fn journey_phase_roundtrip() {
        let phase = JourneyPhase {
            name: "Discovery".into(),
            time_range: Some("Day 1-3".into()),
            actions: vec!["Searches for tools".into()],
            thoughts: vec!["Is this worth trying?".into()],
            emotions: "Curious but skeptical".into(),
            touchpoints: vec!["GitHub".into(), "Blog".into()],
            pain_points: vec![],
            opportunities: vec!["Clear value prop".into()],
            emotional_score: EmotionalScore::new(0).unwrap(),
        };
        let json = serde_json::to_string(&phase).unwrap();
        let roundtrip: JourneyPhase = serde_json::from_str(&json).unwrap();
        assert_eq!(phase.name, roundtrip.name);
        assert_eq!(phase.emotional_score, roundtrip.emotional_score);
    }

    #[test]
    fn ecosystem_map_with_failure_modes() {
        let map = EcosystemMap {
            title: "Rafters Ecosystem".into(),
            core_service: "Design token pipeline".into(),
            actors: vec![Actor {
                name: "Developer".into(),
                actor_type: ActorType::Primary,
                description: "Builds with rafters".into(),
            }],
            channels: vec![],
            value_exchanges: vec![],
            moments_of_truth: vec![],
            failure_modes: vec![FailureMode {
                mode: "Token sync fails silently".into(),
                impact: "Stale design values in production".into(),
                recovery: "Manual re-sync via CLI".into(),
            }],
        };
        let json = serde_json::to_string(&map).unwrap();
        let roundtrip: EcosystemMap = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.failure_modes.len(), 1);
    }

    #[test]
    fn persona_minimal_fields() {
        let json = r#"{
            "overview": {"name": "Alex", "role": "Frontend Dev"},
            "background": "5 years of React"
        }"#;
        let persona: Persona = serde_json::from_str(json).unwrap();
        assert_eq!(persona.overview.name, "Alex");
        assert!(persona.goals.is_empty());
        assert!(persona.quotes.is_empty());
    }

    #[test]
    fn pain_point_matrix_scoring() {
        let matrix = PainPointMatrix {
            rubric: ScoringRubric {
                dimensions: vec![ScoringDimension {
                    name: "Frequency".into(),
                    weight: 0.4,
                    scale_max: 5,
                }],
            },
            themes: vec![PainTheme {
                name: "Config complexity".into(),
                scores: HashMap::from([("Frequency".into(), 4.0)]),
                composite_score: 4.0,
                evidence: "3 of 5 users mentioned this".into(),
                monthly_cost: Some("$2000 in support tickets".into()),
            }],
            ranked_priorities: vec!["Config complexity".into()],
            disconfirmation_log: vec![],
            probe_backlog: vec![],
        };
        assert_eq!(matrix.themes[0].composite_score, 4.0);
    }
}
