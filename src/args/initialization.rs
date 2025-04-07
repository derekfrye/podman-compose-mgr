use chrono::Utc;
use serde_json;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use super::validators::check_readable_file;

/// Checks if a file is valid for initialization
/// It must be readable and either valid JSON or contain a list of filenames
///
/// When processing a file containing a list of filenames, this function:
/// 1. Reads the input file
/// 2. Validates each filename
/// 3. Creates a JSON array with valid filenames
/// 4. Writes the JSON to a new file with timestamp extension
/// 5. Returns the path to the new JSON file
///
/// # Arguments
///
/// * `file_path` - Path to the file to check
///
/// # Returns
///
/// * `Result<PathBuf, String>` - The validated or generated new PathBuf
pub fn check_init_filepath(file_path: &str) -> Result<PathBuf, String> {
    // First check if the file is readable
    let path = check_readable_file(file_path)?;

    // Try to parse as JSON
    let mut file = File::open(&path).map_err(|e| e.to_string())?;
    let mut file_content = String::new();
    file.read_to_string(&mut file_content)
        .map_err(|e| e.to_string())?;

    // If empty file, return error
    if file_content.trim().is_empty() {
        return Err(format!("The file '{}' is empty.", file_path));
    }

    // Try to parse as JSON first
    let json_result = serde_json::from_str::<serde_json::Value>(&file_content);

    match json_result {
        Ok(_) => {
            // Valid JSON, no further processing needed
            Ok(path)
        }
        Err(_) => {
            // Not valid JSON, check if it's a list of filenames (one per line)
            let lines: Vec<&str> = file_content.lines().collect();

            // Filter out empty lines and commented lines
            let non_empty_lines: Vec<&str> = lines
                .iter()
                .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
                .copied()
                .collect();

            if non_empty_lines.is_empty() {
                return Err(format!(
                    "The file '{}' does not contain valid JSON or filenames.",
                    file_path
                ));
            }

            // Check if each filename is a readable file
            let mut all_valid = true;
            for line in &non_empty_lines {
                let filename = line.trim();
                if check_readable_file(filename).is_err() {
                    eprintln!(
                        "Warning: File '{}' does not exist or is not readable.",
                        filename
                    );
                    all_valid = false;
                }
            }

            if !all_valid {
                eprintln!(
                    "Warning: Some files are not readable. Continuing with valid files only."
                );
            }

            // Create JSON array with filenames (only for files that exist and are readable)
            let mut file_array = Vec::new();
            for line in non_empty_lines {
                let filename = line.trim();
                match check_readable_file(filename) {
                    Ok(path) => {
                        if let Some(path_str) = path.to_str() {
                            file_array.push(serde_json::json!({"filenm": path_str}));
                        } else {
                            eprintln!(
                                "Warning: Path '{}' contains invalid UTF-8 characters, skipping",
                                filename
                            );
                        }
                    }
                    Err(_) => continue, // Skip invalid files
                }
            }

            // If no valid files found, return an error
            if file_array.is_empty() {
                return Err(format!("No readable files found in '{}'.", file_path));
            }

            let json_content = serde_json::to_string_pretty(&file_array)
                .map_err(|e| format!("Failed to convert filename list to JSON: {}", e))?;

            // Write back to the file with unix timestamp and .json extension
            let new_extension = format!(".{}.json", Utc::now().timestamp());
            let new_file_path = path.with_extension(new_extension);
            let mut output_file = File::create(&new_file_path)
                .map_err(|e| format!("Failed to open file for writing: {}", e))?;

            output_file
                .write_all(json_content.as_bytes())
                .map_err(|e| format!("Failed to write JSON content: {}", e))?;

            Ok(new_file_path)
        }
    }
}
