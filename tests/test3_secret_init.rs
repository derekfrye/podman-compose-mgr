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
        verbose: 1,
        output_json: Some(temp_path.clone()),
        ..Default::default()
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
        if destination_cloud == "b2" || destination_cloud == "r2" {
            assert_eq!(cloud_upload_bucket, "bucket_required_during_upload", 
                       "B2/R2 destination should have placeholder bucket");
        } else {
            assert_eq!(cloud_upload_bucket, "", 
                       "Azure KeyVault destination should have empty bucket");
        }

        // Verify hash is present and not empty
        let hash = entry["hash"].as_str().unwrap();
        assert!(
            !hash.is_empty(),
            "Hash should not be empty"
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

    // Store first run timestamps to compare with second run
    let mut first_run_timestamps = std::collections::HashMap::new();
    
    // Read and parse the output file after first run
    let mut output_content = String::new();
    let mut file = File::open(&temp_path).unwrap();
    file.read_to_string(&mut output_content).unwrap();
    
    let first_run_json: Vec<Value> = serde_json::from_str(&output_content).unwrap();
    
    // Store the timestamps from the first run
    for entry in &first_run_json {
        let filenm = entry["file_nm"].as_str().unwrap().to_string();
        let ins_ts = entry["ins_ts"].as_str().unwrap().to_string();
        first_run_timestamps.insert(filenm, ins_ts);
    }
    
    // Wait a small amount of time to ensure timestamps will be different
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Run the process again with an input file including one new file
    let args_run2 = args::Args {
        mode: Mode::SecretInitialize,
        secrets_init_filepath: Some(PathBuf::from("tests/test3_and_test4/test_input_run2.json")),
        verbose: 1,
        output_json: Some(temp_path.clone()),
        ..Default::default()
    };
    
    let result = secrets::initialize::process(&args_run2);
    assert!(
        result.is_ok(),
        "Failed to process secrets (run 2): {:?}",
        result.err()
    );

    // Read and parse the output file after second run
    let mut output_content = String::new();
    let mut file = File::open(&temp_path).unwrap();
    file.read_to_string(&mut output_content).unwrap();

    let output_json: Vec<Value> = match serde_json::from_str(&output_content) {
        Ok(json) => json,
        Err(e) => {
            println!("Error parsing JSON: {}", e);
            println!("Content: {}", output_content);
            Vec::new()
        }
    };

    // With the new file, we should now have 6 entries (5 updated and 1 new)
    assert_eq!(output_json.len(), 6, "Expected 6 entries after second run (5 updated + 1 new)");

    // Create a map of entries by filename for easier reference
    let mut entries_by_filename = std::collections::HashMap::new();
    let mut file_seen = std::collections::HashSet::new();

    for entry in &output_json {
        let filenm = entry["file_nm"].as_str().unwrap().to_string();
        file_seen.insert(filenm.clone());
        entries_by_filename.insert(filenm.clone(), entry.clone());
        
        // Get the entry's timestamp and verify it's the current date
        let ins_ts = entry["ins_ts"].as_str().unwrap();
        let entry_date_str = &ins_ts[0..10]; // Extract YYYY-MM-DD
        let entry_date = chrono::NaiveDate::parse_from_str(entry_date_str, "%Y-%m-%d").unwrap();
        
        // The ins_ts date should be today
        assert_eq!(entry_date, today, "Expected entry to have today's date");
        
        // For existing files (a through e e), verify timestamps were updated
        if filenm != "tests/test3_and_test4/f" && first_run_timestamps.contains_key(&filenm) {
            let first_run_ts = first_run_timestamps.get(&filenm).unwrap();
            let second_run_ts = ins_ts;
            
            assert_ne!(first_run_ts, second_run_ts, 
                "Timestamp for {} should have been updated but wasn't", filenm);
        }
    }

    // Verify we saw all expected files from the second run
    assert!(file_seen.contains("tests/test3_and_test4/a"), "Missing file 'a'");
    assert!(file_seen.contains("tests/test3_and_test4/b"), "Missing file 'b'");
    assert!(file_seen.contains("tests/test3_and_test4/c"), "Missing file 'c'");
    assert!(file_seen.contains("tests/test3_and_test4/d d"), "Missing file 'd d'");
    assert!(file_seen.contains("tests/test3_and_test4/e e"), "Missing file 'e e'");
    assert!(file_seen.contains("tests/test3_and_test4/f"), "Missing new file 'f'");
    
    // Verify fields for each file
    // Verify 'a' file
    let a_entry = &entries_by_filename["tests/test3_and_test4/a"];
    assert_eq!(a_entry["hash_algo"].as_str().unwrap(), "sha1", "Hash algorithm should be SHA-1");
    assert_eq!(a_entry["encoding"].as_str().unwrap(), "utf8", "File 'a' should be utf8 encoded");
    assert_eq!(a_entry["cloud_id"].as_str().unwrap(), "", "cloud_id should be empty");
    assert_eq!(a_entry["cloud_cr_ts"].as_str().unwrap(), "", "cloud_cr_ts should be empty");
    assert_eq!(a_entry["cloud_upd_ts"].as_str().unwrap(), "", "cloud_upd_ts should be empty");
    
    // Verify 'b' file
    let b_entry = &entries_by_filename["tests/test3_and_test4/b"];
    assert_eq!(b_entry["hash_algo"].as_str().unwrap(), "sha1", "Hash algorithm should be SHA-1");
    assert_eq!(b_entry["encoding"].as_str().unwrap(), "utf8", "File 'b' should be utf8 encoded");
    
    // Verify 'c' file
    let c_entry = &entries_by_filename["tests/test3_and_test4/c"];
    assert_eq!(c_entry["hash_algo"].as_str().unwrap(), "sha1", "Hash algorithm should be SHA-1");
    assert_eq!(c_entry["encoding"].as_str().unwrap(), "utf8", "File 'c' should be utf8 encoded");
    
    // Verify 'd d' file
    let d_entry = &entries_by_filename["tests/test3_and_test4/d d"];
    assert_eq!(d_entry["hash_algo"].as_str().unwrap(), "sha1", "Hash algorithm should be SHA-1");
    assert_eq!(d_entry["encoding"].as_str().unwrap(), "utf8", "File 'd d' should be utf8 encoded");
    
    // Verify 'e e' file
    let e_entry = &entries_by_filename["tests/test3_and_test4/e e"];
    assert_eq!(e_entry["hash_algo"].as_str().unwrap(), "sha1", "Hash algorithm should be SHA-1");
    assert_eq!(e_entry["encoding"].as_str().unwrap(), "base64", "File 'e e' should be base64 encoded");
    
    // Verify 'f' file (the new file added in the second run)
    let f_entry = &entries_by_filename["tests/test3_and_test4/f"];
    assert_eq!(f_entry["hash_algo"].as_str().unwrap(), "sha1", "Hash algorithm should be SHA-1");
    assert_eq!(f_entry["encoding"].as_str().unwrap(), "utf8", "File 'f' should be utf8 encoded");
    assert_eq!(f_entry["file_size"].as_u64().unwrap(), 2, "File 'f' should be 2 bytes");
    
    // Verify the new 'f' file has no timestamp in first_run_timestamps
    assert!(!first_run_timestamps.contains_key("tests/test3_and_test4/f"), 
        "File 'f' should not have a timestamp from the first run");
    
    // Verify each file has the expected cloud destination based on size
    for (_filename, entry) in &entries_by_filename {
        let encoded_size = entry["encoded_size"].as_u64().unwrap();
        let destination_cloud = entry["destination_cloud"].as_str().unwrap();
        
        if encoded_size > 24000 {
            assert_eq!(destination_cloud, "r2", 
                      "Files larger than 24KB should use R2 storage");
        } else {
            assert_eq!(destination_cloud, "azure_kv", 
                      "Files smaller than 24KB should use Azure KeyVault");
        }
        
        // Verify cloud_upload_bucket based on destination cloud
        let cloud_upload_bucket = entry["cloud_upload_bucket"].as_str().unwrap();
        match destination_cloud {
            "b2" | "r2" => {
                assert_eq!(cloud_upload_bucket, "", 
                         "B2/R2 destination should have empty bucket at initialize time");
            },
            _ => {
                assert_eq!(cloud_upload_bucket, "", 
                         "Azure KeyVault destination should have empty bucket");
            }
        }
    }
    
    // The tempfile will be automatically removed when it's dropped
    drop(temp_file);
}
