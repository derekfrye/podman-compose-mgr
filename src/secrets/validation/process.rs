use crate::args::Args;
use crate::interfaces::AzureKeyVaultClient;
use crate::secrets::error::Result;
use crate::secrets::models::JsonOutput;
use crate::secrets::r2_storage::R2Client;
use crate::secrets::utils::{extract_validation_fields, get_current_timestamp, get_hostname};
use crate::secrets::validation::cloud_storage::{DownloadParams, download_from_cloud};
use crate::secrets::validation::file_ops::compare_files;
use crate::secrets::validation::ui::prompt_for_diff;
use crate::utils::log_utils::Logger;
use serde_json::Value;
use std::path::Path;
use tempfile::NamedTempFile;

/// Process a single entry for secret retrieval and comparison
///
/// This function:
/// 1. Downloads the file from appropriate cloud storage
/// 2. Compares with the local file if it exists
/// 3. Prompts the user to view differences or details
///
/// Returns the JSON output of the validation or None if the entry was skipped
pub fn process_retrieve_entry(
    entry: &Value,
    azure_client: &dyn AzureKeyVaultClient,
    r2_client: &R2Client,
    args: &Args,
    logger: &Logger,
) -> Result<Option<JsonOutput>> {
    // Extract required fields
    let (cloud_id, file_path, secret_name, encoding, storage_type) =
        extract_validation_fields(entry)?;

    // Check if file exists in cloud storage
    logger.info(&format!("Processing {}", file_path));

    // Create a temporary file to download the content
    let temp_file = NamedTempFile::new().map_err(|e| {
        Box::<dyn std::error::Error>::from(format!("Failed to create temporary file: {}", e))
    })?;
    let temp_path = temp_file.path().to_string_lossy().to_string();

    // Download content based on storage type
    let download_params = DownloadParams {
        storage_type: storage_type.clone(), // Clone to avoid moving the value
        secret_name,
        cloud_id: &cloud_id,
        azure_client,
        r2_client,
        entry,
        temp_path: &temp_path,
        encoding: encoding.as_str(),
        args,
    };
    let downloaded = download_from_cloud(download_params)?;

    if !downloaded {
        return Ok(None); // Skip if download failed
    }

    // Check if the local file exists
    if Path::new(&file_path).exists() {
        // Compare the downloaded file with the local file
        if compare_files(
            &temp_path,
            &file_path,
            encoding.as_str(),
            azure_client,
            entry,
            args,
        )? {
            println!("Files are identical: {}", file_path);
        } else {
            // Files differ - prompt user
            match prompt_for_diff(&temp_path, &file_path, entry, args)? {
                // No output needed for user's choice
                None => return Ok(None),
                // User asked to see diff, but we still return the validation result
                _ => {}
            }
        }
    } else {
        println!("Local file does not exist: {}", file_path);
    }

    // Create JSON output for this entry regardless of comparison result
    let formatted_date = get_current_timestamp()?;
    let hostname = get_hostname()?;

    // Create output with details from the cloud
    let output = create_retrieve_output(
        file_path,
        formatted_date,
        hostname,
        encoding,
        entry,
        storage_type.as_str(),
    )?;

    Ok(Some(output))
}

/// Create an output structure from the retrieved file
pub fn create_retrieve_output(
    file_nm: String,
    formatted_date: String,
    hostname: String,
    encoding: String,
    entry: &Value,
    _storage_type: &str, // Not used but kept for clarity
) -> Result<JsonOutput> {
    // Extract hash from entry
    let hash = entry["hash"].as_str().unwrap_or("").to_string();
    let hash_algo = entry["hash_algo"].as_str().unwrap_or("sha1").to_string();

    // Extract cloud details
    let cloud_id = entry["cloud_id"].as_str().unwrap_or("").to_string();
    let secret_name = entry["az_name"].as_str().unwrap_or("").to_string();
    let cloud_created = entry["cloud_cr_ts"].as_str().unwrap_or("").to_string();
    let cloud_updated = entry["cloud_upd_ts"].as_str().unwrap_or("").to_string();

    // Create output structure
    let output = JsonOutput {
        file_nm,
        md5: hash.clone(),
        ins_ts: formatted_date,
        az_id: cloud_id,
        az_create: cloud_created,
        az_updated: cloud_updated,
        az_name: secret_name,
        hostname,
        encoding,
        hash_val: hash,
        hash_algo,
    };

    Ok(output)
}
