pub mod error;
pub mod model;
pub mod parser;
pub mod validation;

pub use error::ServiceDesignError;
pub use model::{
    Actor, ActorType, Blueprint, BlueprintMeta, BlueprintStep, Channel, Dependency, EcosystemMap,
    EmotionalScore, ExpertiseTier, FailureMode, JourneyMap, JourneyMeta, JourneyPhase,
    MomentOfTruth, PainPoint, PainPointMatrix, PainTheme, Persona, PersonaOverview, Probe,
    ScoringDimension, ScoringRubric, ServiceDesignArtifact, ValueExchange,
};
pub use parser::{ArtifactParser, ArtifactParserRegistry, Frontmatter};
pub use validation::{
    ArtifactValidator, ArtifactValidatorPipeline, Severity, ValidationContext, ValidationIssue,
    ValidationResult,
};
