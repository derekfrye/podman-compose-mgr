use super::expansion::expand_tilde;
use std::fs;
use std::path::{Path, PathBuf};

/// Checks if a directory is readable.
///
/// # Errors
///
/// Returns an error if the directory does not exist, cannot be read, or metadata lookups fail.
pub fn check_readable_dir(dir: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(dir);
    let expanded_path = expand_tilde(&path)?;

    if expanded_path.is_dir()
        && fs::metadata(&expanded_path).is_ok()
        && fs::read_dir(&expanded_path).is_ok()
    {
        Ok(expanded_path)
    } else {
        Err(format!("The directory '{dir}' is not readable."))
    }
}

/// Checks if a directory `PathBuf` is readable.
///
/// # Errors
///
/// Returns an error if the path is not valid UTF-8 or the directory cannot be read.
pub fn check_readable_dir_path(dir: &Path) -> Result<PathBuf, String> {
    if let Some(dir_str) = dir.to_str() {
        check_readable_dir(dir_str)
    } else {
        Err("Invalid path: contains non-UTF-8 characters".to_string())
    }
}

/// Checks if a directory is writable, creating it if it doesn't exist.
///
/// # Errors
///
/// Returns an error if the directory cannot be created, does not resolve to a directory,
/// or a test write fails.
pub fn check_writable_dir(dir: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(dir);
    let expanded_path = expand_tilde(&path)?;

    if !expanded_path.exists() {
        fs::create_dir_all(&expanded_path).map_err(|e| {
            format!(
                "Failed to create directory '{}': {}",
                expanded_path.display(),
                e
            )
        })?;
    }

    if !expanded_path.is_dir() {
        return Err(format!("'{}' is not a directory.", expanded_path.display()));
    }

    match tempfile::tempfile_in(&expanded_path) {
        Ok(_) => Ok(expanded_path),
        Err(e) => Err(format!(
            "Directory '{}' is not writable: {}",
            expanded_path.display(),
            e
        )),
    }
}
