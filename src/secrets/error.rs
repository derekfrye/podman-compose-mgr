use thiserror::Error;

#[derive(Debug, Error)]
pub enum SecretError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Azure Key Vault error: {0}")]
    KeyVault(String),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("Runtime error: {0}")]
    Runtime(String),

    #[error("Field missing in JSON: {0}")]
    MissingField(String),

    #[error("Time parse error: {0}")]
    TimeError(String),

    #[error("Hostname error: {0}")]
    HostnameError(String),

    #[error("Path error: {0}")]
    PathError(String),

    #[error("Url parse error: {0}")]
    UrlError(String),

    #[error("Generic error: {0}")]
    Other(String),
}

// Use Box<dyn Error> for public functions for backward compatibility
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
