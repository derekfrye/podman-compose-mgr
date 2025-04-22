use thiserror::Error;

#[derive(Debug, Error)]
pub enum RebuildError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("YAML parsing error: {0}")]
    YamlParse(#[from] serde_yaml::Error),
    
    #[error("Path not found: {0}")]
    PathNotFound(String),
    
    #[error("Missing field in YAML: {0}")]
    MissingField(String),
    
    #[error("Invalid container configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Command execution error: {0}")]
    CommandExecution(String),
    
    #[error("Date parsing error: {0}")]
    DateParse(String),
    
    #[error("Error: {0}")]
    Other(String),
}