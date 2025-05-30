use home::home_dir;
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Checks if a file is readable
///
/// # Arguments
///
/// * `file` - Path to check
///
/// # Returns
///
/// * `Result<PathBuf, String>` - The validated PathBuf or an error message
pub fn check_readable_file(file: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(file);

    let xpath = if path.starts_with("~") {
        if let Some(home) = home_dir() {
            home.join(path.strip_prefix("~").unwrap_or(path.as_path()))
        } else {
            return Err("Home directory could not be determined.".to_string());
        }
    } else {
        path
    };

    if xpath.is_file() && fs::metadata(&xpath).is_ok() {
        Ok(xpath)
    } else {
        Err(format!("The file '{}' is not readable.", file))
    }
}

/// Checks if a file is readable (PathBuf version)
///
/// # Arguments
///
/// * `file` - PathBuf to check
///
/// # Returns
///
/// * `Result<PathBuf, String>` - The validated PathBuf or an error message
pub fn check_readable_path(file: &Path) -> Result<PathBuf, String> {
    if let Some(file_str) = file.to_str() {
        check_readable_file(file_str)
    } else {
        Err("Invalid path: contains non-UTF-8 characters".to_string())
    }
}

/// Checks if a file is a valid JSON file
///
/// # Arguments
///
/// * `file` - Path to check
///
/// # Returns
///
/// * `Result<PathBuf, String>` - The validated PathBuf or an error message
pub fn check_valid_json_file(file: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(file);

    let mut file_handle =
        File::open(&path).map_err(|e| format!("Unable to open '{}': {}", file, e))?;
    let mut file_content = String::new();
    file_handle
        .read_to_string(&mut file_content)
        .map_err(|e| format!("Unable to read '{}': {}", file, e))?;

    let mut entries = Vec::new();
    let deserializer = serde_json::Deserializer::from_str(&file_content).into_iter::<Value>();

    for entry in deserializer {
        let entry = entry.map_err(|e| format!("Invalid JSON in '{}': {}", file, e))?;
        entries.push(entry);
    }
    Ok(path)
}

/// Checks if a PathBuf is a valid JSON file
///
/// # Arguments
///
/// * `file` - PathBuf to check
///
/// # Returns
///
/// * `Result<PathBuf, String>` - The validated PathBuf or an error message
pub fn check_valid_json_path(file: &Path) -> Result<PathBuf, String> {
    if let Some(file_str) = file.to_str() {
        check_valid_json_file(file_str)
    } else {
        Err("Invalid path: contains non-UTF-8 characters".to_string())
    }
}

/// Checks if a directory is readable
///
/// # Arguments
///
/// * `dir` - Path to check
///
/// # Returns
///
/// * `Result<PathBuf, String>` - The validated PathBuf or an error message
pub fn check_readable_dir(dir: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(dir);

    if path.is_dir() && fs::metadata(&path).is_ok() && fs::read_dir(&path).is_ok() {
        Ok(path)
    } else {
        Err(format!("The directory '{}' is not readable.", dir))
    }
}

/// Checks if a directory PathBuf is readable
///
/// # Arguments
///
/// * `dir` - PathBuf to check
///
/// # Returns
///
/// * `Result<PathBuf, String>` - The validated PathBuf or an error message
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
/// * `Result<PathBuf, String>` - The validated PathBuf or an error message
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

/// Checks if a file is writable (or can be created and written to)
///
/// # Arguments
///
/// * `file_path` - Path to check
///
/// # Returns
///
/// * `Result<PathBuf, String>` - The validated PathBuf or an error message
pub fn check_file_writable(file_path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(file_path);

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

    // First check if the parent directory exists and is writable
    if let Some(parent) = expanded_path.parent() {
        if !parent.exists() {
            return Err(format!(
                "The parent directory of '{}' does not exist.",
                expanded_path.display()
            ));
        }

        if !parent.is_dir() {
            return Err(format!(
                "The parent path '{}' is not a directory.",
                parent.display()
            ));
        }
    }

    // Try to open the file in write mode
    match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false) // Don't truncate an existing file
        .open(&expanded_path)
    {
        Ok(_) => Ok(expanded_path),
        Err(e) => Err(format!(
            "The file '{}' is not writable: {}",
            expanded_path.display(),
            e
        )),
    }
}

/// Checks if a PathBuf is writable (or can be created and written to)
///
/// # Arguments
///
/// * `file_path` - PathBuf to check
///
/// # Returns
///
/// * `Result<PathBuf, String>` - The validated PathBuf or an error message
pub fn check_file_writable_path(file_path: &Path) -> Result<PathBuf, String> {
    if let Some(path_str) = file_path.to_str() {
        check_file_writable(path_str)
    } else {
        Err("Invalid path: contains non-UTF-8 characters".to_string())
    }
}
