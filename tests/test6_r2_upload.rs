use std::fs;
use mockall::predicate as testing;

use podman_compose_mgr::args::{Args, Mode};
use podman_compose_mgr::interfaces::{MockAzureKeyVaultClient, MockB2StorageClient, MockR2StorageClient, MockReadInteractiveInputHelper};
use podman_compose_mgr::read_interactive_input::ReadValResult;
use podman_compose_mgr::secrets::r2_storage::R2UploadResult;
use podman_compose_mgr::secrets::upload;
use podman_compose_mgr::secrets::utils::calculate_hash;
use serde_json::json;
use tempfile::NamedTempFile;

#[test]
fn test_r2_upload_process() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create a temporary input JSON file with entries for our test files
    let input_json = create_test_input_json()?;
    let input_path = input_json.path().to_path_buf();

    // 2. Create a temporary output JSON file
    let output_json = NamedTempFile::new()?;
    let output_path = output_json.path().to_path_buf();

    // 3. Create a temporary file for the client secret (needed for args even though we're mocking)
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
        r2_account_id: Some("test-cloudflare-account-id".to_string()),
        r2_access_key_id: Some("test-r2-access-key-id".to_string()),
        r2_access_key: Some("test-r2-access-key".to_string()),
        ..Default::default()
    };

    // List of file paths in our test that will be processed
    let test_files = vec![
        "tests/test3_and_test4/a".to_string(),
        "tests/test3_and_test4/b".to_string(),
        "tests/test3_and_test4/c".to_string(),
        "tests/test3_and_test4/d d".to_string(),
    ];

    // Test with R2 upload path
    {
        // Create mock clients
        let azure_client = MockAzureKeyVaultClient::new();
        let b2_client = MockB2StorageClient::new();

        // Create a mock ReadInteractiveInputHelper that always returns "Y" to approve uploads
        let mut read_val_helper = MockReadInteractiveInputHelper::new();

        // Track which files we're processing
        let file_index = std::cell::Cell::new(0);

        // Set up the mock to return "Y" for all four files
        read_val_helper
            .expect_read_val_from_cmd_line_and_proceed()
            .times(4)
            .returning(move |_, _| {
                // Get the current file being processed and increment counter
                let current_index = file_index.get();
                file_index.set(current_index + 1);
                
                // Return "Y" to approve all uploads
                ReadValResult {
                    user_entered_val: Some("Y".to_string()),
                }
            });
            
        // Create mock R2 storage client
        let mut r2_client = MockR2StorageClient::new();
        
        // Create expected R2UploadResults for each file
        let expected_results: Vec<(String, R2UploadResult)> = test_files
            .iter()
            .map(|file_path| {
                let hash = calculate_hash(file_path).unwrap();
                let r2_result = R2UploadResult {
                    hash: format!("test-etag-{}", hash),
                    id: format!("test-id-{}", hash),
                    bucket_id: "test-r2-bucket".to_string(),
                    name: format!("secrets/{}", hash),
                    created: "2023-01-01T00:00:00Z".to_string(),
                    updated: "2023-01-01T00:00:00Z".to_string(),
                };
                (file_path.clone(), r2_result)
            })
            .collect();
        
        // Set up expectations for check_file_exists_with_details for each file
        for file_path in test_files.iter() {
            let hash = calculate_hash(file_path).unwrap();
            r2_client
                .expect_check_file_exists_with_details()
                .with(
                    testing::eq(hash.clone()),
                    testing::eq(Some("test-r2-upload-bucket".to_string()))
                )
                .times(1)
                .returning(|_, _| {
                    Ok(Some((false, "".to_string(), "".to_string())))
                });
        }
        
        // Set up expectations for upload_file_with_details for each file
        for (file_path, r2_result) in expected_results {
            let file_path_clone = file_path.clone();
            r2_client
                .expect_upload_file_with_details()
                .withf(move |details| details.file_path == file_path_clone)
                .times(1)
                .returning(move |_| {
                    Ok(R2UploadResult {
                        hash: r2_result.hash.clone(),
                        id: r2_result.id.clone(),
                        bucket_id: r2_result.bucket_id.clone(),
                        name: r2_result.name.clone(),
                        created: r2_result.created.clone(),
                        updated: r2_result.updated.clone(),
                    })
                });
        }
        
        // Run the process function with our mock clients
        let result = upload::process_with_injected_dependencies_and_clients(
            &args,
            &read_val_helper,
            Box::new(azure_client),
            Box::new(b2_client),
            Box::new(r2_client),
        );

        // Check that the test succeeded
        assert!(result.is_ok(), "R2 upload test failed: {:?}", result.err());

        // Check the output JSON file to verify the uploads were processed correctly
        let output_content = fs::read_to_string(output_path.clone())?;
        let output_entries: Vec<serde_json::Value> = serde_json::from_str(&output_content)?;
        
        // Verify we have 4 entries
        assert_eq!(output_entries.len(), 4, "Expected 4 entries in output JSON");
        
        // Verify each entry has the correct R2-specific fields
        for (i, entry) in output_entries.iter().enumerate() {
            let file_path = &test_files[i];
            let hash = calculate_hash(file_path)?;
            
            // R2-specific fields
            assert_eq!(entry["destination_cloud"].as_str().unwrap(), "r2");
            assert_eq!(entry["cloud_id"].as_str().unwrap(), format!("test-id-{}", hash));
            assert_eq!(entry["r2_hash"].as_str().unwrap(), format!("test-etag-{}", hash));
            assert_eq!(entry["r2_bucket_id"].as_str().unwrap(), "test-r2-bucket");
            assert_eq!(entry["r2_name"].as_str().unwrap(), format!("secrets/{}", hash));
            
            // Common fields
            assert_eq!(entry["file_nm"].as_str().unwrap(), file_path);
            assert_eq!(entry["hash"].as_str().unwrap(), hash);
            assert_eq!(entry["cloud_upload_bucket"].as_str().unwrap(), "test-r2-upload-bucket");
        }
        
        println!("R2 upload test succeeded!");
    }

    // Clean up
    drop(input_json);
    drop(output_json);
    drop(client_secret_file);

    Ok(())
}

/// Create a test input JSON file with R2 destination_cloud
fn create_test_input_json() -> Result<NamedTempFile, Box<dyn std::error::Error>> {
    // Create a temporary file
    let temp_file = NamedTempFile::new()?;

    // Create test data with calculated hashes
    let a_hash = calculate_hash("tests/test3_and_test4/a")?;
    let b_hash = calculate_hash("tests/test3_and_test4/b")?;
    let c_hash = calculate_hash("tests/test3_and_test4/c")?;
    let d_hash = calculate_hash("tests/test3_and_test4/d d")?;

    // Create JSON content with R2 destination
    let json_content = json!([
        {
            "file_nm": "tests/test3_and_test4/a",
            "hash": a_hash,
            "hash_algo": "sha1",
            "ins_ts": "2023-01-01T00:00:00Z",
            "hostname": hostname::get()?.to_string_lossy().to_string(),
            "encoding": "utf8",
            "file_size": fs::metadata("tests/test3_and_test4/a")?.len(),
            "encoded_size": 2000, // Size doesn't matter for R2, just the destination_cloud
            "destination_cloud": "r2",
            // "secret_name": format!("file-{}", a_hash),
            "cloud_upload_bucket": "test-r2-upload-bucket"
        },
        {
            "file_nm": "tests/test3_and_test4/b",
            "hash": b_hash,
            "hash_algo": "sha1",
            "ins_ts": "2023-01-01T00:00:00Z",
            "hostname": hostname::get()?.to_string_lossy().to_string(),
            "encoding": "utf8",
            "file_size": fs::metadata("tests/test3_and_test4/b")?.len(),
            "encoded_size": 2000,
            "destination_cloud": "r2",
            // "secret_name": format!("file-{}", b_hash),
            "cloud_upload_bucket": "test-r2-upload-bucket"
        },
        {
            "file_nm": "tests/test3_and_test4/c",
            "hash": c_hash,
            "hash_algo": "sha1",
            "ins_ts": "2023-01-01T00:00:00Z",
            "hostname": hostname::get()?.to_string_lossy().to_string(),
            "encoding": "utf8",
            "file_size": fs::metadata("tests/test3_and_test4/c")?.len(),
            "encoded_size": 2000,
            "destination_cloud": "r2",
            // "secret_name": format!("file-{}", c_hash),
            "cloud_upload_bucket": "test-r2-upload-bucket"
        },
        {
            "file_nm": "tests/test3_and_test4/d d",
            "hash": d_hash,
            "hash_algo": "sha1",
            "ins_ts": "2023-01-01T00:00:00Z",
            "hostname": hostname::get()?.to_string_lossy().to_string(),
            "encoding": "utf8",
            "file_size": fs::metadata("tests/test3_and_test4/d d")?.len(),
            "encoded_size": 2000,
            "destination_cloud": "r2",
            // "secret_name": format!("file-{}", d_hash),
            "cloud_upload_bucket": "test-r2-upload-bucket"
        }
    ]);

    // Write to the temporary file
    std::fs::write(temp_file.path(), json_content.to_string())?;

    Ok(temp_file)
}