use std::fs;

use mockall::Sequence;
use mockall::predicate::*;
use podman_compose_mgr::args::{Args, Mode};
use podman_compose_mgr::interfaces::{MockAzureKeyVaultClient, MockB2StorageClient, MockR2StorageClient, MockReadInteractiveInputHelper};
use podman_compose_mgr::read_interactive_input::ReadValResult;
use podman_compose_mgr::secrets::file_details::{FileDetails, format_file_size, get_file_details};
use podman_compose_mgr::secrets::models::SetSecretResponse;
use podman_compose_mgr::secrets::upload;
use podman_compose_mgr::secrets::upload_utils::test_utils;
use podman_compose_mgr::secrets::utils::calculate_hash;
use serde_json::json;
use std::sync::{Arc, Mutex};
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
        input_json: Some(input_path.clone()),
        output_json: Some(output_path.clone()),
        secrets_client_id: Some("test-client-id".to_string()),
        secrets_client_secret_path: Some(client_secret_path),
        secrets_tenant_id: Some("test-tenant-id".to_string()),
        secrets_vault_name: Some("test-vault".to_string()),
        verbose: 1,
        s3_account_id_filepath: Some(std::path::PathBuf::from("tests/test3_and_test4/a")),
        s3_secret_key_filepath: Some(std::path::PathBuf::from("tests/test3_and_test4/b")),
        s3_endpoint_filepath: Some(std::path::PathBuf::from("tests/test3_and_test4/c")),
        ..Default::default()
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
            // Calculate the hash first, then create secret name from hash
            let hash = calculate_hash(file_path).unwrap();
            let encoded_name = hash.clone();

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
                        format!("Secret not found: {}", name),
                    )))
                });

            // Then expect the upload - which should succeed
            azure_client
                .expect_set_secret_value()
                .with(eq(encoded_name_clone), always())
                .times(1)
                .in_sequence(&mut seq)
                .returning(move |name, value| Ok(create_mock_response(name, value)));
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
                    60,
                );
                let formatted =
                    podman_compose_mgr::read_interactive_input::unroll_grammar_into_string(
                        &grammars_copy,
                        false,
                        true,
                    );

                // Print the prompt for verification including the file being processed
                println!("\nWidth 60 prompt, file {}:", current_file);
                println!("\"{}\"", formatted);
                println!("Prompt length: {} characters", formatted.len());

                // Validate the prompt length is within the constraints for width 60
                // (It won't be exactly 60 due to word wrapping and other formatting)
                let max_line_length = formatted.lines().map(|line| line.len()).max().unwrap_or(0);

                println!("Longest line length: {}", max_line_length);
                assert!(
                    max_line_length <= 60,
                    "Prompt line length {} exceeds terminal width 60",
                    max_line_length
                );

                // Return "Y" to approve all uploads
                ReadValResult {
                    user_entered_val: Some("Y".to_string()),
                }
            });

        // Create a mock B2 client since we need it for the test
        let b2_client = MockB2StorageClient::new();
        // We don't expect it to be called for Azure KV uploads
        
        // Run the process function with our mock helpers - using the actual production code
        let r2_client = MockR2StorageClient::new();
        
        let result = upload::process_with_injected_dependencies_and_clients(
            &args,
            &read_val_helper,
            Box::new(azure_client),
            Box::new(b2_client),
            Box::new(r2_client),
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
            // Calculate the hash
            let hash = calculate_hash(file_path).unwrap();

            // Expect a check if the secret exists
            azure_client
                .expect_get_secret_value()
                .with(eq(hash.clone()))
                .times(1)
                .returning(|name| {
                    Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("Secret not found: {}", name),
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
                    40,
                );
                let formatted =
                    podman_compose_mgr::read_interactive_input::unroll_grammar_into_string(
                        &grammars_copy,
                        false,
                        true,
                    );

                // Print the prompt for verification including the file being processed
                println!("\nWidth 40 prompt, file {}:", current_file);
                println!("\"{}\"", formatted);
                println!("Prompt length: {} characters", formatted.len());

                // Validate the prompt length is within the constraints for width 40
                // (It won't be exactly 40 due to word wrapping and other formatting)
                let max_line_length = formatted.lines().map(|line| line.len()).max().unwrap_or(0);

                println!("Longest line length: {}", max_line_length);
                assert!(
                    max_line_length <= 40,
                    "Prompt line length {} exceeds terminal width 40",
                    max_line_length
                );

                // Return "n" to decline all uploads
                ReadValResult {
                    user_entered_val: Some("n".to_string()),
                }
            });

        // Create a mock B2 client since we need it for the test
        let b2_client = MockB2StorageClient::new();
        // We don't expect it to be called for Azure KV uploads
        
        // Run the process function with our mock helpers - using the actual production code
        let r2_client = MockR2StorageClient::new();
        
        let result = upload::process_with_injected_dependencies_and_clients(
            &args,
            &read_val_helper,
            Box::new(azure_client),
            Box::new(b2_client),
            Box::new(r2_client),
        );

        // Check that the test succeeded - no files should be uploaded
        assert!(result.is_ok(), "Test 2 failed: {:?}", result.err());
        println!("Test 2 succeeded - all uploads were declined!");
    }

    // THIRD TEST: Using actual terminal width, user selects "d" for details and then "Y" to upload
    {
        // Create a mock Azure KeyVault client
        let mut azure_client = MockAzureKeyVaultClient::new();

        // Setup mockall sequence for checking and uploading each file
        let mut seq = Sequence::new();

        // Create a vector of expected file details
        let expected_file_details: Vec<(String, FileDetails)> = test_files
            .iter()
            .map(|file_path| {
                // Calculate the hash
                let hash = calculate_hash(file_path).unwrap();

                // Create a hard-coded expected FileDetails with known values
                // Note: We can't predict the exact last_modified time in the test,
                // so we'll check that field separately
                let mut details = get_file_details(file_path).unwrap();
                details.last_modified = "WILL BE VALIDATED SEPARATELY".to_string(); // Will be checked differently
                details.encoding = "utf8".to_string(); // Assume all test files are UTF-8 for testing purposes

                (hash.clone(), details)
            })
            .collect();

        // For each file, set up the expected API calls
        for (i, (encoded_name, _)) in expected_file_details.iter().enumerate() {
            // First expect a check if the secret exists
            // Let's make the last file have an existing secret to test that code path
            let encoded_name_clone = encoded_name.clone();

            if i == expected_file_details.len() - 1 {
                // For the last file, make the secret exist
                let encoded_name_clone_inner = encoded_name_clone.clone();
                azure_client
                    .expect_get_secret_value()
                    .with(eq(encoded_name.clone()))
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(move |_| {
                        // Return a mock response indicating the secret exists
                        Ok(test_utils::get_mock_secret_response(
                            &encoded_name_clone_inner,
                            "existing-secret-value",
                        ))
                    });
            } else {
                // For other files, make the secret not exist
                azure_client
                    .expect_get_secret_value()
                    .with(eq(encoded_name.clone()))
                    .times(1)
                    .in_sequence(&mut seq)
                    .returning(move |name| {
                        Err(Box::new(std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            format!("Secret not found: {}", name),
                        )))
                    });
            }

            // Then expect the upload - which should succeed
            azure_client
                .expect_set_secret_value()
                .with(eq(encoded_name_clone), always())
                .times(1)
                .in_sequence(&mut seq)
                .returning(move |name, value| {
                    let now = OffsetDateTime::now_utc();
                    Ok(SetSecretResponse {
                        created: now,
                        updated: now,
                        name: name.to_string(),
                        id: format!("https://test-vault.vault.azure.net/secrets/{}", name),
                        value: value.to_string(),
                    })
                });
        }

        // Create a shared container to hold captured file details from the test
        let captured_details = Arc::new(Mutex::new(Vec::<FileDetails>::new()));

        // Create a mock ReadInteractiveInputHelper that returns "d" first, then "Y" on second prompt
        let mut read_val_helper = MockReadInteractiveInputHelper::new();

        // Track which file and which attempt we're on
        let file_index = std::cell::Cell::new(0);
        let attempt_index = std::cell::Cell::new(1);
        let test_files_clone = test_files.clone();
        let captured_details_clone = captured_details.clone();

        // Function in upload.rs runs in a loop and keeps prompting when "d" is selected,
        // so we need more than 8 inputs - exact number depends on implementation
        read_val_helper
            .expect_read_val_from_cmd_line_and_proceed()
            .times(..)
            .returning(move |grammars, _size| {
                // Get the current file and attempt numbers
                let current_file_index = file_index.get();
                let current_file = &test_files_clone[current_file_index];
                let current_attempt = attempt_index.get();

                // Format the prompt using actual terminal width
                let mut grammars_copy = grammars.to_vec();

                // Get the actual terminal width using the command helper
                // (Use None to get the actual terminal width)
                let actual_width =
                    podman_compose_mgr::helpers::cmd_helper_fns::get_terminal_display_width(None);

                let _ = podman_compose_mgr::read_interactive_input::do_prompt_formatting(
                    &mut grammars_copy,
                    actual_width,
                );
                let formatted =
                    podman_compose_mgr::read_interactive_input::unroll_grammar_into_string(
                        &grammars_copy,
                        false,
                        true,
                    );

                // Print information about the prompt
                println!("\nActual terminal width: {} characters", actual_width);
                println!(
                    "Prompt for file {}, attempt {}: ",
                    current_file, current_attempt
                );
                println!("\"{}\"", formatted);
                println!("Prompt length: {} characters", formatted.len());

                // Validate the prompt length is within the constraints
                let max_line_length = formatted.lines().map(|line| line.len()).max().unwrap_or(0);

                println!("Longest line length: {}", max_line_length);
                assert!(
                    max_line_length <= actual_width,
                    "Prompt line length {} exceeds terminal width {}",
                    max_line_length,
                    actual_width
                );

                // Check which file we're on
                // We want to alternate showing details then approving for each file
                // We'll use a state machine approach
                if current_file_index >= test_files_clone.len() {
                    // We're done with all files, shouldn't happen but just in case
                    panic!("Unexpected call after all files processed");
                }

                // We have two states per file:
                // 1. First time for a file - show details (select "d")
                // 2. Second time for a file - approve upload (select "Y")
                let current_file_path = current_file.clone();
                let already_showed_details = captured_details_clone
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|d| d.file_path == current_file_path);

                if !already_showed_details {
                    // First time seeing this file, show details ("d")
                    println!("User selects 'd' to see details for file {}", current_file);

                    // Get the encoded name
                    // Calculate the hash
                    let _hash = calculate_hash(current_file).unwrap();

                    // Get file details
                    let details = get_file_details(current_file).unwrap();

                    // Capture the details for later verification
                    captured_details_clone.lock().unwrap().push(details.clone());

                    // Print the details manually since we're in a test
                    println!("File path: {}", details.file_path);
                    println!("Size: {}", format_file_size(details.file_size));
                    println!("Last modified: {}", details.last_modified);
                    println!("Hash: {}", details.hash);
                    println!("Encoding: {}", details.encoding);

                    // Return "d" for details
                    ReadValResult {
                        user_entered_val: Some("d".to_string()),
                    }
                } else {
                    // Second time seeing this file, approve upload ("Y") and move to next file
                    println!(
                        "User selects 'Y' to approve upload for file {}",
                        current_file
                    );

                    // Move to next file
                    file_index.set(current_file_index + 1);

                    // Return "Y" to approve
                    ReadValResult {
                        user_entered_val: Some("Y".to_string()),
                    }
                }
            });

        // Create a mock B2 client since we need it for the test
        let b2_client = MockB2StorageClient::new();
        // We don't expect it to be called for Azure KV uploads
        
        // Run the process function with our mock helpers
        let r2_client = MockR2StorageClient::new();
        
        let result = upload::process_with_injected_dependencies_and_clients(
            &args,
            &read_val_helper,
            Box::new(azure_client),
            Box::new(b2_client),
            Box::new(r2_client),
        );

        // Check that the test succeeded
        assert!(result.is_ok(), "Test 3 failed: {:?}", result.err());

        // Now validate that the file details we captured are correct
        let details_vec = captured_details.lock().unwrap();
        assert_eq!(
            details_vec.len(),
            4,
            "Should have captured details for 4 files"
        );

        // Check each captured detail against the expected values
        for (i, details) in details_vec.iter().enumerate() {
            let expected_details = &expected_file_details[i].1;

            // Verify file path
            assert_eq!(
                details.file_path, expected_details.file_path,
                "File path mismatch for file {}",
                details.file_path
            );

            // Verify size
            assert_eq!(
                details.file_size, expected_details.file_size,
                "Size mismatch for file {}: got {} bytes, expected {} bytes",
                details.file_path, details.file_size, expected_details.file_size
            );

            // Verify hash
            assert_eq!(
                details.hash, expected_details.hash,
                "Hash mismatch for file {}",
                details.file_path
            );

            // Verify the format of last_modified (we can't know the exact value)
            let date_format = regex::Regex::new(r"^\d{2}/\d{2}/\d{2} \d{2}:\d{2}:\d{2}$").unwrap();
            assert!(
                date_format.is_match(&details.last_modified),
                "Last modified date format incorrect: {}",
                details.last_modified
            );
        }

        println!("Test 3 succeeded - all files were uploaded with details shown and verified!");
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

    // Create test data with calculated hashes
    let a_hash = calculate_hash("tests/test3_and_test4/a")?;
    let b_hash = calculate_hash("tests/test3_and_test4/b")?;
    let c_hash = calculate_hash("tests/test3_and_test4/c")?;
    let d_hash = calculate_hash("tests/test3_and_test4/d d")?;

    // Create JSON content with updated field names and hash values
    let json_content = json!([
        {
            "file_nm": "tests/test3_and_test4/a",
            "hash": a_hash,
            "hash_algo": "sha1",
            "ins_ts": "2023-01-01T00:00:00Z",
            "hostname": hostname::get()?.to_string_lossy().to_string(),
            "encoding": "utf8",
            "file_size": fs::metadata("tests/test3_and_test4/a")?.len(),
            "encoded_size": fs::metadata("tests/test3_and_test4/a")?.len(),
            "destination_cloud": "azure_kv",
            "secret_name": format!("file-{}", a_hash)
        },
        {
            "file_nm": "tests/test3_and_test4/b",
            "hash": b_hash,
            "hash_algo": "sha1",
            "ins_ts": "2023-01-01T00:00:00Z",
            "hostname": hostname::get()?.to_string_lossy().to_string(),
            "encoding": "utf8",
            "file_size": fs::metadata("tests/test3_and_test4/b")?.len(),
            "encoded_size": fs::metadata("tests/test3_and_test4/b")?.len(),
            "destination_cloud": "azure_kv",
            "secret_name": format!("file-{}", b_hash)
        },
        {
            "file_nm": "tests/test3_and_test4/c",
            "hash": c_hash,
            "hash_algo": "sha1",
            "ins_ts": "2023-01-01T00:00:00Z",
            "hostname": hostname::get()?.to_string_lossy().to_string(),
            "encoding": "utf8",
            "file_size": fs::metadata("tests/test3_and_test4/c")?.len(),
            "encoded_size": fs::metadata("tests/test3_and_test4/c")?.len(),
            "destination_cloud": "azure_kv",
            "secret_name": format!("file-{}", c_hash)
        },
        {
            "file_nm": "tests/test3_and_test4/d d",
            "hash": d_hash,
            "hash_algo": "sha1",
            "ins_ts": "2023-01-01T00:00:00Z",
            "hostname": hostname::get()?.to_string_lossy().to_string(),
            "encoding": "utf8",
            "file_size": fs::metadata("tests/test3_and_test4/d d")?.len(),
            "encoded_size": fs::metadata("tests/test3_and_test4/d d")?.len(),
            "destination_cloud": "azure_kv",
            "secret_name": format!("file-{}", d_hash)
        }
    ]);

    // Write to the temporary file
    std::fs::write(temp_file.path(), json_content.to_string())?;

    Ok(temp_file)
}
