pub mod datetime;
pub mod image;
pub mod terminal;

use std::ffi::OsString;
use std::sync::{Mutex, OnceLock};
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
pub use datetime::{convert_str_to_date, format_time_ago};
pub use image::{
    get_podman_image_upstream_create_time, get_podman_ondisk_modify_time, pull_base_image,
};
pub use terminal::{file_exists_and_readable, get_terminal_display_width};

/// Resolve the executable name used for invoking podman commands.
///
/// This allows tests to inject a mock implementation by setting the
/// override via the CLI or helper functions.
#[must_use]
pub fn resolve_podman_binary() -> OsString {
    let mutex = PODMAN_BIN_OVERRIDE.get_or_init(|| Mutex::new(None));
    let guard = mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let override_path = guard.clone();

    override_path.unwrap_or_else(|| OsString::from("podman"))
}

/// Override the podman executable path for the current process.
pub fn set_podman_binary_override(path: OsString) {
    let mutex = PODMAN_BIN_OVERRIDE.get_or_init(|| Mutex::new(None));
    let mut guard = mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard = Some(path);
}

/// Clear any configured podman executable override.
pub fn clear_podman_binary_override() {
    let mutex = PODMAN_BIN_OVERRIDE.get_or_init(|| Mutex::new(None));
    let mut guard = mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard = None;
}

static PODMAN_BIN_OVERRIDE: OnceLock<Mutex<Option<OsString>>> = OnceLock::new();
