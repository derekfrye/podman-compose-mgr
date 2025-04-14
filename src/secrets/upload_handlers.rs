use crate::interfaces::{
    AzureKeyVaultClient, B2StorageClient, R2StorageClient, ReadInteractiveInputHelper,
};
use crate::secrets::error::Result;
use crate::secrets::file_details::check_encoding_and_size;
use crate::secrets::models::UploadEntry;
use crate::secrets::user_prompt::prompt_for_upload_with_helper;
use serde_json::Value;
use std::fs;
use std::path::Path;

/// Handle R2 storage upload
pub fn handle_r2_upload<R: ReadInteractiveInputHelper>(
    entry: &UploadEntry,
    r2_client: &dyn R2StorageClient,
    read_val_helper: &R,
    verbose: i32,
) -> Result<Option<Value>> {
    if verbose > 0 {
        println!(
            "info: Uploading file {} to Cloudflare R2 storage",
            entry.file_nm
        );
    }

    // Convert to file details
    let file_details = entry.to_file_details();

    // Check if the file already exists in R2 storage
    let r2_file_exists = r2_client
        .check_file_exists_with_details(&entry.hash, entry.cloud_upload_bucket.clone())
        .ok()
        .flatten();

    // Get metadata to check file size if the file exists
    let mut r2_file_size: Option<u64> = None;

    // If file exists, print a concise warning
    let (file_exists, cloud_created, cloud_updated) =
        if let Some((exists, created, updated)) = r2_file_exists {
            if exists {
                eprintln!("warn: File already exists in R2 storage.");

                // Also get file metadata to check size
                if let Ok(Some(metadata)) = r2_client.get_file_metadata(&entry.hash) {
                    if let Some(content_length) = metadata.get("content_length") {
                        if let Ok(size) = content_length.parse::<u64>() {
                            r2_file_size = Some(size);
                        }
                    }
                }

                (true, Some(created), Some(updated))
            } else {
                (false, None, None)
            }
        } else {
            (false, None, None)
        };

    // Prompt the user for confirmation
    let upload_config = crate::secrets::user_prompt::UploadPromptConfig {
        file_path: &entry.file_nm,
        secret_exists: file_exists,
        cloud_created,
        cloud_updated,
        cloud_type: Some("r2"),
        cloud_file_size: r2_file_size,
        local_file_size: entry.encoded_size,
    };
    let upload_confirmed = prompt_for_upload_with_helper(&upload_config, read_val_helper)?;

    if !upload_confirmed {
        if verbose > 0 {
            println!("info: Skipping upload of {}", entry.file_nm);
        }
        return Ok(None);
    }

    // Upload to R2
    let r2_result = match r2_client.upload_file_with_details(&file_details) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("warn: Failed to upload to R2: {}", e);
            return Ok(None);
        }
    };

    if verbose > 0 {
        println!("info: Successfully uploaded to Cloudflare R2 storage");
    }

    // Create output entry with updated fields for R2
    Ok(Some(entry.create_r2_output_entry(&r2_result)))
}

/// Handle B2 storage upload (now redirects to R2)
pub fn handle_b2_upload<R: ReadInteractiveInputHelper>(
    entry: &UploadEntry,
    b2_client: &dyn B2StorageClient,
    read_val_helper: &R,
    verbose: i32,
) -> Result<Option<Value>> {
    if verbose > 0 {
        println!(
            "info: File {} is too large for Azure KeyVault ({}). Uploading to Backblaze B2 instead.",
            entry.file_nm, entry.encoded_size
        );
    }

    // Convert to file details
    let file_details = entry.to_file_details();

    // Check if the file already exists in B2 storage
    let b2_file_exists = b2_client
        .check_file_exists_with_details(&entry.hash, entry.cloud_upload_bucket.clone())
        .ok()
        .flatten();

    // Get metadata to check file size if the file exists
    let mut r2_file_size: Option<u64> = None;

    // If file exists, print a concise warning
    let (file_exists, cloud_created, cloud_updated) =
        if let Some((exists, created, updated)) = b2_file_exists {
            if exists {
                eprintln!("warn: File already exists in R2 storage.");

                // Also get file metadata to check size
                if let Ok(Some(metadata)) = b2_client.get_file_metadata(&entry.hash) {
                    if let Some(content_length) = metadata.get("content_length") {
                        if let Ok(size) = content_length.parse::<u64>() {
                            r2_file_size = Some(size);
                        }
                    }
                }

                (true, Some(created), Some(updated))
            } else {
                (false, None, None)
            }
        } else {
            (false, None, None)
        };

    // Prompt the user for confirmation
    let upload_config = crate::secrets::user_prompt::UploadPromptConfig {
        file_path: &entry.file_nm,
        secret_exists: file_exists,
        cloud_created,
        cloud_updated,
        cloud_type: Some("r2"),
        cloud_file_size: r2_file_size,
        local_file_size: entry.encoded_size,
    };
    let upload_confirmed = prompt_for_upload_with_helper(&upload_config, read_val_helper)?;

    if !upload_confirmed {
        if verbose > 0 {
            println!("info: Skipping upload of {}", entry.file_nm);
        }
        return Ok(None);
    }

    // Upload to R2 (redirected from B2)
    let r2_result = match b2_client.upload_file_with_details(&file_details) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("warn: Failed to upload to R2 storage: {}", e);
            return Ok(None);
        }
    };

    if verbose > 0 {
        println!("info: Successfully uploaded to R2 storage");
    }

    // Create output entry with updated fields for R2 (redirected from B2)
    Ok(Some(entry.create_r2_output_entry(&r2_result)))
}

/// Handle Azure KeyVault upload
pub fn handle_azure_upload<R: ReadInteractiveInputHelper>(
    entry: &UploadEntry,
    kv_client: &dyn AzureKeyVaultClient,
    read_val_helper: &R,
    verbose: i32,
) -> Result<Option<Value>> {
    // Check if the secret already exists in Azure KeyVault
    let (secret_exists, existing_created, existing_updated) =
        match kv_client.get_secret_value(&entry.hash) {
            Ok(secret) => {
                println!("Secret {} already exists in Azure Key Vault", entry.hash);
                (
                    true,
                    Some(secret.created.to_string()),
                    Some(secret.updated.to_string()),
                )
            }
            Err(_) => (false, None, None),
        };

    // Determine which file to use (original or base64 encoded)
    let file_to_use = if entry.encoding == "base64" {
        format!("{}.base64", entry.file_nm)
    } else {
        entry.file_nm.to_string()
    };

    // Verify the file exists
    if !Path::new(&file_to_use).exists() {
        // If base64 file doesn't exist, try to create it now
        if entry.encoding == "base64" {
            if verbose > 0 {
                println!(
                    "info: Base64 file {} doesn't exist, creating now",
                    file_to_use
                );
            }
            // This will create the base64 file if it doesn't exist
            let _ = check_encoding_and_size(&entry.file_nm)?;

            // Check again if it exists
            if !Path::new(&file_to_use).exists() {
                eprintln!(
                    "Failed to create base64 file for {}, skipping",
                    entry.file_nm
                );
                return Ok(None);
            }
        } else {
            eprintln!("File {} does not exist, skipping", file_to_use);
            return Ok(None);
        }
    }

    // Prompt the user for confirmation
    let upload_config = crate::secrets::user_prompt::UploadPromptConfig {
        file_path: &entry.file_nm,
        secret_exists,
        cloud_created: existing_created,
        cloud_updated: existing_updated,
        cloud_type: Some(&entry.destination_cloud),
        cloud_file_size: None, // Azure KeyVault doesn't expose secret size
        local_file_size: entry.encoded_size,
    };
    let upload_confirmed = prompt_for_upload_with_helper(&upload_config, read_val_helper)?;

    if !upload_confirmed {
        if verbose > 0 {
            println!("info: Skipping upload of {}", entry.file_nm);
        }
        return Ok(None);
    }

    // Upload to Azure KeyVault
    // Read file content
    let content = fs::read_to_string(&file_to_use)?;

    // Upload to Key Vault using hash as the key
    let response = kv_client
        .set_secret_value(&entry.hash, &content)
        .map_err(|e| {
            Box::<dyn std::error::Error>::from(format!(
                "Failed to upload secret {}: {}",
                entry.hash, e
            ))
        })?;

    if verbose > 0 {
        println!("info: Successfully uploaded to Azure Key Vault storage");
    }

    // Create output entry with updated fields for Azure
    Ok(Some(entry.create_azure_output_entry(&response)))
}
