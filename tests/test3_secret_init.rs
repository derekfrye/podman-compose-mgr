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
        secrets_init_filepath: Some(PathBuf::from("tests/test3/test_input.json")),
        secret_mode_input_json: Some(temp_path.clone()),
        path: PathBuf::from("."),
        
        verbose: true,
        exclude_path_patterns: vec![],
        include_path_patterns: vec![],
        build_args: vec![],
        secrets_client_id: None,
        secrets_client_secret_path: None,
        secrets_tenant_id: None,
        secrets_vault_name: None,
        secret_mode_output_json: None,
    };
    
    // Run the initialize process
    let result = secrets::initialize::process(&args);
    assert!(result.is_ok(), "Failed to process secrets: {:?}", result.err());
    
    // Read and parse the output file
    let mut output_content = String::new();
    let mut file = File::open(&temp_path).unwrap();
    file.read_to_string(&mut output_content).unwrap();
    
    let output_json: Vec<Value> = serde_json::from_str(&output_content).unwrap();
    
    // Verify there are 4 entries
    assert_eq!(output_json.len(), 4);
    
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
        
        // Verify MD5 based on file content
        match filenm {
            "tests/test3/a" => assert_eq!(md5, "60b725f10c9c85c70d97880dfe8191b3"),
            "tests/test3/b" => assert_eq!(md5, "bfcc9da4f2e1d313c63cd0a4ee7604e9"),
            "tests/test3/c" => assert_eq!(md5, "c576ec4297a7bdacc878e0061192441e"),
            "tests/test3/d d" => assert_eq!(md5, "ef76b4f269b9a5104e4f061419a5f529"),
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
    
    // Verify there are now 8 entries (4 original + 4 appended)
    assert_eq!(output_json.len(), 8);
    
    // Group entries by filename for more rigorous comparison
    let mut entries_by_filename = std::collections::HashMap::new();
    
    // First make sure we have all expected files and each has exactly 2 entries
    let mut file_counts = std::collections::HashMap::new();
    
    for entry in &output_json {
        let filenm = entry["filenm"].as_str().unwrap().to_string();
        *file_counts.entry(filenm.to_string()).or_insert(0) += 1;
    }
    
    // Verify all files have exactly 2 entries
    assert_eq!(file_counts["tests/test3/a"], 2, "Expected 2 entries for file a");
    assert_eq!(file_counts["tests/test3/b"], 2, "Expected 2 entries for file b");
    assert_eq!(file_counts["tests/test3/c"], 2, "Expected 2 entries for file c");
    assert_eq!(file_counts["tests/test3/d d"], 2, "Expected 2 entries for file 'd d'");
    
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
        
        // Double-check MD5 against expected values
        let md5 = first["md5"].as_str().unwrap();
        match filename.as_str() {
            "tests/test3/a" => assert_eq!(md5, "60b725f10c9c85c70d97880dfe8191b3", "MD5 mismatch for file a"),
            "tests/test3/b" => assert_eq!(md5, "bfcc9da4f2e1d313c63cd0a4ee7604e9", "MD5 mismatch for file b"),
            "tests/test3/c" => assert_eq!(md5, "c576ec4297a7bdacc878e0061192441e", "MD5 mismatch for file c"),
            "tests/test3/d d" => assert_eq!(md5, "ef76b4f269b9a5104e4f061419a5f529", "MD5 mismatch for file 'd d'"),
            _ => panic!("Unexpected filename: {}", filename),
        }
    }
    // The tempfile will be automatically removed when it's dropped
    drop(temp_file);
}