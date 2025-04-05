use std::path::PathBuf;
use std::io::Read;

use podman_compose_mgr::interfaces::{MockAzureKeyVaultClient, MockReadInteractiveInputHelper};
use podman_compose_mgr::read_interactive_input::ReadValResult;
use podman_compose_mgr::secrets::models::SetSecretResponse;
use podman_compose_mgr::args::{Args, Mode};
use mockall::predicate::*;
use mockall::Sequence;
use serde_json::json;
use tempfile::NamedTempFile;
use time::OffsetDateTime;

/// We need a helper function to process uploads with a mock Azure client
/// 
/// This is an enhanced version of process_with_injected_dependencies that takes
/// a mocked Azure KeyVault client instead of creating one
fn process_with_injected_dependencies_for_testing<R: podman_compose_mgr::interfaces::ReadInteractiveInputHelper>(
    args: &Args,
    read_val_helper: &R,
    azure_client: Box<dyn podman_compose_mgr::interfaces::AzureKeyVaultClient>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get required parameters from args
    let input_filepath = args.input_json.as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Input JSON path is required"))?;
    let output_filepath = args.output_json.as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Output JSON path is required"))?;
    
    // Here we would normally create the KeyVault client, but we'll use the provided mock
    let kv_client = azure_client;
    
    // Test connection to Azure Key Vault
    if args.verbose {
        println!("Testing connection to Azure Key Vault...");
    }
    
    // Read input JSON file
    let mut file = std::fs::File::open(input_filepath)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    
    // Parse JSON as array
    let entries: Vec<serde_json::Value> = serde_json::from_str(&content)?;
    
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
            None => podman_compose_mgr::secrets::utils::get_hostname().unwrap_or_default()
        };
        let ins_ts = entry["ins_ts"].as_str().ok_or_else(|| 
            Box::<dyn std::error::Error>::from(format!("Missing ins_ts field in entry: {}", entry)))?;
        
        // Skip this entry if the file doesn't exist
        if !std::path::Path::new(filenm).exists() {
            eprintln!("File {} does not exist, skipping", filenm);
            continue;
        }
        
        // Create a secret name from the full path
        let secret_name = filenm.replace([std::path::MAIN_SEPARATOR, '.'], "-");
        
        // Replace spaces and other problematic characters with URL-encoding
        let mut encoded_name = String::new();
        for c in secret_name.chars() {
            if c.is_alphanumeric() || c == '-' {
                encoded_name.push(c);
            } else {
                for byte in c.to_string().as_bytes() {
                    encoded_name.push_str(&format!("-{:02X}", byte));
                }
            }
        }
        
        let secret_name = encoded_name;
        
        // Check if the secret already exists
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
        let content = std::fs::read_to_string(filenm)
            .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to read file {}: {}", filenm, e)))?;
            
        // Get file size in KiB
        let metadata = std::fs::metadata(filenm)
            .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to get metadata for {}: {}", filenm, e)))?;
        let size_bytes = metadata.len();
        let size_kib = size_bytes as f64 / 1024.0;
        
        // Prompt the user for confirmation using the injected helper
        let upload_confirmed = podman_compose_mgr::secrets::upload::prompt_for_upload_with_helper(
            filenm, &secret_name, size_kib, read_val_helper
        )?;
        
        if !upload_confirmed {
            if args.verbose {
                println!("Skipping upload of {}", filenm);
            }
            continue;
        }
        
        // Upload the secret
        let response = kv_client.set_secret_value(&secret_name, &content)
            .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to upload secret {}: {}", secret_name, e)))?;
        
        if args.verbose {
            println!("Successfully uploaded secret {} to Azure Key Vault", secret_name);
        }
        
        // Add to output entries
        let output_entry = serde_json::json!({
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
    
    // Write output if we have any entries
    if !azure_secret_set_output.is_empty() {
        // Create parent directory if it doesn't exist
        if let Some(parent) = output_filepath.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Box::<dyn std::error::Error>::from(format!("Failed to create directory {}: {}", parent.display(), e))
            })?;
        }
        
        // Check if the file already exists
        let file_exists = output_filepath.exists();
        
        if file_exists {
            // Read existing content to append properly
            let mut existing_file = std::fs::File::open(output_filepath)?;
            let mut existing_content = String::new();
            existing_file.read_to_string(&mut existing_content)?;
            
            let mut existing_entries: Vec<serde_json::Value> = if existing_content.trim().is_empty() {
                Vec::new()
            } else {
                serde_json::from_str(&existing_content)?
            };
            
            // Append new entries
            existing_entries.extend(azure_secret_set_output);
            
            // Write back as valid JSON array
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(output_filepath)?;
            
            serde_json::to_writer_pretty(&mut file, &existing_entries)?;
        } else {
            // Create new file with JSON array
            let mut file = std::fs::File::create(output_filepath)?;
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

#[test]
fn test_upload_process_with_varying_terminal_sizes() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create a temporary input JSON file with entries for our test files
    let input_json = create_test_input_json()?;
    let input_path = input_json.path().to_path_buf();
    
    // 2. Create a temporary output JSON file
    let output_json = NamedTempFile::new()?;
    let output_path = output_json.path().to_path_buf();

    // 3. Create a temporary file for the client secret
    let client_secret_file = NamedTempFile::new()?;
    let client_secret_path = client_secret_file.path().to_path_buf();
    
    // Write a dummy secret to the file
    std::fs::write(client_secret_file.path(), "test-client-secret")?;
    
    // Create Args for the process function
    let args = Args {
        mode: Mode::SecretUpload,
        path: PathBuf::from("."),
        input_json: Some(input_path.clone()),
        output_json: Some(output_path.clone()),
        secrets_client_id: Some("test-client-id".to_string()),
        secrets_client_secret_path: Some(client_secret_path),
        secrets_tenant_id: Some("test-tenant-id".to_string()),
        secrets_vault_name: Some("test-vault".to_string()),
        verbose: true,
        exclude_path_patterns: vec![],
        include_path_patterns: vec![],
        build_args: vec![],
        secrets_init_filepath: None,
    };
    
    // List of file paths in our test that will be processed
    let test_files = vec![
        "tests/test3_and_test4/a",
        "tests/test3_and_test4/b",
        "tests/test3_and_test4/c",
        "tests/test3_and_test4/d d",
    ];
    
    // Helper function to create a mock secret response
    let create_mock_response = |name: &str, value: &str| -> SetSecretResponse {
        let now = OffsetDateTime::now_utc();
        SetSecretResponse {
            created: now,
            updated: now,
            name: name.to_string(),
            id: format!("https://test-vault.vault.azure.net/secrets/{}", name),
            value: value.to_string(),
        }
    };
    
    // Setup mockall sequence to ensure interactions happen in the right order
    let mut seq = Sequence::new();
    
    // FIRST TEST: Terminal width 60, user approves all uploads
    {
        // Create a mock Azure KeyVault client
        let mut azure_client = MockAzureKeyVaultClient::new();
        
        // For each file, expect a get_secret_value call first to check if it exists
        // Then expect a set_secret_value call to upload it
        for file_path in &test_files {
            let encoded_name = file_path.replace([std::path::MAIN_SEPARATOR, '.'], "-")
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '-' {
                        c.to_string()
                    } else {
                        format!("-{:02X}", c as u8)
                    }
                })
                .collect::<String>();
            
            // First expect a check if the secret exists - return an error
            let encoded_name_clone = encoded_name.clone();
            azure_client
                .expect_get_secret_value()
                .with(eq(encoded_name.clone()))
                .times(1)
                .in_sequence(&mut seq)
                .returning(move |name| {
                    Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::NotFound, 
                        format!("Secret not found: {}", name)
                    )))
                });
            
            // Then expect the upload - which should succeed
            azure_client
                .expect_set_secret_value()
                .with(eq(encoded_name_clone), always())
                .times(1)
                .in_sequence(&mut seq)
                .returning(move |name, value| {
                    Ok(create_mock_response(name, value))
                });
        }
        
        // Create a mock ReadInteractiveInputHelper that always returns "Y" to approve uploads
        let mut read_val_helper = MockReadInteractiveInputHelper::new();
        
        // Set up the mock to return "Y" for all four files
        read_val_helper
            .expect_read_val_from_cmd_line_and_proceed()
            .times(4)
            .returning(|grammars, _size| {
                // Format the prompt for verification
                let mut grammars_copy = grammars.to_vec();
                let _ = podman_compose_mgr::read_interactive_input::do_prompt_formatting(
                    &mut grammars_copy, 
                    60
                );
                let formatted = podman_compose_mgr::read_interactive_input::unroll_grammar_into_string(
                    &grammars_copy,
                    false,
                    true
                );
                println!("width 60 prompt: {}", formatted);
                
                // Return "Y" to approve all uploads
                ReadValResult {
                    user_entered_val: Some("Y".to_string()),
                }
            });
            
        // Run the process function with our mock helpers
        let result = process_with_injected_dependencies_for_testing(
            &args, 
            &read_val_helper,
            Box::new(azure_client)
        );
        
        // Check that the test succeeded - all files should be uploaded
        assert!(result.is_ok(), "Test 1 failed: {:?}", result.err());
        println!("Test 1 succeeded - all files were uploaded!");
    }
    
    // SECOND TEST: Terminal width 40, user declines all uploads
    {
        // Create a mock Azure KeyVault client 
        let mut azure_client = MockAzureKeyVaultClient::new();
        
        // Set up expectations: for each file, we only expect the get_secret_value
        // call since the user will decline the upload
        for file_path in &test_files {
            let encoded_name = file_path.replace([std::path::MAIN_SEPARATOR, '.'], "-")
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '-' {
                        c.to_string()
                    } else {
                        format!("-{:02X}", c as u8)
                    }
                })
                .collect::<String>();
            
            // Expect a check if the secret exists
            azure_client
                .expect_get_secret_value()
                .with(eq(encoded_name.clone()))
                .times(1)
                .returning(|name| {
                    Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::NotFound, 
                        format!("Secret not found: {}", name)
                    )))
                });
            
            // We don't expect set_secret_value since the user will decline
        }
        
        // Create a mock ReadInteractiveInputHelper that always returns "n" to decline uploads
        let mut read_val_helper = MockReadInteractiveInputHelper::new();
        
        // Set up the mock to return "n" for all four files
        read_val_helper
            .expect_read_val_from_cmd_line_and_proceed()
            .times(4)
            .returning(|grammars, _size| {
                // Format the prompt for verification
                let mut grammars_copy = grammars.to_vec();
                let _ = podman_compose_mgr::read_interactive_input::do_prompt_formatting(
                    &mut grammars_copy, 
                    40
                );
                let formatted = podman_compose_mgr::read_interactive_input::unroll_grammar_into_string(
                    &grammars_copy,
                    false,
                    true
                );
                println!("width 40 prompt: {}", formatted);
                
                // Return "n" to decline all uploads
                ReadValResult {
                    user_entered_val: Some("n".to_string()),
                }
            });
            
        // Run the process function with our mock helpers
        let result = process_with_injected_dependencies_for_testing(
            &args, 
            &read_val_helper,
            Box::new(azure_client)
        );
        
        // Check that the test succeeded - no files should be uploaded
        assert!(result.is_ok(), "Test 2 failed: {:?}", result.err());
        println!("Test 2 succeeded - all uploads were declined!");
    }
    
    // Clean up
    drop(input_json);
    drop(output_json);
    drop(client_secret_file);
    
    Ok(())
}

/// Create a test input JSON file with the test files
fn create_test_input_json() -> Result<NamedTempFile, Box<dyn std::error::Error>> {
    // Create a temporary file
    let temp_file = NamedTempFile::new()?;
    
    // Create JSON content
    let json_content = json!([
        {"filenm": "tests/test3_and_test4/a", "md5": "60b725f10c9c85c70d97880dfe8191b3", "ins_ts": "2023-01-01T00:00:00Z", "hostname": "test-host"},
        {"filenm": "tests/test3_and_test4/b", "md5": "bfcc9da4f2e1d313c63cd0a4ee7604e9", "ins_ts": "2023-01-01T00:00:00Z", "hostname": "test-host"},
        {"filenm": "tests/test3_and_test4/c", "md5": "c576ec4297a7bdacc878e0061192441e", "ins_ts": "2023-01-01T00:00:00Z", "hostname": "test-host"},
        {"filenm": "tests/test3_and_test4/d d", "md5": "ef76b4f269b9a5104e4f061419a5f529", "ins_ts": "2023-01-01T00:00:00Z", "hostname": "test-host"}
    ]);
    
    // Write to the temporary file
    std::fs::write(temp_file.path(), json_content.to_string())?;
    
    Ok(temp_file)
}