use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceDesignError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Parse error in {file}: {message}")]
    Parse { file: String, message: String },

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Score out of range: {value} (expected {min}..={max})")]
    ScoreOutOfRange { value: i8, min: i8, max: i8 },
}
