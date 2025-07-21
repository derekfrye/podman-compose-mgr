pub mod datetime;
pub mod image;
pub mod terminal;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PodmanHelperError {
    #[error("Command execution error: {0}")]
    CommandExecution(String),

    #[error("Output parsing error: {0}")]
    OutputParsing(String),

    #[error("Date parsing error: {0}")]
    DateParsing(String),

    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),

    #[error("Environment variable error: {0}")]
    EnvVarError(#[from] std::env::VarError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

pub type Result<T> = std::result::Result<T, PodmanHelperError>;

// For compatibility with Box<dyn std::error::Error>
impl From<Box<dyn std::error::Error>> for PodmanHelperError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        PodmanHelperError::CommandExecution(format!("{err}"))
    }
}

// Re-export commonly used functions
pub use datetime::convert_str_to_date;
pub use image::{
    get_podman_image_upstream_create_time, get_podman_ondisk_modify_time, pull_base_image,
};
pub use terminal::{file_exists_and_readable, get_terminal_display_width};
