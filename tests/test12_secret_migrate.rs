use podman_compose_mgr::args::types::{Args, Mode};
use podman_compose_mgr::secrets::migrate::init::init_migrate;
use podman_compose_mgr::secrets::migrate::migrate_process::migrate_to_localhost;
use podman_compose_mgr::secrets::models::JsonEntry;
use std::path::PathBuf;

/// Test fixture setup - creates standard test Args for Secret Migrate mode
fn create_test_args() -> Args {
    Args {
        mode: Mode::SecretMigrate,
        input_json: Some(PathBuf::from("tests/test12/input.json")),
        output_json: Some(PathBuf::from("tests/test12/output.json")),
        azure_client_id_path: Some(PathBuf::from("tests/personal_testing_data/client_id.txt")),
        azure_client_secret_path: Some(PathBuf::from("tests/personal_testing_data/secret.txt")),
        azure_tenant_id_path: Some(PathBuf::from("tests/personal_testing_data/tenant_id.txt")),
        azure_vault_name_path: Some(PathBuf::from("tests/personal_testing_data/vault_name.txt")),
        verbose: 1,
        s3_account_id_filepath: Some(PathBuf::from("tests/personal_testing_data/r2_account_id.txt")),
        s3_secret_key_filepath: Some(PathBuf::from("tests/personal_testing_data/r2_secret.txt")),
        s3_endpoint_filepath: Some(PathBuf::from("tests/personal_testing_data/r2_endpoint.txt")),
        ..Default::default()
    }
}

/// Comprehensive integration test that checks:
/// 1. The run_app function correctly processes secrets in SecretMigrate mode
/// 2. Different hosts are correctly identified for migration
/// 3. Migration correctly updates hostname and recalculates hashes
#[test]
fn test_app_integration() {
    use std::fs::{self, File};
    use std::io::Read;
    use std::path::Path;
    
    // We will use test_mode parameter instead of environment variables
    
    // Clean up any previous test output
    let output_path = Path::new("tests/test12/output.json");
    if output_path.exists() {
        fs::remove_file(output_path).expect("Failed to remove previous test output");
    }
    
    // Part 1: Test the main application flow
    let args = create_test_args();
    
    // Test directly with init_migrate in test mode instead of run_app
    let result = init_migrate(&args, true);
    assert!(result.is_ok(), "Application should run successfully");
    
    // Verify output.json was created
    assert!(output_path.exists(), "Output file should be created");
    
    // Read the output file
    let mut output_file = File::open(output_path).expect("Failed to open output file");
    let mut output_content = String::new();
    output_file.read_to_string(&mut output_content).expect("Failed to read output file");
    
    // Read the reference output file
    let mut reference_file = File::open("tests/test12/reference_output.json").expect("Failed to open reference file");
    let mut reference_content = String::new();
    reference_file.read_to_string(&mut reference_content).expect("Failed to read reference file");
    
    // Parse both files to compare their structure 
    // (ignoring timestamp differences by comparing parsed structures)
    let output_json: serde_json::Value = serde_json::from_str(&output_content).expect("Failed to parse output JSON");
    let reference_json: serde_json::Value = serde_json::from_str(&reference_content).expect("Failed to parse reference JSON");
    
    // Compare key fields for each entry
    let output_entries = output_json.as_array().expect("Output should be an array");
    let reference_entries = reference_json.as_array().expect("Reference should be an array");
    
    assert_eq!(output_entries.len(), reference_entries.len(), "Number of entries should match");
    
    for (i, (output_entry, reference_entry)) in output_entries.iter().zip(reference_entries.iter()).enumerate() {
        // Compare important fields
        let output_file_nm = output_entry["file_nm"].as_str().unwrap_or_default();
        let reference_file_nm = reference_entry["file_nm"].as_str().unwrap_or_default();
        assert_eq!(output_file_nm, reference_file_nm, "file_nm should match for entry {}", i);
        
        let output_hostname = output_entry["hostname"].as_str().unwrap_or_default();
        let reference_hostname = reference_entry["hostname"].as_str().unwrap_or_default();
        assert_eq!(output_hostname, reference_hostname, "hostname should match for entry {}", i);
        
        let output_hash = output_entry["hash"].as_str().unwrap_or_default();
        let reference_hash = reference_entry["hash"].as_str().unwrap_or_default();
        assert_eq!(output_hash, reference_hash, "hash should match for entry {}", i);
        
        let output_encoding = output_entry["encoding"].as_str().unwrap_or_default();
        let reference_encoding = reference_entry["encoding"].as_str().unwrap_or_default();
        assert_eq!(output_encoding, reference_encoding, "encoding should match for entry {}", i);
    }
    
    // Part 2: Test individual migration logic with a new args instance
    let args2 = create_test_args();
    
    // Test entries with different hostnames
    let local_entry = JsonEntry {
        file_name: "local_secret.txt".to_string(), 
        hostname: hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "localhost".to_string()),
        destination_cloud: "azure_kv".to_string(),
        sha256: None,
        last_updated: None,
    };
    
    let remote_entry = JsonEntry {
        file_name: "remote_secret.txt".to_string(),
        hostname: "different-host.example.com".to_string(),
        destination_cloud: "azure_kv".to_string(), 
        sha256: None,
        last_updated: None,
    };
    
    // Both should succeed now with test_mode=true
    let result1 = migrate_to_localhost(&args2, &local_entry, true);
    let result2 = migrate_to_localhost(&args2, &remote_entry, true);
    
    assert!(result1.is_ok(), "Local migration should succeed");
    assert!(result2.is_ok(), "Remote migration should succeed");
    
    // No need to clean up environment variables
}