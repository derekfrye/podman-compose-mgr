use super::expansion::expand_tilde;
use serde_json::Value;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Checks if a file is readable
pub fn check_readable_file(file: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(file);
    let expanded_path = expand_tilde(&path)?;

    if expanded_path.is_file() && fs::metadata(&expanded_path).is_ok() {
        Ok(expanded_path)
    } else {
        Err(format!("The file '{file}' is not readable."))
    }
}

/// Checks if a file is readable (`PathBuf` version)
pub fn check_readable_path(file: &Path) -> Result<PathBuf, String> {
    if let Some(file_str) = file.to_str() {
        check_readable_file(file_str)
    } else {
        Err("Invalid path: contains non-UTF-8 characters".to_string())
    }
}

/// Checks if a file is a valid JSON file
pub fn check_valid_json_file(file: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(file);
    let expanded_path = expand_tilde(&path)?;

    let mut file_handle =
        File::open(&expanded_path).map_err(|e| format!("Unable to open '{file}': {e}"))?;
    let mut file_content = String::new();
    file_handle
        .read_to_string(&mut file_content)
        .map_err(|e| format!("Unable to read '{file}': {e}"))?;

    let mut entries = Vec::new();
    let deserializer = serde_json::Deserializer::from_str(&file_content).into_iter::<Value>();

    for entry in deserializer {
        let entry = entry.map_err(|e| format!("Invalid JSON in '{file}': {e}"))?;
        entries.push(entry);
    }
    Ok(expanded_path)
}

/// Checks if a `PathBuf` is a valid JSON file
pub fn check_valid_json_path(file: &Path) -> Result<PathBuf, String> {
    if let Some(file_str) = file.to_str() {
        check_valid_json_file(file_str)
    } else {
        Err("Invalid path: contains non-UTF-8 characters".to_string())
    }
}

/// Checks if a file is writable (or can be created and written to)
pub fn check_file_writable(file_path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(file_path);
    let expanded_path = expand_tilde(&path)?;

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

    match std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
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

/// Checks if a `PathBuf` is writable (or can be created and written to)
pub fn check_file_writable_path(file_path: &Path) -> Result<PathBuf, String> {
    if let Some(path_str) = file_path.to_str() {
        check_file_writable(path_str)
    } else {
        Err("Invalid path: contains non-UTF-8 characters".to_string())
    }
}
