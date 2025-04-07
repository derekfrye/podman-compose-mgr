use chrono::Utc;
use regex::Regex;
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

            // Process lines with cloud section detection
            let file_array = process_input_lines(lines)?;

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

/// Process input lines and detect cloud provider sections
///
/// This function:
/// 1. Processes each line in the input file
/// 2. Detects cloud provider section headers (e.g. #AZURE#)
/// 3. Associates files with the appropriate cloud provider
/// 4. Creates a JSON array with filenames and their cloud providers
fn process_input_lines(lines: Vec<&str>) -> Result<Vec<serde_json::Value>, String> {
    let cloud_section_regex =
        Regex::new(r"^\s*#{1,}\s*([Aa][Zz][Uu][Rr][Ee]|[Rr]2|[Bb]2)\s*#{1,}\s*$")
            .map_err(|e| format!("Failed to compile regex: {}", e))?;

    let bucket_regex =
        Regex::new(r"^\s*#{1,}\s*[Bb][Uu][Cc][Kk][Ee][Tt]\s+([a-zA-Z0-9_-]+)\s*#{1,}\s*$")
            .map_err(|e| format!("Failed to compile bucket regex: {}", e))?;

    let mut current_cloud = "azure_kv"; // Default to azure_kv
    let mut current_bucket = String::new(); // Empty default bucket
    let mut file_array = Vec::new();

    for line in lines {
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Check if this is a cloud section header
        if let Some(captures) = cloud_section_regex.captures(trimmed) {
            if let Some(cloud_match) = captures.get(1) {
                let cloud_name = cloud_match.as_str().to_lowercase();
                current_cloud = match cloud_name.as_str() {
                    "azure" => "azure_kv",
                    "r2" => "r2",
                    "b2" => "b2",
                    _ => "azure_kv", // Default to azure_kv for unknown
                };
            }
            continue;
        }

        // Check if this is a bucket specification
        if let Some(captures) = bucket_regex.captures(trimmed) {
            if let Some(bucket_match) = captures.get(1) {
                current_bucket = bucket_match.as_str().to_string();
            }
            continue;
        }

        // Skip other commented lines
        if trimmed.starts_with('#') {
            continue;
        }

        // Process the file
        match check_readable_file(trimmed) {
            Ok(path) => {
                if let Some(path_str) = path.to_str() {
                    // Create the JSON entry
                    let mut entry = serde_json::json!({
                        "filenm": path_str,
                        "destination_cloud": current_cloud
                    });

                    // Add bucket name only for r2 or b2 cloud types
                    if (current_cloud == "r2" || current_cloud == "b2")
                        && !current_bucket.is_empty()
                    {
                        if let Some(obj) = entry.as_object_mut() {
                            obj.insert(
                                "cloud_upload_bucket".to_string(),
                                serde_json::Value::String(current_bucket.clone()),
                            );
                        }
                    }

                    file_array.push(entry);
                } else {
                    eprintln!(
                        "Warning: Path '{}' contains invalid UTF-8 characters, skipping",
                        trimmed
                    );
                }
            }
            Err(_) => {
                eprintln!(
                    "Warning: File '{}' does not exist or is not readable.",
                    trimmed
                );
            }
        }
    }

    if file_array.is_empty() {
        return Err("No readable files found in the input.".to_string());
    }

    Ok(file_array)
}
