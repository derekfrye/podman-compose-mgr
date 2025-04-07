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
            "b2"
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
        let cloud_upload_bucket = if destination_cloud == "b2" || destination_cloud == "r2" {
            "bucket_required_during_upload".to_string()
        } else {
            // For Azure KeyVault, this is not needed
            "".to_string()
        };
        
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

    // Create or read existing entries
    let mut all_entries = Vec::new();

    // Check if output file already exists and read its contents if it does
    if Path::new(output_filepath).exists() {
        let mut existing_file = File::open(output_filepath)?;
        let mut existing_content = String::new();
        existing_file.read_to_string(&mut existing_content)?;

        if !existing_content.trim().is_empty() {
            // Parse existing JSON entries
            let existing_entries: Vec<serde_json::Value> = serde_json::from_str(&existing_content)?;

            // Add existing entries to all_entries
            all_entries.extend(existing_entries);
        }
    }

    // Store the count of new entries
    let new_entries_count = new_entries.len();

    // Add new entries to all_entries
    all_entries.extend(new_entries);

    // Write all entries to the output file
    let output_content = serde_json::to_string_pretty(&all_entries)?;
    let mut output_file = File::create(output_filepath)?;
    output_file.write_all(output_content.as_bytes())?;

    if args.verbose > 0 {
        println!(
            "info: Successfully updated output file with {} new entries",
            new_entries_count
        );
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
