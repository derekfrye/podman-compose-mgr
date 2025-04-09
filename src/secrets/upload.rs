use crate::args::Args;
use crate::interfaces::{
    AzureKeyVaultClient, B2StorageClient, DefaultB2StorageClient, DefaultR2StorageClient,
    DefaultReadInteractiveInputHelper, R2StorageClient, ReadInteractiveInputHelper,
};
use crate::secrets::azure::get_keyvault_client;
use crate::secrets::error::Result;
use crate::secrets::json_utils;
use crate::secrets::upload_handlers;
use crate::secrets::utils::get_hostname;
use crate::utils::log_utils::Logger;
use std::path::Path;

/// Process the upload operation to cloud storage using default implementations
pub fn process(args: &Args, logger: &Logger) -> Result<()> {
    // Use default read helper
    let read_val_helper = DefaultReadInteractiveInputHelper;
    process_with_injected_dependencies(args, &read_val_helper, logger)
}

/// Process the upload operation with dependency injection for testing
pub fn process_with_injected_dependencies<R: ReadInteractiveInputHelper>(
    args: &Args,
    read_val_helper: &R,
    logger: &Logger,
) -> Result<()> {
    // Validate that required params exist
    let input_json_path = args
        .input_json
        .as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Input JSON path is required"))?;
    let _ = args
        .output_json
        .as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Output JSON path is required"))?;

    // First, check if any entries in the JSON file match our hostname
    // Only initialize clients if needed
    let entries = json_utils::read_input_json(input_json_path)?;
    let our_hostname = get_hostname()?;

    // Initialize flags to track which clients we need
    let mut need_azure_client = false;
    let mut need_r2_client = false;

    // Check all entries to see if any match our hostname
    for entry in &entries {
        // Get hostname - convert to string for consistent comparison
        let entry_hostname = match entry["hostname"].as_str() {
            Some(h) => h,
            None => &our_hostname, // Use current hostname if missing
        };

        // If this entry is not for our host, skip it
        if entry_hostname != our_hostname {
            continue;
        }

        // This entry is for our host, determine which client it needs
        let destination_cloud = entry["destination_cloud"].as_str().unwrap_or("azure_kv");
        let encoded_size = entry["encoded_size"]
            .as_u64()
            .unwrap_or_else(|| entry["file_size"].as_u64().unwrap_or(0));
        let too_large_for_keyvault = encoded_size > 24000;

        if destination_cloud == "r2" || destination_cloud == "b2" || too_large_for_keyvault {
            // Both B2 and R2 now use the R2 client
            need_r2_client = true;
        } else {
            need_azure_client = true;
        }
    }

    // Only create clients that are actually needed

    // Create Azure Key Vault client if needed
    let kv_client = if need_azure_client {
        let client_id = args.secrets_client_id.as_ref().ok_or_else(|| {
            Box::<dyn std::error::Error>::from("Client ID is required for Azure uploads")
        })?;
        let client_secret_path = args.secrets_client_secret_path.as_ref().ok_or_else(|| {
            Box::<dyn std::error::Error>::from("Client secret path is required for Azure uploads")
        })?;
        let tenant_id = args.secrets_tenant_id.as_ref().ok_or_else(|| {
            Box::<dyn std::error::Error>::from("Tenant ID is required for Azure uploads")
        })?;
        let key_vault_name = args.secrets_vault_name.as_ref().ok_or_else(|| {
            Box::<dyn std::error::Error>::from("Key vault name is required for Azure uploads")
        })?;

        // Create KeyVault client
        get_keyvault_client(client_id, client_secret_path, tenant_id, key_vault_name)?
    } else {
        // Create dummy client if not needed
        println!("No Azure KeyVault uploads required for this host, using dummy client");
        Box::new(crate::interfaces::MockAzureKeyVaultClient::new())
    };

    // Create dummy B2 client (all B2 operations now redirect to R2)
    let b2_client = DefaultB2StorageClient::new_dummy();

    // Create R2 client if needed
    let r2_client = if need_r2_client {
        DefaultR2StorageClient::from_args(args).unwrap_or_else(|e| {
            // This is an error if we actually need the R2 client
            eprintln!(
                "warn: R2 client initialization failed but R2 uploads are needed: {}",
                e
            );
            DefaultR2StorageClient::new_dummy()
        })
    } else {
        // Create dummy client if not needed
        if args.verbose > 0 {
            println!("info: No R2 uploads required for this host, using dummy client");
        }
        DefaultR2StorageClient::new_dummy()
    };

    // Call the function that allows injection of the clients
    process_with_injected_dependencies_and_clients(
        args,
        read_val_helper,
        kv_client,
        Box::new(b2_client),
        Box::new(r2_client),
        logger,
    )
}

/// Process the upload operation with full dependency injection for testing
/// This version allows injecting Azure KeyVault, B2 Storage and R2 Storage clients for testing
pub fn process_with_injected_dependencies_and_clients<R: ReadInteractiveInputHelper>(
    args: &Args,
    read_val_helper: &R,
    kv_client: Box<dyn AzureKeyVaultClient>,
    b2_client: Box<dyn B2StorageClient>,
    r2_client: Box<dyn R2StorageClient>,
    logger: &Logger,
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
    logger.info("Testing connection to cloud storage services...");

    // Read input JSON file
    let entries = json_utils::read_input_json(input_filepath)?;
    let our_hostname = get_hostname()?;

    // Storage for processed entries
    let mut processed_entries = Vec::new();

    // Process each entry
    for entry_value in entries {
        // Parse the entry - skip if there are errors
        let entry = match json_utils::parse_entry(&entry_value) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Error parsing entry: {}", e);
                continue;
            }
        };

        // Skip this entry if it's not for this host
        if entry.hostname != our_hostname {
            if args.verbose > 0 {
                println!(
                    "info: Skipping file {} because it is not on the current host",
                    entry.file_nm
                );
            }
            continue;
        }

        // Check if file exists
        if !Path::new(&entry.file_nm).exists() {
            eprintln!("File {} does not exist, skipping", entry.file_nm);
            continue;
        }

        // For B2 and R2, cloud_upload_bucket is required
        if (entry.destination_cloud == "b2" || entry.destination_cloud == "r2")
            && entry.cloud_upload_bucket.is_none()
        {
            eprintln!(
                "Error: cloud_upload_bucket is required in JSON when destination_cloud is '{}' for file {}",
                entry.destination_cloud, entry.file_nm
            );
            continue;
        }

        // Handle different storage backends based on file size and destination_cloud
        let output_entry = if entry.destination_cloud == "r2" {
            // Handle R2 storage upload
            upload_handlers::handle_r2_upload(&entry, r2_client.as_ref(), read_val_helper, args.verbose.into())?
        } else if entry.is_too_large_for_keyvault() || entry.destination_cloud == "b2" {
            // Handle B2 storage upload (redirected to R2)
            upload_handlers::handle_b2_upload(&entry, b2_client.as_ref(), read_val_helper, args.verbose.into())?
        } else {
            // Handle Azure KeyVault upload
            upload_handlers::handle_azure_upload(&entry, kv_client.as_ref(), read_val_helper, args.verbose.into())?
        };

        // Add to processed entries if upload was successful
        if let Some(entry) = output_entry {
            processed_entries.push(entry);
        }
    }

    // Save output JSON
    json_utils::save_output_json(output_filepath, &processed_entries, args.verbose.into())?;

    Ok(())
}