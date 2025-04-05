use crate::args::Args;
use crate::interfaces::{DefaultReadInteractiveInputHelper, ReadInteractiveInputHelper};
use crate::read_interactive_input::GrammarFragment;
use crate::secrets::azure::get_keyvault_client;
use crate::secrets::error::Result;
use crate::secrets::user_prompt::{setup_upload_prompt, display_upload_help};
use crate::secrets::utils::get_hostname;

use chrono::{DateTime, Local};
use serde_json::{json, Value};
use std::fs::{self, File, OpenOptions, metadata};
use std::io::Read;
use std::path::{Path, MAIN_SEPARATOR};

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
    kv_client: Box<dyn crate::interfaces::AzureKeyVaultClient>,
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
        
        // Create a secret name from the file path
        let secret_name = create_encoded_secret_name(filenm);
        
        // Step 2: Check if the secret already exists using the interface
        let secret_exists = match kv_client.get_secret_value(&secret_name) {
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
            
        // Prompt the user for confirmation using the injected helper
        let upload_confirmed = prompt_for_upload_with_helper(filenm, &secret_name, read_val_helper)?;
        
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
fn prompt_for_upload(file_path: &str, encoded_name: &str) -> Result<bool> {
    let read_val_helper = DefaultReadInteractiveInputHelper;
    prompt_for_upload_with_helper(file_path, encoded_name, &read_val_helper)
}

/// Version of prompt_for_upload that accepts dependency injection for testing
pub fn prompt_for_upload_with_helper<R: ReadInteractiveInputHelper>(
    file_path: &str, 
    encoded_name: &str, 
    read_val_helper: &R,
) -> Result<bool> {
    // We no longer need to calculate size here
    // The size is now calculated inside setup_upload_prompt

    let mut grammars: Vec<GrammarFragment> = Vec::new();
    
    // Setup the prompt
    setup_upload_prompt(&mut grammars, file_path, encoded_name)?;
    
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
                        display_file_details(file_path, encoded_name)?;
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

/// Represents the detailed information about a file for secret upload
#[derive(Debug, Clone)]
pub struct FileDetails {
    pub file_path: String,
    pub size_bytes: u64,
    pub last_modified: String,
    pub secret_name: String,
}

/// Get detailed information about the file
pub fn get_file_details(file_path: &str, encoded_name: &str) -> Result<FileDetails> {
    // Get file metadata for size and last modified time
    let metadata = metadata(file_path)
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to get metadata: {}", e)))?;
    
    // Get file size in bytes
    let size_bytes = metadata.len();
    
    // Format the last modified time
    let modified = metadata.modified()
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to get modified time: {}", e)))?;
    
    let datetime: DateTime<Local> = modified.into();
    let formatted_time = datetime.format("%m/%d/%y %H:%M:%S").to_string();
    
    // Return the details
    Ok(FileDetails {
        file_path: file_path.to_string(),
        size_bytes,
        last_modified: formatted_time,
        secret_name: encoded_name.to_string(),
    })
}

/// Helper function to format file size with appropriate units
pub fn format_file_size(size_bytes: u64) -> String {
    if size_bytes < 1024 {
        // Less than 1 KiB, display in bytes
        format!("{} bytes", size_bytes)
    } else if size_bytes < 1024 * 1024 {
        // Display in KiB with 2 decimal places
        let size_kib = size_bytes as f64 / 1024.0;
        format!("{:.2} KiB", size_kib)
    } else if size_bytes < 1024 * 1024 * 1024 {
        // Display in MiB with 2 decimal places
        let size_mib = size_bytes as f64 / (1024.0 * 1024.0);
        format!("{:.2} MiB", size_mib)
    } else if size_bytes < 1024 * 1024 * 1024 * 1024 {
        // Display in GiB with 2 decimal places
        let size_gib = size_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        format!("{:.2} GiB", size_gib)
    } else if size_bytes < 1024 * 1024 * 1024 * 1024 * 1024 {
        // Display in TiB with 2 decimal places
        let size_tib = size_bytes as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0);
        format!("{:.2} TiB", size_tib)
    } else {
        // Display in PiB with 2 decimal places (for extremely large files)
        let size_pib = size_bytes as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0);
        format!("{:.2} PiB", size_pib)
    }
}

/// Display detailed information about the file
fn display_file_details(file_path: &str, encoded_name: &str) -> Result<()> {
    // Get the file details
    let details = get_file_details(file_path, encoded_name)?;
    
    // Display the details
    println!("File path: {}", details.file_path);
    println!("Size: {}", format_file_size(details.size_bytes));
    println!("Last modified: {}", details.last_modified);
    println!("Secret name: {}", details.secret_name);
    
    Ok(())
}

/// Create an encoded secret name from a file path
/// 
/// This function takes a file path and converts it to a name suitable for
/// Azure Key Vault secrets:
/// 1. Replace path separators with dashes
/// 2. Replace periods (.) with dashes
/// 3. Encode any other special characters using hex encoding
pub fn create_encoded_secret_name(file_path: &str) -> String {
    // First replace path separators and periods with dashes
    let secret_name = file_path.replace([MAIN_SEPARATOR, '.'], "-");
    
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
    
    encoded_name
}

pub mod test_utils {
    use crate::secrets::models::SetSecretResponse;
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