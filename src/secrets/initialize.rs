use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use chrono::{DateTime, Local};
use serde_json::json;

use crate::secrets::file_details::check_encoding_and_size;
// Removed create_secret_name import as it's no longer used

use crate::Args;
use crate::secrets::error::Result;
use crate::utils::cmd_utils;

/// Process the initialization of secrets
///
/// Reads the secrets from the input file, creates JSON entries for each file,
/// and writes the results to the output_json file specified in the arguments.
pub fn process(args: &Args) -> Result<()> {
    // Get the required file paths from args
    let init_filepath = args.secrets_init_filepath.as_ref().unwrap();
    let output_filepath = args.output_json.as_ref().unwrap();

    // Read the input file containing the file paths
    let mut file = File::open(init_filepath)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    // Parse the JSON - it's either a map or an array of objects with filenm key
    let files: Vec<String> = if contents.trim().starts_with('{') {
        // Handle map format
        let files_map: HashMap<String, String> = serde_json::from_str(&contents)?;
        files_map.values().cloned().collect()
    } else {
        // Handle array format with objects that have filenm field
        let files_array: Vec<serde_json::Value> = serde_json::from_str(&contents)?;
        files_array
            .iter()
            .filter_map(|obj| obj.get("filenm").and_then(|v| v.as_str()).map(String::from))
            .collect()
    };

    // Get the hostname
    let hostname = hostname()?;

    // Create new entries for each file
    let mut new_entries = Vec::new();

    for file_nm in files {
        // Check if file exists
        if !Path::new(&file_nm).exists() {
            return Err(Box::<dyn std::error::Error>::from(format!(
                "File {} does not exist",
                file_nm
            )));
        }

        // Calculate SHA-1 hash
        let hash = calculate_hash(&file_nm)?;

        // Get current timestamp
        let now: DateTime<Local> = Local::now();
        let ins_ts = now.to_rfc3339();

        // Check encoding and get file sizes (creates base64 encoded file if needed)
        let (encoding, file_size, encoded_size) = check_encoding_and_size(&file_nm)?;

        // Determine destination cloud based on encoded size
        let destination_cloud = if encoded_size > 24000 {
            "r2"
        } else {
            "azure_kv"
        };

        // We now use hash directly instead of creating a secret name

        // Log verbose message for base64 encoding
        if encoding == "base64" && args.verbose > 0 {
            println!(
                "info: File {} contains non-UTF-8 data. Created base64 version ({}.base64). Will use base64 encoding when uploaded.",
                file_nm, file_nm
            );
        }

        // Set a default cloud_upload_bucket value
        // For B2 and R2, this will need to be specified in the JSON when uploading
        let cloud_upload_bucket = "".to_string();
        
        // Create JSON entry
        let entry = json!({
            "file_nm": file_nm,
            "hash": hash,
            "hash_algo": "sha1",
            "ins_ts": ins_ts,
            "cloud_id": "", // These will be filled in later by cloud storage
            "cloud_cr_ts": "",
            "cloud_upd_ts": "",
            "hostname": hostname,
            "encoding": encoding,
            "file_size": file_size,
            "encoded_size": encoded_size,
            "destination_cloud": destination_cloud,
            "cloud_upload_bucket": cloud_upload_bucket
        });

        new_entries.push(entry);
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
            let mut existing_entries: Vec<serde_json::Value> = serde_json::from_str(&existing_content)?;
            
            // Create a lookup map for existing entries (key = file_nm + hostname)
            let mut updated_indices = std::collections::HashSet::new();
            
            // Create a set to track which new entries have been processed
            let mut processed_new_entries = std::collections::HashSet::new();
            
            // Go through each new entry
            for (new_idx, new_entry) in new_entries.iter().enumerate() {
                // Get file_nm and hostname from the new entry
                if let (Some(file_nm), Some(hostname)) = (
                    new_entry.get("file_nm").and_then(|v| v.as_str()),
                    new_entry.get("hostname").and_then(|v| v.as_str())
                ) {
                    let lookup_key = format!("{}-{}", file_nm, hostname);
                    
                    // Look for a matching entry in existing entries
                    let mut found_match = false;
                    
                    for (idx, existing_entry) in existing_entries.iter_mut().enumerate() {
                        if let (Some(existing_file_nm), Some(existing_hostname)) = (
                            existing_entry.get("file_nm").and_then(|v| v.as_str()),
                            existing_entry.get("hostname").and_then(|v| v.as_str())
                        ) {
                            let existing_key = format!("{}-{}", existing_file_nm, existing_hostname);
                            
                            // If match found, update the existing entry
                            if existing_key == lookup_key {
                                found_match = true;
                                updated_indices.insert(idx);
                                processed_new_entries.insert(new_idx);
                                
                                // Update fields from new entry to existing entry
                                if let (Some(new_obj), Some(existing_obj)) = (
                                    new_entry.as_object(),
                                    existing_entry.as_object_mut()
                                ) {
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

/// Calculate hash for a file (using streaming for large files)
fn calculate_hash(filepath: &str) -> Result<String> {
    crate::secrets::utils::calculate_hash(filepath)
}

/// Get the system hostname
fn hostname() -> Result<String> {
    let hostname = cmd_utils::run_command_with_output("hostname", &[])?
        .trim()
        .to_string();

    Ok(hostname)
}
