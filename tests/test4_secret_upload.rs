use std::path::PathBuf;

use podman_compose_mgr::interfaces::{MockAzureKeyVaultClient, MockReadInteractiveInputHelper};
use podman_compose_mgr::read_interactive_input::ReadValResult;
use podman_compose_mgr::secrets::models::SetSecretResponse;
use podman_compose_mgr::args::{Args, Mode};
use podman_compose_mgr::secrets::upload;
use mockall::predicate::*;
use mockall::Sequence;
use serde_json::json;
use tempfile::NamedTempFile;
use time::OffsetDateTime;

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
    // Use string values that can be cloned
    let test_files = vec![
        "tests/test3_and_test4/a".to_string(),
        "tests/test3_and_test4/b".to_string(),
        "tests/test3_and_test4/c".to_string(),
        "tests/test3_and_test4/d d".to_string(),
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
        
        // Track which files we're processing
        let file_index = std::cell::Cell::new(0);
        let test_files_clone = test_files.clone();
        
        // Set up the mock to return "Y" for all four files
        read_val_helper
            .expect_read_val_from_cmd_line_and_proceed()
            .times(4)
            .returning(move |grammars, _size| {
                // Get the current file being processed
                let current_index = file_index.get();
                let current_file = &test_files_clone[current_index];
                file_index.set(current_index + 1);
                
                // Format the prompt for verification with width 60
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
                
                // Print the prompt for verification including the file being processed
                println!("\nWidth 60 prompt, file {}:", current_file);
                println!("\"{}\"", formatted);
                println!("Prompt length: {} characters", formatted.len());
                
                // Validate the prompt length is within the constraints for width 60
                // (It won't be exactly 60 due to word wrapping and other formatting)
                let max_line_length = formatted.lines()
                    .map(|line| line.len())
                    .max()
                    .unwrap_or(0);
                    
                println!("Longest line length: {}", max_line_length);
                assert!(max_line_length <= 60, 
                    "Prompt line length {} exceeds terminal width 60", max_line_length);
                
                // Return "Y" to approve all uploads
                ReadValResult {
                    user_entered_val: Some("Y".to_string()),
                }
            });
            
        // Run the process function with our mock helpers - using the actual production code
        let result = upload::process_with_injected_dependencies_and_client(
            &args, 
            &read_val_helper,
            Box::new(azure_client)
        );
        
        // Check that the test succeeded - all files should be uploaded
        assert!(result.is_ok(), "Test 1 failed: {:?}", result.err());
        println!("");
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
        
        // Track which files we're processing
        let file_index = std::cell::Cell::new(0);
        let test_files_clone = test_files.clone();
        
        // Set up the mock to return "n" for all four files
        read_val_helper
            .expect_read_val_from_cmd_line_and_proceed()
            .times(4)
            .returning(move |grammars, _size| {
                // Get the current file being processed
                let current_index = file_index.get();
                let current_file = &test_files_clone[current_index];
                file_index.set(current_index + 1);
                
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
                
                // Print the prompt for verification including the file being processed
                println!("\nWidth 40 prompt, file {}:", current_file);
                println!("\"{}\"", formatted);
                println!("Prompt length: {} characters", formatted.len());
                
                // Validate the prompt length is within the constraints for width 40
                // (It won't be exactly 40 due to word wrapping and other formatting)
                let max_line_length = formatted.lines()
                    .map(|line| line.len())
                    .max()
                    .unwrap_or(0);
                    
                println!("Longest line length: {}", max_line_length);
                assert!(max_line_length <= 40, 
                    "Prompt line length {} exceeds terminal width 40", max_line_length);
                
                // Return "n" to decline all uploads
                ReadValResult {
                    user_entered_val: Some("n".to_string()),
                }
            });
            
        // Run the process function with our mock helpers - using the actual production code
        let result = upload::process_with_injected_dependencies_and_client(
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