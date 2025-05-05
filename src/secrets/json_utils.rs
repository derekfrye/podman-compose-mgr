use crate::args::Args; // Add Args import
use crate::secrets::error::Result;
use crate::secrets::models::{JsonOutput, UploadEntry};
use crate::secrets::utils::{calculate_hash, get_hostname};
use serde_json::{Value, from_str, to_string_pretty, to_value, to_writer_pretty};
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Read, Write}; // Added Write for flush()
use std::path::Path;
use tempfile::{Builder as TempFileBuilder, NamedTempFile}; // Add builder

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
    args: &Args, // Add args parameter
) -> Result<(NamedTempFile, Vec<String>)> {
    // Create a temporary file in the specified directory
    let temp_file = TempFileBuilder::new()
        .prefix("r2_test_")
        .suffix(".json")
        .tempfile_in(&args.temp_file_path)?;

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

/// Writes the JSON output entries to a temporary file.
///
/// Returns the PathBuf of the temporary file.
fn write_json_output_to_temp_file(
    output_entries: &[JsonOutput],
    args: &Args, // Add args parameter
) -> Result<NamedTempFile> {
    // Create a temporary file in the specified directory
    let temp_file = TempFileBuilder::new()
        .prefix("output_")
        .suffix(".json")
        .tempfile_in(&args.temp_file_path) // Use args path
        .map_err(|e| {
            Box::<dyn std::error::Error>::from(format!(
                "Failed to create temporary JSON file in {}: {}",
                args.temp_file_path.display(),
                e
            ))
        })?;

    let file_handle = temp_file.reopen()?; // Reopen to get a File handle for BufWriter
    let mut writer = BufWriter::new(file_handle);

    // Serialize the output entries to the temporary file
    serde_json::to_writer_pretty(&mut writer, output_entries).map_err(|e| {
        Box::<dyn std::error::Error>::from(format!("Failed to write JSON to temporary file: {}", e))
    })?;

    writer.flush().map_err(|e| {
        Box::<dyn std::error::Error>::from(format!("Failed to flush writer: {}", e))
    })?;

    Ok(temp_file) // Return the NamedTempFile handle
}

/// Write the final JSON output, merging with existing if necessary.
pub fn write_json_output(
    output_entries: Vec<JsonOutput>,
    output_path: &Path,
    args: &Args,        // Add args parameter
    sort_entries: bool, // Whether to sort entries (true for SecretUpload mode)
) -> Result<()> {
    // Check if the file already exists
    let mut final_entries;

    if output_path.exists() {
        // Read existing content to merge
        let mut file = File::open(output_path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        // Try to parse existing entries
        if !content.trim().is_empty() {
            let existing_entries: Vec<JsonOutput> =
                serde_json::from_str(&content).map_err(|e| {
                    Box::<dyn std::error::Error>::from(format!("Failed to parse JSON: {}", e))
                })?;

            // Merge entries by file_nm, keeping the newer entry in case of duplicates
            let mut entries_map = std::collections::HashMap::new();

            // First add existing entries
            for entry in existing_entries {
                entries_map.insert(entry.file_nm.clone(), entry);
            }

            // Then add new entries, overwriting any duplicates
            for entry in &output_entries {
                entries_map.insert(entry.file_nm.clone(), entry.clone());
            }

            // Convert back to Vec
            final_entries = entries_map.into_values().collect();
        } else {
            // Empty file, just use new entries
            final_entries = output_entries.clone();
        }
    } else {
        // No existing file, just use new entries
        final_entries = output_entries;
    }

    // Sort the entries by hostname (ascending) and then by hash if requested
    if sort_entries {
        final_entries.sort_by(|a, b| {
            // First sort by hostname (ascending)
            let hostname_cmp = a.hostname.cmp(&b.hostname);
            if hostname_cmp != std::cmp::Ordering::Equal {
                return hostname_cmp;
            }

            // If hostnames are equal, sort by hash
            let a_hash = if !a.hash_val.is_empty() {
                &a.hash_val
            } else {
                &a.md5
            };
            let b_hash = if !b.hash_val.is_empty() {
                &b.hash_val
            } else {
                &b.md5
            };
            a_hash.cmp(b_hash)
        });
    }

    // Write new/updated entries to a temporary file first
    let temp_output_file = write_json_output_to_temp_file(&final_entries, args)?; // Pass args

    // Try atomic rename first (which is faster)
    if let Err(e) = fs::rename(temp_output_file.path(), output_path) {
        // If rename fails with cross-device error, fall back to copy and remove
        if e.kind() == std::io::ErrorKind::CrossesDevices {
            // Copy the content first
            let temp_content = fs::read(temp_output_file.path())?;
            // Make sure parent directory exists
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }
            // Write the content to destination
            fs::write(output_path, temp_content)?;
        } else {
            // For other errors, return the error
            return Err(Box::<dyn std::error::Error>::from(format!(
                "Failed to replace output file '{}': {}",
                output_path.display(),
                e
            )));
        }
    }

    Ok(())
}
