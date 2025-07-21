use home::home_dir;
use std::fs;
use std::path::{Path, PathBuf};

/// Checks if a directory is readable
///
/// # Arguments
///
/// * `dir` - Path to check
///
/// # Returns
///
/// * `Result<PathBuf, String>` - The validated `PathBuf` or an error message
///
/// # Errors
///
/// Returns an error if the directory is not readable.
pub fn check_readable_dir(dir: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(dir);

    if path.is_dir() && fs::metadata(&path).is_ok() && fs::read_dir(&path).is_ok() {
        Ok(path)
    } else {
        Err(format!("The directory '{dir}' is not readable."))
    }
}

/// Checks if a directory `PathBuf` is readable
///
/// # Arguments
///
/// * `dir` - `PathBuf` to check
///
/// # Returns
///
/// * `Result<PathBuf, String>` - The validated `PathBuf` or an error message
///
/// # Errors
///
/// Returns an error if the directory is not readable or contains non-UTF-8 characters.
pub fn check_readable_dir_path(dir: &Path) -> Result<PathBuf, String> {
    if let Some(dir_str) = dir.to_str() {
        check_readable_dir(dir_str)
    } else {
        Err("Invalid path: contains non-UTF-8 characters".to_string())
    }
}

/// Checks if a directory is writable, creating it if it doesn't exist.
///
/// # Arguments
///
/// * `dir` - Path to check
///
/// # Returns
///
/// * `Result<PathBuf, String>` - The validated `PathBuf` or an error message
///
/// # Errors
///
/// Returns an error if the directory cannot be created or is not writable.
pub fn check_writable_dir(dir: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(dir);

    // Resolve ~ if present
    let expanded_path = if path.starts_with("~") {
        if let Some(home) = home_dir() {
            home.join(path.strip_prefix("~").unwrap_or(path.as_path()))
        } else {
            return Err("Home directory could not be determined.".to_string());
        }
    } else {
        path
    };

    // Create the directory if it doesn't exist
    if !expanded_path.exists() {
        fs::create_dir_all(&expanded_path).map_err(|e| {
            format!(
                "Failed to create directory '{}': {}",
                expanded_path.display(),
                e
            )
        })?;
    }

    // Check if it's a directory
    if !expanded_path.is_dir() {
        return Err(format!("'{}' is not a directory.", expanded_path.display()));
    }

    // Check if it's writable by trying to create a temporary file inside it
    match tempfile::tempfile_in(&expanded_path) {
        Ok(_) => Ok(expanded_path), // Successfully created and implicitly deleted a temp file
        Err(e) => Err(format!(
            "Directory '{}' is not writable: {}",
            expanded_path.display(),
            e
        )),
    }
}
