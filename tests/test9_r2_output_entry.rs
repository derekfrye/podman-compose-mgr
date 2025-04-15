use podman_compose_mgr::secrets::models::UploadEntry;
use podman_compose_mgr::secrets::r2_storage::R2UploadResult;
use serde_json::Value;
use std::fs::{self, File};
use std::io::{Read, Write};
use tempfile::NamedTempFile;

// const OUTPUT_PATH: &str = "tests/test9/output.json";
const INPUT_JSON_PATH: &str = "tests/test7_new_init_fmt/test_output.json";

#[test]
fn test_create_r2_output_entry() {
    // Load test JSON input
    let json_str = fs::read_to_string(INPUT_JSON_PATH)
        .expect("Failed to read test JSON input file");
    
    let entries: Vec<Value> = serde_json::from_str(&json_str)
        .expect("Failed to parse input JSON");
    
    // Create a mock R2 upload result
    let r2_result = R2UploadResult {
        hash: "test_r2_hash_value".to_string(),
        id: "test_cloud_id".to_string(),
        bucket_id: "test_bucket_id".to_string(),
        name: "test_r2_name".to_string(),
        created: "2024-01-01T00:00:00Z".to_string(),
        updated: "2024-01-01T00:00:00Z".to_string(),
    };
    
    // Process all entries that have "r2" as destination_cloud
    let mut r2_outputs = Vec::new();
    
    for entry in entries.iter().filter(|e| e["destination_cloud"].as_str().unwrap_or("") == "r2") {
        let file_path = entry["filenm"].as_str().unwrap_or("unknown");
        
        // Get actual file size if the file exists
        let file_size = match fs::metadata(file_path) {
            Ok(metadata) => metadata.len(),
            Err(_) => 0, // File might not exist in test environment
        };
        
        // Create an UploadEntry from the test data
        let upload_entry = UploadEntry {
            file_nm: file_path.to_string(),
            hash: "test_file_hash".to_string(),  // Mock hash value
            hash_algo: "sha256".to_string(),
            ins_ts: "2024-01-01T00:00:00Z".to_string(),
            hostname: "test-hostname".to_string(),
            encoding: "utf8".to_string(),
            file_size,
            encoded_size: file_size,
            destination_cloud: "r2".to_string(),
            cloud_upload_bucket: entry["cloud_upload_bucket"].as_str().map(String::from),
            cloud_prefix: None,
        };
        
        // Generate R2 output entry
        let r2_output = upload_entry.create_r2_output_entry(&r2_result);
        r2_outputs.push(r2_output);
    }
    
    // Check if output file exists
    // let output_exists = Path::new(OUTPUT_PATH).exists();
    
    
        // Subsequent runs: compare with reference
        let mut temp_file = NamedTempFile::new()
            .expect("Failed to create temporary file");
        
        // Write current output to temp file
        let json_string = serde_json::to_string_pretty(&r2_outputs)
            .expect("Failed to serialize output to JSON");
        temp_file.write_all(json_string.as_bytes())
            .expect("Failed to write to temporary file");
        
        // Read reference file
        let mut reference_content = String::new();
        File::open("tests/test9/output.json")
            .expect("Failed to open reference file")
            .read_to_string(&mut reference_content)
            .expect("Failed to read reference file");
        
        // Parse reference JSON
        let reference_json: Vec<Value> = serde_json::from_str(&reference_content)
            .expect("Failed to parse reference JSON");
        
        // Make sure we have the same number of entries
        assert_eq!(r2_outputs.len(), reference_json.len(), "Number of entries mismatch");
        
        // Compare each entry
        for (i, (current, reference)) in r2_outputs.iter().zip(reference_json.iter()).enumerate() {
            assert_eq!(current["file_nm"], reference["file_nm"], "file_nm mismatch at entry {}", i);
            assert_eq!(current["hash"], reference["hash"], "hash mismatch at entry {}", i);
            assert_eq!(current["hash_algo"], reference["hash_algo"], "hash_algo mismatch at entry {}", i);
            assert_eq!(current["cloud_id"], reference["cloud_id"], "cloud_id mismatch at entry {}", i);
            assert_eq!(current["hostname"], reference["hostname"], "hostname mismatch at entry {}", i);
            assert_eq!(current["encoding"], reference["encoding"], "encoding mismatch at entry {}", i);
            assert_eq!(current["file_size"], reference["file_size"], "file_size mismatch at entry {}", i);
            assert_eq!(current["encoded_size"], reference["encoded_size"], "encoded_size mismatch at entry {}", i);
            assert_eq!(current["destination_cloud"], reference["destination_cloud"], "destination_cloud mismatch at entry {}", i);
            assert_eq!(current["cloud_upload_bucket"], reference["cloud_upload_bucket"], "cloud_upload_bucket mismatch at entry {}", i);
            assert_eq!(current["cloud_prefix"], reference["cloud_prefix"], "cloud_prefix mismatch at entry {}", i);
            assert_eq!(current["r2_hash"], reference["r2_hash"], "r2_hash mismatch at entry {}", i);
            assert_eq!(current["r2_bucket_id"], reference["r2_bucket_id"], "r2_bucket_id mismatch at entry {}", i);
            assert_eq!(current["r2_name"], reference["r2_name"], "r2_name mismatch at entry {}", i);
        }
        
        println!("All {} entries match reference", r2_outputs.len());
    }
