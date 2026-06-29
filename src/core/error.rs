use thiserror::Error;

/// Custom error types for the launcher application
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON Serialization/Deserialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Process management error: {0}")]
    Process(String),

    #[error("Mojang API error: {0}")]
    Manifest(String),

    #[error("Download error: {0}")]
    Download(String),

    #[error("General launcher error: {0}")]
    Other(String),
}
