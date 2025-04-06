use crate::args::Args;
use crate::interfaces::{DefaultReadInteractiveInputHelper, ReadInteractiveInputHelper, AzureKeyVaultClient};
use crate::secrets::azure::get_keyvault_client;
use crate::secrets::error::Result;
use crate::secrets::upload_utils::create_encoded_secret_name;
use crate::secrets::user_prompt::prompt_for_upload_with_helper;
use crate::secrets::utils::get_hostname;
use azure_core::base64;

use serde_json::{json, Value};
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::Path;

/// Process the upload operation to Azure Key Vault using default implementations
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
    // Validate that required params exist, even though we don't use them directly here
    let _ = args.input_json.as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Input JSON path is required"))?;
    let _ = args.output_json.as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Output JSON path is required"))?;
    
    // Create Azure Key Vault client
    let client_id = args.secrets_client_id.as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Client ID is required"))?;
    let client_secret_path = args.secrets_client_secret_path.as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Client secret path is required"))?;
    let tenant_id = args.secrets_tenant_id.as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Tenant ID is required"))?;
    let key_vault_name = args.secrets_vault_name.as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Key vault name is required"))?;
    
    // Create KeyVault client
    let kv_client = get_keyvault_client(client_id, client_secret_path, tenant_id, key_vault_name)?;
    
    // Call the function that allows injection of the KeyVault client
    process_with_injected_dependencies_and_client(args, read_val_helper, kv_client)
}

/// Process the upload operation with full dependency injection for testing
/// This version allows injecting a mock AzureKeyVaultClient for testing
pub fn process_with_injected_dependencies_and_client<R: ReadInteractiveInputHelper>(
    args: &Args,
    read_val_helper: &R,
    kv_client: Box<dyn AzureKeyVaultClient>,
) -> Result<()> {
    // Get required parameters from args
    let input_filepath = args.input_json.as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Input JSON path is required"))?;
    let output_filepath = args.output_json.as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Output JSON path is required"))?;
    
    // Test connection to Azure Key Vault
    if args.verbose {
        println!("Testing connection to Azure Key Vault...");
    }
    
    // Read input JSON file
    let mut file = File::open(input_filepath)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let our_hostname = get_hostname()?;
    
    // Parse JSON as array
    let entries: Vec<Value> = serde_json::from_str(&content)?;
    
    // Storage for processed entries
    let mut azure_secret_set_output = Vec::new();
    
    // Process each entry
    for entry in entries {
        let filenm = entry["filenm"].as_str().ok_or_else(|| 
            Box::<dyn std::error::Error>::from(format!("Missing filenm field in entry: {}", entry)))?;
        let md5 = entry["md5"].as_str().ok_or_else(|| 
            Box::<dyn std::error::Error>::from(format!("Missing md5 field in entry: {}", entry)))?;
        // Get hostname either from the entry or from the system
        let hostname = match entry["hostname"].as_str() {
            Some(h) => h.to_string(),
            None => get_hostname().unwrap_or_default()
        };
        let ins_ts = entry["ins_ts"].as_str().ok_or_else(|| 
            Box::<dyn std::error::Error>::from(format!("Missing ins_ts field in entry: {}", entry)))?;
        
        // Skip this entry if the file doesn't exist
        if hostname!=our_hostname {
            if args.verbose {
                println!("Skipping file {} because it is not on the current host", filenm);
            }
            continue;
        }
        if !Path::new(filenm).exists() {
            eprintln!("File {} does not exist, skipping", filenm);
            continue;
        }
        
        // Create a secret name from the file path
        let secret_name = create_encoded_secret_name(filenm);
        
        // Step 2: Check if the secret already exists using the interface and get its metadata
        let (secret_exists, existing_created, existing_updated) = match kv_client.get_secret_value(&secret_name) {
            Ok(secret) => {
                println!("Secret {} already exists in the key vault", secret_name);
                (true, 
                 Some(secret.created.to_string()), 
                 Some(secret.updated.to_string()))
            },
            Err(_) => (false, None, None),
        };
        
        // Read file content as bytes (handles non-UTF-8 files)
        let file_bytes = fs::read(filenm)
            .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to read file {}: {}", filenm, e)))?;
            
        // Check if the file is valid UTF-8
        let (content, encoding) = match std::str::from_utf8(&file_bytes) {
            Ok(text) => (text.to_string(), "utf8"),
            Err(_) => {
                if args.verbose {
                    println!("Warning: File {} contains non-UTF-8 data. Using base64 encoding.", filenm);
                }
                // Convert binary data to base64 string
                (base64::encode(&file_bytes), "base64")
            }
        };
            
        // Prompt the user for confirmation using the injected helper
        let upload_confirmed = prompt_for_upload_with_helper(
            filenm, 
            &secret_name, 
            read_val_helper, 
            secret_exists, 
            existing_created, 
            existing_updated
        )?;
        
        if !upload_confirmed {
            if args.verbose {
                println!("Skipping upload of {}", filenm);
            }
            continue;
        }
        
        // Step 3: Upload the secret using the interface
        let response = kv_client.set_secret_value(&secret_name, &content)
            .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to upload secret {}: {}", secret_name, e)))?;
        
        if args.verbose {
            println!("Successfully uploaded secret {} to Azure Key Vault", secret_name);
        }
        
        // Step 4: Fetch metadata and save to output
        let output_entry = json!({
            "filenm": filenm,
            "md5": md5,
            "ins_ts": ins_ts,
            "az_id": response.id,
            "az_create": response.created.to_string(),
            "az_updated": response.updated.to_string(),
            "az_name": response.name,
            "hostname": hostname,
            "encoding": encoding  // Add encoding info (utf8 or base64)
        });
        
        azure_secret_set_output.push(output_entry);
    }
    
    // Step 5: Append to output file if we have any entries
    if !azure_secret_set_output.is_empty() {
        // Create parent directory if it doesn't exist
        if let Some(parent) = output_filepath.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                Box::<dyn std::error::Error>::from(format!("Failed to create directory {}: {}", parent.display(), e))
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
            existing_entries.extend(azure_secret_set_output);
            
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
            serde_json::to_writer_pretty(&mut file, &azure_secret_set_output)?;
        }
        
        if args.verbose {
            println!("Successfully saved entries to {}", output_filepath.display());
        }
    } else if args.verbose {
        println!("No entries were processed successfully.");
    }
    
    Ok(())
}