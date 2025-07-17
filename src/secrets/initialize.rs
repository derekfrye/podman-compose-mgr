use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

use chrono::{DateTime, Local};
use serde_json::json;

use crate::secrets::file_details::check_encoding_and_size;
// Removed create_secret_name import as it's no longer used

use crate::Args;
use crate::secrets::error::Result;
use crate::utils::log_utils::Logger;

/// Process the initialization of secrets
///
/// Reads the secrets from the input file, creates JSON entries for each file,
/// and writes the results to the output_json file specified in the arguments.
pub fn process(args: &Args, _logger: &Logger) -> Result<()> {
    // Get the required file paths from args
    let init_filepath = args.secrets_init_filepath.as_ref().unwrap();
    let output_filepath = args.output_json.as_ref().unwrap();

    // Read the input file containing the file paths
    let mut file = File::open(init_filepath)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    // Parse the JSON - it's either a map or an array of objects with filenm key
    let files_array: Vec<serde_json::Value> = serde_json::from_str(&contents).unwrap_or_default();

    // Instead of just collecting filenames, we'll create a mapping of each file to its entry
    // This way we can handle multiple entries for the same file with different cloud providers
    let file_entries: Vec<serde_json::Value> = if contents.trim().starts_with('{') {
        // Handle map format
        let files_map: HashMap<String, String> = serde_json::from_str(&contents)?;
        // Convert each entry to a JSON object with filenm field
        files_map
            .values()
            .map(|filename| {
                json!({
                    "filenm": filename
                })
            })
            .collect()
    } else {
        // If it's already an array format, just use it directly
        files_array.clone()
    };

    // Get the hostname
    let hostname = hostname()?;

    // Create new entries for each file entry in the input
    let mut new_entries = Vec::new();

    for entry_json in &file_entries {
        // Get the filename from the entry
        let file_nm = match entry_json.get("filenm").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => continue, // Skip entries without a filename
        };

        // Check if file exists
        if !Path::new(file_nm).exists() {
            return Err(Box::<dyn std::error::Error>::from(format!(
                "File {} does not exist",
                file_nm
            )));
        }

        // Calculate SHA-1 hash
        let hash = calculate_hash(file_nm)?;

        // Get current timestamp
        let now: DateTime<Local> = Local::now();
        let ins_ts = now.to_rfc3339();

        // Check encoding and get file sizes (creates base64 encoded file if needed)
        let (_encoding, _file_size, encoded_size) = check_encoding_and_size(file_nm)?;

        // Get cloud provider from the entry, or use default logic
        let destination_cloud = match entry_json.get("destination_cloud").and_then(|v| v.as_str()) {
            Some(cloud) => cloud,
            None => {
                // Fall back to size-based logic if not specified
                if encoded_size > 20000 {
                    "r2"
                } else {
                    "azure_kv"
                }
            }
        };

        // For Azure KV, we need base64 encoding for binary files
        // For R2/B2, we can upload binary files directly
        let (final_encoding, final_file_size, final_encoded_size) =
            if destination_cloud == "azure_kv" {
                // We need proper encoding for Azure KV files
                check_encoding_and_size(file_nm)?
            } else {
                // For R2/B2, just use utf8 encoding and original file size
                let file_size = fs::metadata(file_nm)?.len();
                ("utf8".to_string(), file_size, file_size)
            };

        // Log verbose message for base64 encoding
        if final_encoding == "base64" && args.verbose > 0 {
            println!(
                "info: File {} contains non-UTF-8 data. Created base64 version ({}.base64). Will use base64 encoding when uploaded to Azure Key Vault.",
                file_nm, file_nm
            );
        }

        // Get cloud_upload_bucket from the entry if it's r2 or b2
        let cloud_upload_bucket = if destination_cloud == "r2" || destination_cloud == "b2" {
            entry_json
                .get("cloud_upload_bucket")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        } else {
            // For Azure, always use empty string as it doesn't need a bucket
            "".to_string()
        };

        // Create JSON entry
        let output_entry = json!({
            "file_nm": file_nm,
            "hash": hash,
            "hash_algo": "sha1",
            "ins_ts": ins_ts,
            "cloud_id": "", // These will be filled in later by cloud storage
            "cloud_cr_ts": "",
            "cloud_upd_ts": "",
            "hostname": hostname,
            "encoding": final_encoding,
            "file_size": final_file_size,
            "encoded_size": final_encoded_size,
            "destination_cloud": destination_cloud,
            "cloud_upload_bucket": cloud_upload_bucket
        });

        new_entries.push(output_entry);
    }

    // Initialize the output entries vector and counters
    let mut all_entries = Vec::new();
    let mut updated_entries_count = 0;
    let mut new_entries_count = 0;

    // Check if output file already exists and read its contents if it does
    if Path::new(output_filepath).exists() {
        let mut existing_file = File::open(output_filepath)?;
        let mut existing_content = String::new();
        existing_file.read_to_string(&mut existing_content)?;

        if !existing_content.trim().is_empty() {
            // Parse existing JSON entries
            let mut existing_entries: Vec<serde_json::Value> =
                serde_json::from_str(&existing_content)?;

            // Create a lookup map for existing entries (key = file_nm + hostname)
            let mut updated_indices = std::collections::HashSet::new();

            // Create a set to track which new entries have been processed
            let mut processed_new_entries = std::collections::HashSet::new();

            // Go through each new entry
            for (new_idx, new_entry) in new_entries.iter().enumerate() {
                // Get file_nm, hostname, and cloud provider from the new entry
                if let (Some(file_nm), Some(hostname), Some(cloud)) = (
                    new_entry.get("file_nm").and_then(|v| v.as_str()),
                    new_entry.get("hostname").and_then(|v| v.as_str()),
                    new_entry.get("destination_cloud").and_then(|v| v.as_str()),
                ) {
                    // Include cloud provider in the key to ensure same file with different providers gets separate entries
                    let lookup_key = format!("{}-{}-{}", file_nm, hostname, cloud);

                    // Look for a matching entry in existing entries
                    let mut found_match = false;

                    for (idx, existing_entry) in existing_entries.iter_mut().enumerate() {
                        if let (
                            Some(existing_file_nm),
                            Some(existing_hostname),
                            Some(existing_cloud),
                        ) = (
                            existing_entry.get("file_nm").and_then(|v| v.as_str()),
                            existing_entry.get("hostname").and_then(|v| v.as_str()),
                            existing_entry
                                .get("destination_cloud")
                                .and_then(|v| v.as_str()),
                        ) {
                            let existing_key = format!(
                                "{}-{}-{}",
                                existing_file_nm, existing_hostname, existing_cloud
                            );

                            // If match found, update the existing entry
                            // The existing_key already includes the destination_cloud now
                            if existing_key == lookup_key {
                                found_match = true;
                                updated_indices.insert(idx);
                                processed_new_entries.insert(new_idx);

                                // Update fields from new entry to existing entry
                                if let (Some(new_obj), Some(existing_obj)) =
                                    (new_entry.as_object(), existing_entry.as_object_mut())
                                {
                                    for (key, value) in new_obj {
                                        existing_obj.insert(key.clone(), value.clone());
                                    }
                                }

                                updated_entries_count += 1;
                                break;
                            }
                        }
                    }

                    // No need to add new entries here - we'll do it in one pass below
                    if !found_match {
                        // Mark this entry as not processed
                        // (meaning it needs to be added as a new entry)
                    }
                }
            }

            // Start with all existing entries in the output
            for entry in existing_entries {
                all_entries.push(entry);
            }

            // Add all new entries that weren't updates to existing ones
            for (idx, new_entry) in new_entries.iter().enumerate() {
                if !processed_new_entries.contains(&idx) {
                    all_entries.push(new_entry.clone());
                    new_entries_count += 1;
                }
            }
        } else {
            // No existing content, all entries are new
            all_entries.extend(new_entries.clone());
            new_entries_count = new_entries.len();
        }
    } else {
        // No existing file, all entries are new
        all_entries.extend(new_entries.clone());
        new_entries_count = new_entries.len();
    }

    // Write all entries to the output file
    let output_content = serde_json::to_string_pretty(&all_entries)?;
    let mut output_file = File::create(output_filepath)?;
    output_file.write_all(output_content.as_bytes())?;

    if args.verbose > 0 {
        if updated_entries_count > 0 && new_entries_count > 0 {
            println!(
                "info: Successfully updated output file with {} new entries and {} updated entries",
                new_entries_count, updated_entries_count
            );
        } else if updated_entries_count > 0 {
            println!(
                "info: Successfully updated output file with {} updated entries",
                updated_entries_count
            );
        } else {
            println!(
                "info: Successfully updated output file with {} new entries",
                new_entries_count
            );
        }
    }

    Ok(())
}

/// Calculate hash for a filepath (not the file contents)
/// This ensures consistent locations for files even if their content changes
fn calculate_hash(filepath: &str) -> Result<String> {
    crate::secrets::utils::calculate_hash(filepath)
}

/// Get the system hostname
fn hostname() -> Result<String> {
    crate::secrets::utils::get_hostname()
}
