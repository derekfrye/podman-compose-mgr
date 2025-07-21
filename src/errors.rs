use thiserror::Error;

#[derive(Debug, Error)]
pub enum PodmanComposeMgrError {
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),

    #[error("Path contains invalid UTF-8: {0}")]
    InvalidPath(String),

    #[error("Rebuild error: {0}")]
    RebuildError(String),

    #[error("Command execution error: {0}")]
    CommandExecution(#[from] Box<dyn std::error::Error>),

    #[error("Buildfile error: {0}")]
    Buildfile(#[from] crate::image_build::buildfile::BuildfileError),
}
