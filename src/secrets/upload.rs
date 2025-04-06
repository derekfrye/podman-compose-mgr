use crate::args::Args;
use crate::interfaces::{
    AzureKeyVaultClient, DefaultReadInteractiveInputHelper, ReadInteractiveInputHelper,
};
use crate::secrets::azure::get_keyvault_client;
use crate::secrets::b2_storage::B2Client;
use crate::secrets::error::Result;
use crate::secrets::file_details::{check_encoding_and_size, FileDetails};
use crate::secrets::upload_utils::create_secret_name;
use crate::secrets::user_prompt::prompt_for_upload_with_helper;
use crate::secrets::utils::get_hostname;

use serde_json::{Value, json};
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::Path;


/// Process the upload operation to cloud storage using default implementations
pub fn process(args: &Args) -> Result<()> {
    // Use default read helper
    let read_val_helper = DefaultReadInteractiveInputHelper;
    process_with_injected_dependencies(args, &read_val_helper)
}

/// Process the upload operation with dependency injection for testing
pub fn process_with_injected_dependencies<R: ReadInteractiveInputHelper>(
    args: &Args,
    read_val_helper: &R,
) -> Result<()> {
    // Validate that required params exist
    let _input_json_path = args
        .input_json
        .as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Input JSON path is required"))?;
    let _ = args
        .output_json
        .as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Output JSON path is required"))?;
        

    // Create Azure Key Vault client
    let client_id = args
        .secrets_client_id
        .as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Client ID is required"))?;
    let client_secret_path = args
        .secrets_client_secret_path
        .as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Client secret path is required"))?;
    let tenant_id = args
        .secrets_tenant_id
        .as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Tenant ID is required"))?;
    let key_vault_name = args
        .secrets_vault_name
        .as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Key vault name is required"))?;

    // Create KeyVault client
    let kv_client = get_keyvault_client(client_id, client_secret_path, tenant_id, key_vault_name)?;

    // Call the function that allows injection of the clients
    process_with_injected_dependencies_and_client(args, read_val_helper, kv_client)
}

/// Process the upload operation with full dependency injection for testing
/// This version allows injecting mock clients for testing
pub fn process_with_injected_dependencies_and_client<R: ReadInteractiveInputHelper>(
    args: &Args,
    read_val_helper: &R,
    kv_client: Box<dyn AzureKeyVaultClient>,
) -> Result<()> {
    // Get required parameters from args
    let input_filepath = args
        .input_json
        .as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Input JSON path is required"))?;
    let output_filepath = args
        .output_json
        .as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Output JSON path is required"))?;

    // Test connection to storage
    if args.verbose {
        println!("Testing connection to cloud storage services...");
    }

    // Read input JSON file
    let mut file = File::open(input_filepath)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let our_hostname = get_hostname()?;

    // Parse JSON as array
    let entries: Vec<Value> = serde_json::from_str(&content)?;

    // Storage for processed entries
    let mut processed_entries = Vec::new();

    // Process each entry
    for entry in entries {
        // Get file path - support both "filenm" (legacy) and "file_nm" (new) fields
        let file_path = entry["file_nm"]
            .as_str()
            .or_else(|| entry["filenm"].as_str())
            .ok_or_else(|| {
                Box::<dyn std::error::Error>::from(format!(
                    "Missing file_nm field in entry: {}",
                    entry
                ))
            })?;

        // Get hash value
        let hash = entry["hash"].as_str().ok_or_else(|| {
            Box::<dyn std::error::Error>::from(format!("Missing hash field in entry: {}", entry))
        })?;

        // Get hash algorithm
        let hash_algo = entry["hash_algo"].as_str().unwrap_or("sha1");

        // Get timestamp
        let ins_ts = entry["ins_ts"].as_str().ok_or_else(|| {
            Box::<dyn std::error::Error>::from(format!("Missing ins_ts field in entry: {}", entry))
        })?;

        // Get hostname - legacy or new
        let hostname = entry["hostname"].as_str().ok_or_else(|| {
            Box::<dyn std::error::Error>::from(format!(
                "Missing hostname field in entry: {}",
                entry
            ))
        })?;

        // Get encoding - defaults to utf8 for backward compatibility
        let encoding = entry["encoding"].as_str().unwrap_or("utf8");

        // Get file sizes
        let file_size = entry["file_size"].as_u64().unwrap_or(0);
        let encoded_size = entry["encoded_size"].as_u64().unwrap_or(file_size);

        // Skip this entry if it's not for this host
        if hostname != our_hostname {
            if args.verbose {
                println!(
                    "Skipping file {} because it is not on the current host",
                    file_path
                );
            }
            continue;
        }

        // Check if file exists
        if !Path::new(file_path).exists() {
            eprintln!("File {} does not exist, skipping", file_path);
            continue;
        }

        // Get or generate secret name
        let secret_name = entry["secret_name"]
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| create_secret_name(hash));

        // Determine which storage backend to use
        let destination_cloud = entry["destination_cloud"].as_str().unwrap_or("azure_kv");

        // Get cloud upload bucket if specified
        let cloud_upload_bucket = entry["cloud_upload_bucket"].as_str().map(String::from);
        let too_large_for_keyvault = encoded_size > 24000;

        // Handle different storage backends based on file size
        if too_large_for_keyvault || destination_cloud == "b2" {
            if args.verbose {
                println!(
                    "File {} is too large for Azure KeyVault ({}). Uploading to Backblaze B2 instead.",
                    file_path, encoded_size
                );
            }
            
            // NOTE: This is a partial implementation that compiles but may not be fully functional yet
            // until we resolve issues with AWS SDK credentials
            let b2_client = match B2Client::from_args(args) {
                Ok(client) => client,
                Err(e) => {
                    eprintln!("Failed to create B2 client: {}", e);
                    continue;
                }
            };
            
            // Create a FileDetails struct for the file
            let file_details = FileDetails {
                file_path: file_path.to_string(),
                file_size,
                encoded_size,
                last_modified: String::new(), // Not needed for upload
                secret_name: secret_name.clone(),
                encoding: encoding.to_string(),
                cloud_created: None,
                cloud_updated: None,
                cloud_type: Some("b2".to_string()),
                hash: hash.to_string(),
                hash_algo: hash_algo.to_string(),
                cloud_upload_bucket: cloud_upload_bucket.clone(), // Use the bucket from JSON
            };
            
            // Prompt the user for confirmation
            let upload_confirmed = prompt_for_upload_with_helper(
                file_path,
                &secret_name,
                read_val_helper,
                false, // Don't check B2 existence yet
                None,
                None,
                Some("b2"),
            )?;
            
            if !upload_confirmed {
                if args.verbose {
                    println!("Skipping upload of {}", file_path);
                }
                continue;
            }
            
            // Upload to B2
            let b2_result = match b2_client.upload_file_with_details(&file_details) {
                Ok(result) => result,
                Err(e) => {
                    eprintln!("Failed to upload to B2: {}", e);
                    continue;
                }
            };
            
            if args.verbose {
                println!("Successfully uploaded to Backblaze B2 storage");
            }
            
            // Create output entry with updated fields
            let output_entry = json!({
                "file_nm": file_path,
                "hash": hash,
                "hash_algo": hash_algo,
                "ins_ts": ins_ts,
                "cloud_id": b2_result.id,
                "cloud_cr_ts": "",  // B2 doesn't provide created time separately
                "cloud_upd_ts": "", // Use current time if needed
                "hostname": hostname,
                "encoding": encoding,
                "file_size": file_size,
                "encoded_size": encoded_size,
                "destination_cloud": "b2",
                "secret_name": secret_name,
                "cloud_upload_bucket": cloud_upload_bucket.unwrap_or_else(|| "".to_string()),
                "b2_hash": b2_result.hash,
                "b2_bucket_id": b2_result.bucket_id,
                "b2_name": b2_result.name
            });
            
            processed_entries.push(output_entry);
            continue;
        }
        

        // Check if the secret already exists in Azure KeyVault
        let (secret_exists, existing_created, existing_updated) =
            match kv_client.get_secret_value(&secret_name) {
                Ok(secret) => {
                    println!("Secret {} already exists in Azure Key Vault", secret_name);
                    (
                        true,
                        Some(secret.created.to_string()),
                        Some(secret.updated.to_string()),
                    )
                }
                Err(_) => (false, None, None),
            };

        // Determine which file to use (original or base64 encoded)
        let file_to_use = if encoding == "base64" {
            format!("{}.base64", file_path)
        } else {
            file_path.to_string()
        };

        // Verify the file exists
        if !Path::new(&file_to_use).exists() {
            // If base64 file doesn't exist, try to create it now
            if encoding == "base64" {
                if args.verbose {
                    println!("Base64 file {} doesn't exist, creating now", file_to_use);
                }
                // This will create the base64 file if it doesn't exist
                let _ = check_encoding_and_size(file_path)?;

                // Check again if it exists
                if !Path::new(&file_to_use).exists() {
                    eprintln!("Failed to create base64 file for {}, skipping", file_path);
                    continue;
                }
            } else {
                eprintln!("File {} does not exist, skipping", file_to_use);
                continue;
            }
        }

        // Prompt the user for confirmation
        let upload_confirmed = prompt_for_upload_with_helper(
            file_path,
            &secret_name,
            read_val_helper,
            secret_exists,
            existing_created,
            existing_updated,
            Some(destination_cloud),
        )?;

        if !upload_confirmed {
            if args.verbose {
                println!("Skipping upload of {}", file_path);
            }
            continue;
        }

        // Upload to Azure KeyVault
        // Read file content
        let content = fs::read_to_string(&file_to_use)?;

        // Upload to Key Vault
        let response = kv_client
            .set_secret_value(&secret_name, &content)
            .map_err(|e| {
                Box::<dyn std::error::Error>::from(format!(
                    "Failed to upload secret {}: {}",
                    secret_name, e
                ))
            })?;

        if args.verbose {
            println!("Successfully uploaded to Azure Key Vault storage");
        }

        // Create output entry with updated fields
        let output_entry = json!({
            "file_nm": file_path,
            "hash": hash,
            "hash_algo": hash_algo,
            "ins_ts": ins_ts,
            "cloud_id": response.id,
            "cloud_cr_ts": response.created.to_string(),
            "cloud_upd_ts": response.updated.to_string(),
            "hostname": hostname,
            "encoding": encoding,
            "file_size": file_size,
            "encoded_size": encoded_size,
            "destination_cloud": destination_cloud,
            "secret_name": secret_name,
            "cloud_upload_bucket": cloud_upload_bucket.unwrap_or_else(|| "".to_string())
        });

        processed_entries.push(output_entry);
    }

    
    // Append to output file if we have any entries
    if !processed_entries.is_empty() {
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
                serde_json::from_str(&existing_content)?
            };

            // Append new entries
            existing_entries.extend(processed_entries.clone());

            // Write back as valid JSON array
            let mut file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(output_filepath)?;

            serde_json::to_writer_pretty(&mut file, &existing_entries)?;
        } else {
            // Create new file with JSON array
            let mut file = File::create(output_filepath)?;
            serde_json::to_writer_pretty(&mut file, &processed_entries)?;
        }

        if args.verbose {
            println!(
                "Successfully saved {} entries to {}",
                processed_entries.len(),
                output_filepath.display()
            );
        }
    } else if args.verbose {
        println!("No entries were processed successfully.");
    }

    Ok(())
}
