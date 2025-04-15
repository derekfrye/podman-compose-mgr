use podman_compose_mgr::secrets::models::UploadEntry;
use podman_compose_mgr::secrets::r2_storage::R2UploadResult;
use serde_json::Value;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use tempfile::NamedTempFile;

const OUTPUT_PATH: &str = "tests/test9/output.json";
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
    
    // Find an R2 entry to test with
    let r2_entry = entries.iter()
        .find(|entry| {
            entry["destination_cloud"].as_str().unwrap_or("") == "r2" &&
            entry["cloud_upload_bucket"].as_str().is_some()
        })
        .expect("No suitable R2 entry found in test data");
    
    // Create an UploadEntry from the test data
    let upload_entry = UploadEntry {
        file_nm: r2_entry["filenm"].as_str().unwrap_or("unknown").to_string(),
        hash: "test_file_hash".to_string(),  // Mock hash value
        hash_algo: "sha256".to_string(),
        ins_ts: "2024-01-01T00:00:00Z".to_string(),
        hostname: "test-hostname".to_string(),
        encoding: "utf8".to_string(),
        file_size: 1024,
        encoded_size: 1024,
        destination_cloud: "r2".to_string(),
        cloud_upload_bucket: r2_entry["cloud_upload_bucket"].as_str().map(String::from),
        cloud_prefix: None,
    };
    
    // Generate R2 output entry
    let r2_output = upload_entry.create_r2_output_entry(&r2_result);
    
    // Check if output file exists
    let output_exists = Path::new(OUTPUT_PATH).exists();
    
    if !output_exists {
        // First run: save the output as the reference
        let json_string = serde_json::to_string_pretty(&r2_output)
            .expect("Failed to serialize output to JSON");
        
        // Ensure directory exists
        if let Some(parent) = Path::new(OUTPUT_PATH).parent() {
            fs::create_dir_all(parent).expect("Failed to create output directory");
        }
        
        // Write to file
        let mut file = File::create(OUTPUT_PATH)
            .expect("Failed to create output file");
        file.write_all(json_string.as_bytes())
            .expect("Failed to write to output file");
        
        println!("Created reference output at {}", OUTPUT_PATH);
    } else {
        // Subsequent runs: compare with reference
        let mut temp_file = NamedTempFile::new()
            .expect("Failed to create temporary file");
        
        // Write current output to temp file
        let json_string = serde_json::to_string_pretty(&r2_output)
            .expect("Failed to serialize output to JSON");
        temp_file.write_all(json_string.as_bytes())
            .expect("Failed to write to temporary file");
        
        // Read reference file
        let mut reference_content = String::new();
        File::open(OUTPUT_PATH)
            .expect("Failed to open reference file")
            .read_to_string(&mut reference_content)
            .expect("Failed to read reference file");
        
        // Parse both JSONs
        let reference_json: Value = serde_json::from_str(&reference_content)
            .expect("Failed to parse reference JSON");
        
        // Compare element by element
        assert_eq!(r2_output["file_nm"], reference_json["file_nm"], "file_nm mismatch");
        assert_eq!(r2_output["hash"], reference_json["hash"], "hash mismatch");
        assert_eq!(r2_output["hash_algo"], reference_json["hash_algo"], "hash_algo mismatch");
        assert_eq!(r2_output["cloud_id"], reference_json["cloud_id"], "cloud_id mismatch");
        assert_eq!(r2_output["hostname"], reference_json["hostname"], "hostname mismatch");
        assert_eq!(r2_output["encoding"], reference_json["encoding"], "encoding mismatch");
        assert_eq!(r2_output["file_size"], reference_json["file_size"], "file_size mismatch");
        assert_eq!(r2_output["encoded_size"], reference_json["encoded_size"], "encoded_size mismatch");
        assert_eq!(r2_output["destination_cloud"], reference_json["destination_cloud"], "destination_cloud mismatch");
        assert_eq!(r2_output["cloud_upload_bucket"], reference_json["cloud_upload_bucket"], "cloud_upload_bucket mismatch");
        assert_eq!(r2_output["cloud_prefix"], reference_json["cloud_prefix"], "cloud_prefix mismatch");
        assert_eq!(r2_output["r2_hash"], reference_json["r2_hash"], "r2_hash mismatch");
        assert_eq!(r2_output["r2_bucket_id"], reference_json["r2_bucket_id"], "r2_bucket_id mismatch");
        assert_eq!(r2_output["r2_name"], reference_json["r2_name"], "r2_name mismatch");
        
        println!("Output matches reference");
    }
}