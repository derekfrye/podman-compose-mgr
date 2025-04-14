use crate::secrets::error::Result;
use crate::secrets::models::UploadEntry;
use crate::secrets::utils::{calculate_hash, get_hostname};
use serde_json::{Value, from_str, to_string_pretty, to_value, to_writer_pretty};
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::Path;
use tempfile::NamedTempFile;

/// Read and parse the input JSON file
pub fn read_input_json(input_filepath: &Path) -> Result<Vec<Value>> {
    let mut file = File::open(input_filepath)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    // Parse JSON as array
    let entries: Vec<Value> = from_str(&content)?;

    Ok(entries)
}

/// Filter entries to only include those for the current hostname
pub fn filter_by_hostname(entries: &[Value], our_hostname: &str) -> Vec<Value> {
    entries
        .iter()
        .filter(|entry| {
            // Get hostname - convert to string for consistent comparison
            let entry_hostname = match entry["hostname"].as_str() {
                Some(h) => h,
                None => our_hostname, // Use current hostname if missing
            };

            // Only include entries for this host
            entry_hostname == our_hostname
        })
        .cloned()
        .collect()
}

/// Parse a JSON Value into an UploadEntry
pub fn parse_entry(entry: &Value) -> Result<UploadEntry> {
    // Parse the entry
    let upload_entry: UploadEntry = match serde_json::from_value(entry.clone()) {
        Ok(entry) => entry,
        Err(_) => {
            // If serde parsing fails, use manual extraction for backward compatibility
            let file_path = entry["file_nm"]
                .as_str()
                .or_else(|| entry["filenm"].as_str())
                .ok_or_else(|| {
                    Box::<dyn std::error::Error>::from(format!(
                        "Missing file_nm field in entry: {}",
                        entry
                    ))
                })?;

            let hash = entry["hash"]
                .as_str()
                .or_else(|| entry["md5"].as_str())
                .ok_or_else(|| {
                    Box::<dyn std::error::Error>::from(format!(
                        "Missing hash/md5 field in entry: {}",
                        entry
                    ))
                })?;

            let ins_ts = entry["ins_ts"].as_str().ok_or_else(|| {
                Box::<dyn std::error::Error>::from(format!(
                    "Missing ins_ts field in entry: {}",
                    entry
                ))
            })?;

            // Get optional fields with defaults
            let hash_algo = entry["hash_algo"].as_str().unwrap_or("sha1");
            let encoding = entry["encoding"].as_str().unwrap_or("utf8");
            let file_size = entry["file_size"].as_u64().unwrap_or(0);
            let encoded_size = entry["encoded_size"].as_u64().unwrap_or(file_size);
            let destination_cloud = entry["destination_cloud"].as_str().unwrap_or("azure_kv");

            // Get hostname - default to current hostname if missing
            let hostname = match entry["hostname"].as_str() {
                Some(h) => h.to_string(),
                None => get_hostname().unwrap_or_else(|_| "unknown_host".to_string()),
            };

            // Get cloud upload bucket if specified
            let cloud_upload_bucket = entry["cloud_upload_bucket"].as_str().map(String::from);
            let cloud_prefix = entry["cloud_prefix"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(String::from);

            // Create the entry manually
            UploadEntry {
                file_nm: file_path.to_string(),
                hash: hash.to_string(),
                ins_ts: ins_ts.to_string(),
                hostname,
                hash_algo: hash_algo.to_string(),
                encoding: encoding.to_string(),
                file_size,
                encoded_size,
                destination_cloud: destination_cloud.to_string(),
                cloud_upload_bucket,
                cloud_prefix,
            }
        }
    };

    Ok(upload_entry)
}

/// Save processed entries to an output JSON file
pub fn save_output_json(
    output_filepath: &Path,
    processed_entries: &[Value],
    verbose: i32,
) -> Result<()> {
    // Skip if no entries to process
    if processed_entries.is_empty() {
        if verbose > 0 {
            println!("info: No entries were processed successfully.");
        }
        return Ok(());
    }

    // Create parent directory if it doesn't exist
    if let Some(parent) = output_filepath.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            Box::<dyn std::error::Error>::from(format!(
                "Failed to create directory {}: {}",
                parent.display(),
                e
            ))
        })?;
    }

    // Check if the file already exists
    let file_exists = output_filepath.exists();

    if file_exists {
        // Read existing content to append properly
        let mut existing_file = File::open(output_filepath)?;
        let mut existing_content = String::new();
        existing_file.read_to_string(&mut existing_content)?;

        let mut existing_entries: Vec<Value> = if existing_content.trim().is_empty() {
            Vec::new()
        } else {
            from_str(&existing_content)?
        };

        // Append new entries
        existing_entries.extend(processed_entries.iter().cloned());

        // Write back as valid JSON array
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(output_filepath)?;

        to_writer_pretty(&mut file, &existing_entries)?;
    } else {
        // Create new file with JSON array
        let mut file = File::create(output_filepath)?;
        to_writer_pretty(&mut file, &processed_entries)?;
    }

    if verbose > 0 {
        println!(
            "info: Successfully saved {} entries to {}",
            processed_entries.len(),
            output_filepath.display()
        );
    }

    Ok(())
}

/// Create a test upload JSON file with R2 destination
pub fn create_r2_test_json(
    file_paths: &[String],
    bucket: &str,
) -> Result<(NamedTempFile, Vec<String>)> {
    // Create a temporary file
    let temp_file = NamedTempFile::new()?;

    // Create JSON entries for each test file
    let mut json_entries = Vec::new();
    let mut hashes = Vec::new();

    for file_path in file_paths {
        let hash = calculate_hash(file_path)?;
        hashes.push(hash.clone());

        let entry = UploadEntry::new_for_r2(file_path, &hash, bucket)
            .with_size_info(fs::metadata(file_path)?.len(), None);

        json_entries.push(to_value(entry)?);
    }

    // Write to the temporary file
    std::fs::write(temp_file.path(), to_string_pretty(&json_entries)?)?;

    Ok((temp_file, hashes))
}
