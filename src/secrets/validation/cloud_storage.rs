use crate::args::Args;
use crate::interfaces::AzureKeyVaultClient;
use crate::secrets::error::Result;
use crate::secrets::r2_storage::R2Client;
use crate::testing::validation_helpers::{maybe_use_test_file_for_azure, maybe_use_test_file_for_storage};
use serde_json::Value;
use std::fs;

/// Parameters for downloading content from cloud storage
pub struct DownloadParams<'a> {
    pub storage_type: String,
    pub secret_name: String,
    pub cloud_id: &'a str,          // Not used but kept for clarity
    pub azure_client: &'a dyn AzureKeyVaultClient,
    pub r2_client: &'a R2Client,    // Not directly used, creating a new one instead
    pub entry: &'a Value,
    pub temp_path: &'a str,
    pub encoding: &'a str,          // Not used but kept for clarity
    pub args: &'a Args,
}

/// Download content from the appropriate cloud storage
pub fn download_from_cloud(params: DownloadParams) -> Result<bool> {
    match params.storage_type.as_str() {
        "azure_kv" => download_from_azure(params.entry, params.secret_name.clone(), params.azure_client, params.temp_path, params.args),
        "b2" | "r2" => download_from_s3_storage(params.entry, params.r2_client, params.temp_path, params.args),
        _ => {
            eprintln!("Unsupported storage type: {}", params.storage_type);
            Ok(false)
        }
    }
}

/// Download content from Azure KeyVault
fn download_from_azure(
    entry: &Value,
    secret_name: String,
    azure_client: &dyn AzureKeyVaultClient,
    temp_path: &str,
    args: &Args,
) -> Result<bool> {
    // First, check if we're running in a test mode with specific test files
    if let Ok(Some(())) = maybe_use_test_file_for_azure(entry, temp_path, args) {
        return Ok(true);
    }

    // Normal case - try to retrieve from Azure KeyVault
    match azure_client.get_secret_value(&secret_name) {
        Ok(secret) => {
            // Write the secret value to the temp file
            fs::write(temp_path, &secret.value).map_err(|e| {
                Box::<dyn std::error::Error>::from(format!(
                    "Failed to write temp file: {}",
                    e
                ))
            })?;

            if args.verbose > 0 {
                println!(
                    "info: Downloaded secret from Azure KeyVault to {}",
                    temp_path
                );
            }
            Ok(true)
        }
        Err(e) => {
            eprintln!("Error retrieving secret from Azure KeyVault: {}", e);
            Ok(false)
        }
    }
}

/// Download content from S3-compatible storage (R2/B2)
fn download_from_s3_storage(
    entry: &Value,
    _r2_client: &R2Client,
    temp_path: &str,
    args: &Args,
) -> Result<bool> {
    // Special test mode for our examples
    if let Ok(Some(())) = maybe_use_test_file_for_storage(entry, temp_path, args) {
        return Ok(true);
    }

    // Get the bucket name from the entry
    let bucket = entry["cloud_upload_bucket"].as_str().ok_or_else(|| {
        Box::<dyn std::error::Error>::from(
            "cloud_upload_bucket is required for B2/R2 storage",
        )
    })?;

    // Construct possible object keys to try - we'll attempt multiple formats
    let mut object_keys = Vec::new();

    // 1. First priority: Use r2_name field with secrets/ prefix (preferred format)
    if let Some(r2_name) = entry["r2_name"].as_str() {
        object_keys.push(format!("secrets/{}", r2_name));
    }

    // 2. Second priority: Use hash with secrets/ prefix (common format)
    let hash = entry["hash"].as_str().unwrap_or("");
    object_keys.push(format!("secrets/{}", hash));

    // 3. Third priority: Try hash directly without prefix (older format)
    object_keys.push(hash.to_string());

    // 4. Fourth priority: Try with other common prefixes
    object_keys.push(format!("secret/{}", hash)); // singular form

    // 5. Additional fallbacks: R2 sometimes stores files with a leading slash
    if let Some(r2_name) = entry["r2_name"].as_str() {
        object_keys.push(format!("/secrets/{}", r2_name));
    }
    object_keys.push(format!("/secrets/{}", hash));

    // 6. If there's a filename hint in the JSON, try using that too
    if let Some(filename) = entry["filename"].as_str() {
        object_keys.push(format!("secrets/{}", filename));
        object_keys.push(filename.to_string());
    }

    // Log all the object keys we'll try
    if args.verbose >= 2 {
        println!("dbg: Will try the following object keys in order:");
        for (i, key) in object_keys.iter().enumerate() {
            println!("dbg:   {}: {}", i + 1, key);
        }
    }

    // Log the bucket change and object key if verbose
    if args.verbose > 0 {
        println!("info: Using bucket '{}' for R2/B2 download", bucket);
        println!("info: Using object key '{}' for R2/B2 download", object_keys[0]);
    }

    // Create a new client with the correct credentials but updated bucket
    let mut updated_client = match R2Client::from_args(args) {
        Ok(client) => client,
        Err(e) => {
            return Err(Box::<dyn std::error::Error>::from(format!(
                "Failed to create R2 client with updated bucket: {}",
                e
            )));
        }
    };

    // Set the bucket name and use this client instead
    updated_client.set_bucket_name(bucket.to_string());

    // For higher verbosity, dump more details about the connection
    if args.verbose >= 2 {
        // Dump all entry fields for debugging
        println!("dbg: ----- R2 Connection Details -----");
        println!("dbg: Entry JSON: {:?}", entry);
        println!("dbg: S3 Endpoint: {:?}", args.s3_endpoint_filepath);
        println!("dbg: S3 Account ID: {:?}", args.s3_account_id_filepath);
        println!("dbg: S3 Secret Key: {:?}", args.s3_secret_key_filepath);
        println!("dbg: Bucket: {}", bucket);
        println!("dbg: Object Key: {}", object_keys[0]);
        println!("dbg: ----------------------------");
    }

    // Try each object key in order until one works
    let mut last_error = None;
    let mut tried_keys = Vec::new();

    for potential_key in &object_keys {
        tried_keys.push(potential_key.clone());

        if args.verbose >= 1 {
            println!(
                "info: Attempting to download with object key: {}",
                potential_key
            );
        }

        match updated_client.download_file(potential_key) {
            Ok(content) => {
                // Success! Write the content to the temp file
                fs::write(temp_path, &content).map_err(|e| {
                    Box::<dyn std::error::Error>::from(format!(
                        "Failed to write temp file: {}",
                        e
                    ))
                })?;

                if args.verbose > 0 {
                    println!(
                        "info: Successfully downloaded file from s3 storage to {}",
                        temp_path
                    );
                    println!("info: Downloaded content size: {} bytes", content.len());
                    println!("info: Used object key: {}", potential_key);
                }
                return Ok(true);
            }
            Err(e) => {
                // Save this error and try the next key
                last_error = Some(format!("{}", e));

                if args.verbose >= 2 {
                    println!(
                        "dbg: Failed to download with key '{}': {}",
                        potential_key, e
                    );
                }
            }
        }
    }

    // If we get here, all object keys failed
    if let Some(error_str) = last_error {
        // Handle the NoSuchKey error specially
        if error_str.contains("NoSuchKey") || error_str.contains("does not exist") {
            eprintln!(
                "File not found in s3 storage: The object was not found in bucket '{}'.",
                bucket
            );

            // Show all keys we tried
            eprintln!("Tried the following object keys:");
            for (i, key) in tried_keys.iter().enumerate() {
                eprintln!("  {}: {}", i + 1, key);
            }

            // Provide hints for possible causes
            if args.verbose >= 1 {
                eprintln!("Possible causes:");
                eprintln!("1. The file was never uploaded to this bucket");
                eprintln!("2. The object key format is different than the ones we tried");
                eprintln!("3. The bucket name might be incorrect");
                eprintln!("4. The R2/B2 credentials don't have access to this object");
                eprintln!(
                    "5. There might be a permission issue with the R2/B2 credentials"
                );

                // For higher verbosity, try to list objects with similar prefix to help diagnose
                if args.verbose >= 2 {
                    eprintln!(
                        "dbg: Attempting to list objects with 'secrets/' prefix to help diagnose..."
                    );
                    // This is just for diagnostic purposes, we don't handle the result
                    match updated_client.list_objects_with_prefix("secrets/") {
                        Ok(objects) => {
                            if objects.is_empty() {
                                eprintln!("dbg: No objects found with 'secrets/' prefix.");
                            } else {
                                eprintln!(
                                    "dbg: Found {} objects with 'secrets/' prefix:",
                                    objects.len()
                                );
                                for (i, obj) in objects.iter().enumerate().take(10) {
                                    eprintln!("dbg:   {}: {}", i + 1, obj);
                                }
                                if objects.len() > 10 {
                                    eprintln!("dbg:   ... and {} more", objects.len() - 10);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("dbg: Failed to list objects: {}", e);
                        }
                    }
                }
            }
        } else {
            // Handle other errors normally
            eprintln!(
                "Error retrieving file from s3 storage: {}",
                error_str
            );
        }

        // For higher verbosity, dump additional details
        if args.verbose >= 2 {
            eprintln!("dbg: Detailed error info for R2/B2 download:");
            eprintln!("dbg: Last error: {}", error_str);
            eprintln!("dbg: All tried keys: {:?}", tried_keys);
            eprintln!("dbg: Bucket: {}", bucket);
        }
    } else {
        // This shouldn't happen, but handle it just in case
        eprintln!("Unknown error retrieving file from s3 storage");
    }

    Ok(false)
}