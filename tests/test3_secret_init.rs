use chrono::Local;
use podman_compose_mgr::{
    args::{self, Mode},
    secrets,
};
use serde_json::Value;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use tempfile::NamedTempFile;

#[test]
fn test_initialize_process() {
    // Create a temporary file for the output JSON
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_path_buf();

    // Create Args with the necessary parameters
    let args = args::Args {
        mode: Mode::SecretInitialize,
        secrets_init_filepath: Some(PathBuf::from("tests/test3_and_test4/test_input.json")),
        input_json: None,
        path: PathBuf::from("."),

        verbose: true,
        exclude_path_patterns: vec![],
        include_path_patterns: vec![],
        build_args: vec![],
        secrets_client_id: None,
        secrets_client_secret_path: None,
        secrets_tenant_id: None,
        secrets_vault_name: None,
        output_json: Some(temp_path.clone()),
        b2_key_id: None,
        b2_application_key: None,
        b2_bucket_name: None,
        b2_bucket_for_upload: Some("test_bucket".to_string()), // Add test bucket for B2 uploads
        b2_account_id_filepath: None,
        b2_account_key_filepath: None,
        r2_account_id: None,
        r2_access_key_id: None,
        r2_access_key: None,
        r2_access_key_id_filepath: None,
        r2_access_key_filepath: None,
        r2_bucket_for_upload: None,
    };

    // Run the initialize process
    let result = secrets::initialize::process(&args);
    assert!(
        result.is_ok(),
        "Failed to process secrets: {:?}",
        result.err()
    );

    // Read and parse the output file
    let mut output_content = String::new();
    let mut file = File::open(&temp_path).unwrap();
    file.read_to_string(&mut output_content).unwrap();

    let output_json: Vec<Value> = serde_json::from_str(&output_content).unwrap();

    // Verify there are 5 entries
    assert_eq!(output_json.len(), 5);

    // Get the current date
    let today = Local::now().date_naive();

    // Verify the entries
    for entry in &output_json {
        let filenm = entry["file_nm"].as_str().unwrap();
        let _hash = entry["hash"].as_str().unwrap(); // We don't check the exact hash value
        let ins_ts = entry["ins_ts"].as_str().unwrap();
        let hostname = entry["hostname"].as_str().unwrap();

        // Verify date (just year, month, day)
        let entry_date_str = &ins_ts[0..10]; // Extract YYYY-MM-DD
        let entry_date = chrono::NaiveDate::parse_from_str(entry_date_str, "%Y-%m-%d").unwrap();
        assert_eq!(entry_date, today);

        // Verify empty cloud fields
        assert_eq!(entry["cloud_id"].as_str().unwrap(), "");
        assert_eq!(entry["cloud_cr_ts"].as_str().unwrap(), "");
        assert_eq!(entry["cloud_upd_ts"].as_str().unwrap(), "");
        
        // Verify cloud_upload_bucket based on destination cloud
        let destination_cloud = entry["destination_cloud"].as_str().unwrap();
        let cloud_upload_bucket = entry["cloud_upload_bucket"].as_str().unwrap();
        if destination_cloud == "b2" {
            assert_eq!(cloud_upload_bucket, "test_bucket", 
                       "B2 destination should have bucket set to 'test_bucket'");
        } else {
            assert_eq!(cloud_upload_bucket, "", 
                       "Azure KeyVault destination should have empty bucket");
        }

        // Verify secret_name is not empty (format is "file-<hash>")
        let secret_name = entry["secret_name"].as_str().unwrap();
        assert!(
            secret_name.starts_with("file-"),
            "Secret name should start with 'file-'"
        );
        assert_eq!(
            secret_name.len(),
            45,
            "Secret name should be 45 characters long (5 for 'file-' + 40 for SHA-1 hash)"
        );

        // Verify hostname is not empty
        assert!(!hostname.is_empty());

        // Verify encoding exists
        let encoding = entry["encoding"].as_str().unwrap();
        
        // Verify destination cloud based on file size
        let encoded_size = entry["encoded_size"].as_u64().unwrap();
        let destination_cloud = entry["destination_cloud"].as_str().unwrap();
        
        if encoded_size > 24000 {
            assert_eq!(destination_cloud, "b2", 
                      "Files larger than 24KB should use B2 storage");
        } else {
            assert_eq!(destination_cloud, "azure_kv", 
                      "Files smaller than 24KB should use Azure KeyVault");
        }

        // Verify hash based on file content and check encoding
        match filenm {
            "tests/test3_and_test4/a" => {
                // We don't check the hash value since we switched from MD5 to SHA-1
                assert_eq!(encoding, "utf8", "File 'a' should be utf8 encoded");
            }
            "tests/test3_and_test4/b" => {
                // We don't check the hash value since we switched from MD5 to SHA-1
                assert_eq!(encoding, "utf8", "File 'b' should be utf8 encoded");
            }
            "tests/test3_and_test4/c" => {
                // We don't check the hash value since we switched from MD5 to SHA-1
                assert_eq!(encoding, "utf8", "File 'c' should be utf8 encoded");
            }
            "tests/test3_and_test4/d d" => {
                // We don't check the hash value since we switched from MD5 to SHA-1
                assert_eq!(encoding, "utf8", "File 'd d' should be utf8 encoded");
            }
            "tests/test3_and_test4/e e" => {
                // We don't check the hash value
                assert_eq!(encoding, "base64", "File 'e e' should be base64 encoded");
            }
            _ => panic!("Unexpected file: {}", filenm),
        }
    }

    // Test append functionality - run the process again
    let result = secrets::initialize::process(&args);
    assert!(
        result.is_ok(),
        "Failed to process secrets (append): {:?}",
        result.err()
    );

    // Read and parse the output file again
    let mut output_content = String::new();
    let mut file = File::open(&temp_path).unwrap();
    file.read_to_string(&mut output_content).unwrap();

    let output_json: Vec<Value> = serde_json::from_str(&output_content).unwrap();

    // Verify there are now 10 entries (5 original + 5 appended)
    assert_eq!(output_json.len(), 10);

    // Group entries by filename for more rigorous comparison
    let mut entries_by_filename = std::collections::HashMap::new();

    // First make sure we have all expected files and each has exactly 2 entries
    let mut file_counts = std::collections::HashMap::new();

    for entry in &output_json {
        let filenm = entry["file_nm"].as_str().unwrap().to_string();
        *file_counts.entry(filenm.to_string()).or_insert(0) += 1;
    }

    // Verify all files have exactly 2 entries
    assert_eq!(
        file_counts["tests/test3_and_test4/a"], 2,
        "Expected 2 entries for file a"
    );
    assert_eq!(
        file_counts["tests/test3_and_test4/b"], 2,
        "Expected 2 entries for file b"
    );
    assert_eq!(
        file_counts["tests/test3_and_test4/c"], 2,
        "Expected 2 entries for file c"
    );
    assert_eq!(
        file_counts["tests/test3_and_test4/d d"], 2,
        "Expected 2 entries for file 'd d'"
    );
    assert_eq!(
        file_counts["tests/test3_and_test4/e e"], 2,
        "Expected 2 entries for file 'e e'"
    );

    // Group entries by filename for comparison
    for entry in output_json {
        let filenm = entry["file_nm"].as_str().unwrap().to_string();
        entries_by_filename
            .entry(filenm)
            .or_insert_with(Vec::new)
            .push(entry);
    }

    // Verify that entries for each file match exactly
    for (filename, entries) in &entries_by_filename {
        let first = &entries[0];
        let second = &entries[1];

        // Compare all fields
        assert_eq!(
            first["file_nm"], second["file_nm"],
            "file_nm doesn't match for {}",
            filename
        );
        assert_eq!(
            first["hash"], second["hash"],
            "hash doesn't match for {}",
            filename
        );
        assert_eq!(
            first["cloud_id"], second["cloud_id"],
            "cloud_id doesn't match for {}",
            filename
        );
        assert_eq!(
            first["cloud_cr_ts"], second["cloud_cr_ts"],
            "cloud_cr_ts doesn't match for {}",
            filename
        );
        assert_eq!(
            first["cloud_upd_ts"], second["cloud_upd_ts"],
            "cloud_upd_ts doesn't match for {}",
            filename
        );
        assert_eq!(
            first["secret_name"], second["secret_name"],
            "secret_name doesn't match for {}",
            filename
        );
        assert_eq!(
            first["hostname"], second["hostname"],
            "hostname doesn't match for {}",
            filename
        );
        assert_eq!(
            first["encoding"], second["encoding"],
            "encoding doesn't match for {}",
            filename
        );
        assert_eq!(
            first["cloud_upload_bucket"], second["cloud_upload_bucket"],
            "cloud_upload_bucket doesn't match for {}",
            filename
        );

        // Double-check hash algorithm and verify encoding
        let hash_algo = first["hash_algo"].as_str().unwrap();
        let encoding = first["encoding"].as_str().unwrap();

        // Verify hash algorithm is SHA-1
        assert_eq!(hash_algo, "sha1", "Hash algorithm should be SHA-1");

        match filename.as_str() {
            "tests/test3_and_test4/a" => {
                assert_eq!(encoding, "utf8", "File 'a' should be utf8 encoded");
            }
            "tests/test3_and_test4/b" => {
                assert_eq!(encoding, "utf8", "File 'b' should be utf8 encoded");
            }
            "tests/test3_and_test4/c" => {
                assert_eq!(encoding, "utf8", "File 'c' should be utf8 encoded");
            }
            "tests/test3_and_test4/d d" => {
                assert_eq!(encoding, "utf8", "File 'd d' should be utf8 encoded");
            }
            "tests/test3_and_test4/e e" => {
                assert_eq!(encoding, "base64", "File 'e e' should be base64 encoded");
            }
            _ => panic!("Unexpected filename: {}", filename),
        }
    }
    // The tempfile will be automatically removed when it's dropped
    drop(temp_file);
}
