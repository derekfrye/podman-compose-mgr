use crate::args::Args;
use crate::interfaces::{DefaultReadInteractiveInputHelper, ReadInteractiveInputHelper};
use crate::read_interactive_input::GrammarFragment;
use crate::secrets::azure::{get_keyvault_client, get_secret_value, set_secret_value};
use crate::secrets::error::Result;
use crate::secrets::user_prompt::{setup_upload_prompt, display_upload_help};
use crate::secrets::utils::get_hostname;

use chrono::{DateTime, Local};
use serde_json::{json, Value};
use std::fs::{self, File, OpenOptions, metadata};
use std::io::Read;
use std::path::{Path, MAIN_SEPARATOR};
use tokio::runtime::Runtime;

/// Process the upload operation to Azure Key Vault
pub fn process(args: &Args) -> Result<()> {
    // Get required parameters from args
    let input_filepath = args.secret_mode_input_json.as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Input JSON path is required"))?;
    let output_filepath = args.output_json.as_ref()
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
    
    // Create runtime for async operations
    let rt = Runtime::new()?;
    
    // Test connection to Azure Key Vault
    if args.verbose {
        println!("Testing connection to Azure Key Vault...");
    }
    
    // Read input JSON file
    let mut file = File::open(input_filepath)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    
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
        if !Path::new(filenm).exists() {
            eprintln!("File {} does not exist, skipping", filenm);
            continue;
        }
        
        // Create a secret name from the full path
        // 1. Replace path separators with dashes
        // 2. Replace periods (.) with dashes
        // 3. Encode remaining special characters
        let secret_name = filenm.replace([MAIN_SEPARATOR, '.'], "-");
        
        // Replace spaces and other problematic characters with URL-encoding
        // Note: In Azure Key Vault, secret names can only contain alphanumeric characters and dashes
        let mut encoded_name = String::new();
        for c in secret_name.chars() {
            if c.is_alphanumeric() || c == '-' {
                encoded_name.push(c);
            } else {
                // For space and other special characters, convert to percent encoding
                // but use their hex value directly
                for byte in c.to_string().as_bytes() {
                    encoded_name.push_str(&format!("-{:02X}", byte));
                }
            }
        }
        
        let secret_name = encoded_name;
        
        // Step 2: Check if the secret already exists
        let secret_exists = match rt.block_on(get_secret_value(&secret_name, &kv_client)) {
            Ok(_) => {
                eprintln!("Secret {} already exists in the key vault, skipping", secret_name);
                true
            },
            Err(_) => false,
        };
        
        if secret_exists {
            continue;
        }
        
        // Read file content
        let content = fs::read_to_string(filenm)
            .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to read file {}: {}", filenm, e)))?;
            
        // Get file size in KiB
        let metadata = metadata(filenm)
            .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to get metadata for {}: {}", filenm, e)))?;
        let size_bytes = metadata.len();
        let size_kib = size_bytes as f64 / 1024.0;
        
        // Prompt the user for confirmation, using default implementation
        let read_val_helper = DefaultReadInteractiveInputHelper;
        let upload_confirmed = prompt_for_upload_with_helper(filenm, &secret_name, size_kib, &read_val_helper)?;
        
        if !upload_confirmed {
            if args.verbose {
                println!("Skipping upload of {}", filenm);
            }
            continue;
        }
        
        // Step 3: Upload the secret
        let response = rt.block_on(set_secret_value(&secret_name, &kv_client, &content))
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
            "hostname": hostname
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

/// Prompt the user for confirmation before uploading a file
/// 
/// This function uses the default ReadInteractiveInputHelper
#[allow(dead_code)]
fn prompt_for_upload(file_path: &str, encoded_name: &str, size_kib: f64) -> Result<bool> {
    let read_val_helper = DefaultReadInteractiveInputHelper;
    prompt_for_upload_with_helper(file_path, encoded_name, size_kib, &read_val_helper)
}

/// Version of prompt_for_upload that accepts dependency injection for testing
pub fn prompt_for_upload_with_helper<R: ReadInteractiveInputHelper>(
    file_path: &str, 
    encoded_name: &str, 
    size_kib: f64,
    read_val_helper: &R,
) -> Result<bool> {
    let mut grammars: Vec<GrammarFragment> = Vec::new();
    
    // Setup the prompt
    setup_upload_prompt(&mut grammars, file_path, size_kib, encoded_name)?;
    
    loop {
        // Display prompt and get user input using the provided helper
        let result = read_val_helper.read_val_from_cmd_line_and_proceed(&mut grammars, None);
        
        match result.user_entered_val {
            None => return Ok(false), // Empty input means no
            Some(choice) => {
                match choice.as_str() {
                    // Yes, upload the file
                    "Y" => {
                        return Ok(true);
                    },
                    // No, skip this file
                    "n" => {
                        return Ok(false);
                    },
                    // Display details about the file
                    "d" => {
                        display_file_details(file_path, size_kib, encoded_name)?;
                    },
                    // Display help
                    "?" => {
                        display_upload_help();
                    },
                    // Invalid choice
                    _ => {
                        eprintln!("Invalid choice: {}", choice);
                    }
                }
            }
        }
    }
}

/// Display detailed information about the file
fn display_file_details(file_path: &str, size_kib: f64, encoded_name: &str) -> Result<()> {
    // Get file metadata for the last modified time
    let metadata = metadata(file_path)
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to get metadata: {}", e)))?;
    
    // Format the last modified time
    let modified = metadata.modified()
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to get modified time: {}", e)))?;
    
    let datetime: DateTime<Local> = modified.into();
    let formatted_time = datetime.format("%m/%d/%y %H:%M:%S").to_string();
    
    // Display the details
    println!("File path: {}", file_path);
    println!("Size: {:.2} KiB", size_kib);
    println!("Last modified: {}", formatted_time);
    println!("Secret name: {}", encoded_name);
    
    Ok(())
}

#[cfg(test)]
pub mod test_utils {
    use crate::secrets::models::SetSecretResponse;

    // use super::*;
    // use azure_security_keyvault::KeyvaultClient;
    use time::OffsetDateTime;

    // For testing, we can override the get_secret_value and set_secret_value functions
    // This function is used to get a mock SecretResponse for testing
    pub fn get_mock_secret_response(name: &str, value: &str) -> SetSecretResponse {
        let now = OffsetDateTime::now_utc();
        SetSecretResponse {
            created: now,
            updated: now,
            name: name.to_string(),
            id: format!("https://keyvault.vault.azure.net/secrets/{}", name),
            value: value.to_string(),
        }
    }
}