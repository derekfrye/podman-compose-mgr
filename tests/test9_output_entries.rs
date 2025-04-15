use podman_compose_mgr::secrets::models::{SetSecretResponse, UploadEntry};
use podman_compose_mgr::secrets::r2_storage::R2UploadResult;
use serde_json::Value;
use std::fs::{self, File};
use std::io::{Read, Write};
use tempfile::NamedTempFile;
use time::OffsetDateTime;

// const OUTPUT_PATH: &str = "tests/test9/output.json";
const INPUT_JSON_PATH: &str = "tests/test7_new_init_fmt/test_output.json";

#[test]
fn test_create_output_entries() {
    // Load test JSON input
    let json_str =
        fs::read_to_string(INPUT_JSON_PATH).expect("Failed to read test JSON input file");

    let entries: Vec<Value> = serde_json::from_str(&json_str).expect("Failed to parse input JSON");

    // Create mock upload results for each cloud provider
    let r2_result = R2UploadResult {
        hash: "test_r2_hash_value".to_string(),
        id: "test_cloud_id".to_string(),
        bucket_id: "test_bucket_id".to_string(),
        name: "test_r2_name".to_string(),
        created: "2024-01-01T00:00:00Z".to_string(),
        updated: "2024-01-01T00:00:00Z".to_string(),
    };

    // B2 is similar to R2 in structure, so we'll just modify the R2 output
    // when needed rather than creating a separate B2 result

    let now = OffsetDateTime::now_utc();
    let azure_result = SetSecretResponse {
        created: now,
        updated: now,
        name: "test_secret_name".to_string(),
        id: "https://keyvault.vault.azure.net/secrets/test-secret-name".to_string(),
        value: "test_secret_value".to_string(),
    };

    // Process all entries and create corresponding output entries
    let mut all_outputs = Vec::new();

    for entry in entries.iter() {
        let file_path = entry["filenm"].as_str().unwrap_or("unknown");
        let cloud_type = entry["destination_cloud"].as_str().unwrap_or("azure_kv");

        // Get actual file size if the file exists
        let file_size = match fs::metadata(file_path) {
            Ok(metadata) => metadata.len(),
            Err(_) => 0, // File might not exist in test environment
        };

        // Create an UploadEntry using the proper constructor and then customize it
        let mut upload_entry = UploadEntry::new(file_path, "test_file_hash");

        // Update fields with test values to ensure consistency
        upload_entry.hash_algo = "sha256".to_string();
        upload_entry.ins_ts = "2024-01-01T00:00:00Z".to_string();
        upload_entry.hostname = "test-hostname".to_string();
        upload_entry.file_size = file_size;
        upload_entry.encoded_size = file_size;
        upload_entry.destination_cloud = cloud_type.to_string();

        // Set the bucket if it exists in the input
        if let Some(bucket) = entry["cloud_upload_bucket"].as_str() {
            upload_entry.cloud_upload_bucket = Some(bucket.to_string());
        }

        // Generate output entry using the actual production methods from UploadEntry
        let output = match cloud_type {
            "r2" => upload_entry.create_r2_output_entry(&r2_result),
            "b2" => {
                // Start with R2 output (since they're similar) and modify for B2
                let mut output = upload_entry.create_r2_output_entry(&r2_result);

                // Replace R2-specific fields with B2 equivalents
                output["r2_hash"] = Value::String("test_b2_hash_value".to_string());
                output["r2_name"] = Value::String("test_b2_name".to_string());
                output["destination_cloud"] = Value::String("b2".to_string());

                output
            }
            _ => upload_entry.create_azure_output_entry(&azure_result),
        };

        all_outputs.push(output);
    }

    // Write current output to temp file for debugging
    let mut temp_file = NamedTempFile::new().expect("Failed to create temporary file");

    let json_string =
        serde_json::to_string_pretty(&all_outputs).expect("Failed to serialize output to JSON");
    temp_file
        .write_all(json_string.as_bytes())
        .expect("Failed to write to temporary file");

    // Store outputs for each cloud type in separate collections
    let r2_outputs: Vec<Value> = all_outputs
        .iter()
        .filter(|e| e["destination_cloud"].as_str().unwrap_or("") == "r2")
        .cloned()
        .collect();

    let azure_outputs: Vec<Value> = all_outputs
        .iter()
        .filter(|e| e["destination_cloud"].as_str().unwrap_or("") == "azure_kv")
        .cloned()
        .collect();

    let b2_outputs: Vec<Value> = all_outputs
        .iter()
        .filter(|e| e["destination_cloud"].as_str().unwrap_or("") == "b2")
        .cloned()
        .collect();

    // Read reference file for R2 entries
    let mut reference_content = String::new();
    File::open("tests/test9/output.json")
        .expect("Failed to open reference file")
        .read_to_string(&mut reference_content)
        .expect("Failed to read reference file");

    // Parse reference JSON
    let reference_json: Vec<Value> =
        serde_json::from_str(&reference_content).expect("Failed to parse reference JSON");

    // Make sure we have the same number of R2 entries
    assert_eq!(
        r2_outputs.len(),
        reference_json.len(),
        "Number of R2 entries mismatch"
    );

    // Compare each R2 entry with the reference
    for (i, (current, reference)) in r2_outputs.iter().zip(reference_json.iter()).enumerate() {
        assert_eq!(
            current["file_nm"], reference["file_nm"],
            "file_nm mismatch at entry {}",
            i
        );
        assert_eq!(
            current["hash"], reference["hash"],
            "hash mismatch at entry {}",
            i
        );
        assert_eq!(
            current["hash_algo"], reference["hash_algo"],
            "hash_algo mismatch at entry {}",
            i
        );
        assert_eq!(
            current["cloud_id"], reference["cloud_id"],
            "cloud_id mismatch at entry {}",
            i
        );
        assert_eq!(
            current["hostname"], reference["hostname"],
            "hostname mismatch at entry {}",
            i
        );
        assert_eq!(
            current["encoding"], reference["encoding"],
            "encoding mismatch at entry {}",
            i
        );
        assert_eq!(
            current["file_size"], reference["file_size"],
            "file_size mismatch at entry {}",
            i
        );
        assert_eq!(
            current["encoded_size"], reference["encoded_size"],
            "encoded_size mismatch at entry {}",
            i
        );
        assert_eq!(
            current["destination_cloud"], reference["destination_cloud"],
            "destination_cloud mismatch at entry {}",
            i
        );
        assert_eq!(
            current["cloud_upload_bucket"], reference["cloud_upload_bucket"],
            "cloud_upload_bucket mismatch at entry {}",
            i
        );
        assert_eq!(
            current["cloud_prefix"], reference["cloud_prefix"],
            "cloud_prefix mismatch at entry {}",
            i
        );
        assert_eq!(
            current["r2_hash"], reference["r2_hash"],
            "r2_hash mismatch at entry {}",
            i
        );
        assert_eq!(
            current["r2_bucket_id"], reference["r2_bucket_id"],
            "r2_bucket_id mismatch at entry {}",
            i
        );
        assert_eq!(
            current["r2_name"], reference["r2_name"],
            "r2_name mismatch at entry {}",
            i
        );
    }

    // Verify Azure entries contain expected fields
    for (i, entry) in azure_outputs.iter().enumerate() {
        assert!(
            entry.get("cloud_id").is_some(),
            "Azure entry {} missing cloud_id",
            i
        );
        assert!(
            entry.get("cloud_cr_ts").is_some(),
            "Azure entry {} missing cloud_cr_ts",
            i
        );
        assert!(
            entry.get("cloud_upd_ts").is_some(),
            "Azure entry {} missing cloud_upd_ts",
            i
        );
        assert_eq!(
            entry["destination_cloud"].as_str().unwrap_or(""),
            "azure_kv",
            "Azure entry {} has incorrect destination_cloud",
            i
        );
    }

    // Verify B2 entries contain expected fields (which are similar to R2 but with different naming)
    for (i, entry) in b2_outputs.iter().enumerate() {
        assert!(
            entry.get("r2_hash").is_some(),
            "B2 entry {} missing r2_hash (hash)",
            i
        );
        assert!(
            entry.get("r2_bucket_id").is_some(),
            "B2 entry {} missing r2_bucket_id (bucket)",
            i
        );
        assert!(
            entry.get("r2_name").is_some(),
            "B2 entry {} missing r2_name (name)",
            i
        );
        assert_eq!(
            entry["destination_cloud"].as_str().unwrap_or(""),
            "b2",
            "B2 entry {} has incorrect destination_cloud",
            i
        );
    }

    println!("All cloud entry types validated:");
    println!("  - {} R2 entries match reference", r2_outputs.len());
    println!(
        "  - {} Azure KeyVault entries validated",
        azure_outputs.len()
    );
    println!("  - {} B2 Storage entries validated", b2_outputs.len());
    println!(
        "Generated {} total entries across all cloud types",
        all_outputs.len()
    );
}
