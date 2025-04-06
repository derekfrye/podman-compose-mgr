use podman_compose_mgr::{args::{self, Mode}, secrets};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use chrono::Local;
use serde_json::Value;

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
    };
    
    // Run the initialize process
    let result = secrets::initialize::process(&args);
    assert!(result.is_ok(), "Failed to process secrets: {:?}", result.err());
    
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
        let filenm = entry["filenm"].as_str().unwrap();
        let md5 = entry["md5"].as_str().unwrap();
        let ins_ts = entry["ins_ts"].as_str().unwrap();
        let hostname = entry["hostname"].as_str().unwrap();
        
        // Verify date (just year, month, day)
        let entry_date_str = &ins_ts[0..10]; // Extract YYYY-MM-DD
        let entry_date = chrono::NaiveDate::parse_from_str(entry_date_str, "%Y-%m-%d").unwrap();
        assert_eq!(entry_date, today);
        
        // Verify empty Azure fields
        assert_eq!(entry["az_id"].as_str().unwrap(), "");
        assert_eq!(entry["az_create"].as_str().unwrap(), "");
        assert_eq!(entry["az_updated"].as_str().unwrap(), "");
        assert_eq!(entry["az_name"].as_str().unwrap(), "");
        
        // Verify hostname is not empty
        assert!(!hostname.is_empty());
        
        // Verify encoding exists
        let encoding = entry["encoding"].as_str().unwrap();
        
        // Verify MD5 based on file content and check encoding
        match filenm {
            "tests/test3_and_test4/a" => {
                assert_eq!(md5, "60b725f10c9c85c70d97880dfe8191b3");
                assert_eq!(encoding, "utf8", "File 'a' should be utf8 encoded");
            },
            "tests/test3_and_test4/b" => {
                assert_eq!(md5, "bfcc9da4f2e1d313c63cd0a4ee7604e9");
                assert_eq!(encoding, "utf8", "File 'b' should be utf8 encoded");
            },
            "tests/test3_and_test4/c" => {
                assert_eq!(md5, "c576ec4297a7bdacc878e0061192441e");
                assert_eq!(encoding, "utf8", "File 'c' should be utf8 encoded");
            },
            "tests/test3_and_test4/d d" => {
                assert_eq!(md5, "ef76b4f269b9a5104e4f061419a5f529");
                assert_eq!(encoding, "utf8", "File 'd d' should be utf8 encoded");
            },
            "tests/test3_and_test4/e e" => {
                // We don't hard-code the MD5 for the random file
                assert_eq!(encoding, "base64", "File 'e e' should be base64 encoded");
            },
            _ => panic!("Unexpected file: {}", filenm),
        }
    }
    
    // Test append functionality - run the process again
    let result = secrets::initialize::process(&args);
    assert!(result.is_ok(), "Failed to process secrets (append): {:?}", result.err());
    
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
        let filenm = entry["filenm"].as_str().unwrap().to_string();
        *file_counts.entry(filenm.to_string()).or_insert(0) += 1;
    }
    
    // Verify all files have exactly 2 entries
    assert_eq!(file_counts["tests/test3_and_test4/a"], 2, "Expected 2 entries for file a");
    assert_eq!(file_counts["tests/test3_and_test4/b"], 2, "Expected 2 entries for file b");
    assert_eq!(file_counts["tests/test3_and_test4/c"], 2, "Expected 2 entries for file c");
    assert_eq!(file_counts["tests/test3_and_test4/d d"], 2, "Expected 2 entries for file 'd d'");
    assert_eq!(file_counts["tests/test3_and_test4/e e"], 2, "Expected 2 entries for file 'e e'");
    
    // Group entries by filename for comparison
    for entry in output_json {
        let filenm = entry["filenm"].as_str().unwrap().to_string();
        entries_by_filename.entry(filenm).or_insert_with(Vec::new).push(entry);
    }
    
    // Verify that entries for each file match exactly
    for (filename, entries) in &entries_by_filename {
        let first = &entries[0];
        let second = &entries[1];
        
        // Compare all fields
        assert_eq!(first["filenm"], second["filenm"], "filenm doesn't match for {}", filename);
        assert_eq!(first["md5"], second["md5"], "md5 doesn't match for {}", filename);
        assert_eq!(first["az_id"], second["az_id"], "az_id doesn't match for {}", filename);
        assert_eq!(first["az_create"], second["az_create"], "az_create doesn't match for {}", filename);
        assert_eq!(first["az_updated"], second["az_updated"], "az_updated doesn't match for {}", filename);
        assert_eq!(first["az_name"], second["az_name"], "az_name doesn't match for {}", filename);
        assert_eq!(first["hostname"], second["hostname"], "hostname doesn't match for {}", filename);
        assert_eq!(first["encoding"], second["encoding"], "encoding doesn't match for {}", filename);
        
        // Double-check MD5 against expected values and verify encoding
        let md5 = first["md5"].as_str().unwrap();
        let encoding = first["encoding"].as_str().unwrap();
        
        match filename.as_str() {
            "tests/test3_and_test4/a" => {
                assert_eq!(md5, "60b725f10c9c85c70d97880dfe8191b3", "MD5 mismatch for file a");
                assert_eq!(encoding, "utf8", "File 'a' should be utf8 encoded");
            },
            "tests/test3_and_test4/b" => {
                assert_eq!(md5, "bfcc9da4f2e1d313c63cd0a4ee7604e9", "MD5 mismatch for file b");
                assert_eq!(encoding, "utf8", "File 'b' should be utf8 encoded");
            },
            "tests/test3_and_test4/c" => {
                assert_eq!(md5, "c576ec4297a7bdacc878e0061192441e", "MD5 mismatch for file c");
                assert_eq!(encoding, "utf8", "File 'c' should be utf8 encoded");
            },
            "tests/test3_and_test4/d d" => {
                assert_eq!(md5, "ef76b4f269b9a5104e4f061419a5f529", "MD5 mismatch for file 'd d'");
                assert_eq!(encoding, "utf8", "File 'd d' should be utf8 encoded");
            },
            "tests/test3_and_test4/e e" => {
                assert_eq!(encoding, "base64", "File 'e e' should be base64 encoded");
            },
            _ => panic!("Unexpected filename: {}", filename),
        }
    }
    // The tempfile will be automatically removed when it's dropped
    drop(temp_file);
}