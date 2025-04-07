use podman_compose_mgr::{
    args::{self, Mode, initialization},
    secrets,
};
use serde_json::Value;
use std::fs::File;
use std::io::Read;
use tempfile::NamedTempFile;

#[test]
fn test_cloud_section_parsing() {
    // Test the new cloud section parsing functionality in initialization.rs
    let input_file = "tests/test7_new_init_fmt/test_for_init";
    let expected_output_file = "tests/test7_new_init_fmt/test_output.json";

    // First, process the input file using check_init_filepath to generate JSON
    let result = initialization::check_init_filepath(input_file);
    assert!(
        result.is_ok(),
        "Failed to process input file: {:?}",
        result.err()
    );

    let processed_path = result.unwrap();

    // Read the processed JSON file
    let mut processed_content = String::new();
    let mut file = File::open(&processed_path).unwrap();
    file.read_to_string(&mut processed_content).unwrap();

    // Read the expected output JSON
    let mut expected_content = String::new();
    let mut file = File::open(expected_output_file).unwrap();
    file.read_to_string(&mut expected_content).unwrap();

    // Parse both as JSON
    let processed_json: Vec<Value> = serde_json::from_str(&processed_content).unwrap();
    let expected_json: Vec<Value> = serde_json::from_str(&expected_content).unwrap();

    // Create a more comprehensive mapping with both destination_cloud and cloud_upload_bucket
    let expected_entries: std::collections::HashMap<String, serde_json::Value> = expected_json
        .iter()
        .map(|entry| (entry["filenm"].as_str().unwrap().to_string(), entry.clone()))
        .collect();

    let processed_entries: std::collections::HashMap<String, serde_json::Value> = processed_json
        .iter()
        .map(|entry| (entry["filenm"].as_str().unwrap().to_string(), entry.clone()))
        .collect();

    // Verify that each file in the expected output has the correct values in the processed output
    for (filename, expected_entry) in &expected_entries {
        assert!(
            processed_entries.contains_key(filename),
            "Missing file {} in processed output",
            filename
        );

        // Verify destination_cloud
        let expected_cloud = expected_entry["destination_cloud"].as_str().unwrap();
        let processed_cloud = processed_entries[filename]["destination_cloud"]
            .as_str()
            .unwrap();
        assert_eq!(
            processed_cloud, expected_cloud,
            "Mismatch for file {}: expected cloud {}, got {}",
            filename, expected_cloud, processed_cloud
        );

        // Verify cloud_upload_bucket if present in expected entry
        if expected_entry.get("cloud_upload_bucket").is_some() {
            let expected_bucket = expected_entry["cloud_upload_bucket"].as_str().unwrap();

            assert!(
                processed_entries[filename]
                    .get("cloud_upload_bucket")
                    .is_some(),
                "Missing cloud_upload_bucket for file {}",
                filename
            );

            let processed_bucket = processed_entries[filename]["cloud_upload_bucket"]
                .as_str()
                .unwrap();
            assert_eq!(
                processed_bucket, expected_bucket,
                "Mismatch for file {}: expected bucket {}, got {}",
                filename, expected_bucket, processed_bucket
            );
        }
    }

    // Now test that initialize.rs respects the cloud provider from the input JSON
    // Create a temporary file for the output JSON
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_path_buf();

    // Create Args with the necessary parameters
    let args = args::Args {
        mode: Mode::SecretInitialize,
        secrets_init_filepath: Some(processed_path),
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

    // Create a mapping from filename to entry for comparison
    let output_entries: std::collections::HashMap<String, serde_json::Value> = output_json
        .iter()
        .map(|entry| {
            (
                entry["file_nm"].as_str().unwrap().to_string(),
                entry.clone(),
            )
        })
        .collect();

    // Verify that each file in the expected output has the correct values in the final output
    for (filename, expected_entry) in &expected_entries {
        assert!(
            output_entries.contains_key(filename),
            "Missing file {} in final output",
            filename
        );

        // Verify destination_cloud
        let expected_cloud = expected_entry["destination_cloud"].as_str().unwrap();
        let output_cloud = output_entries[filename]["destination_cloud"]
            .as_str()
            .unwrap();
        assert_eq!(
            output_cloud, expected_cloud,
            "Mismatch for file {}: expected cloud {}, got {}",
            filename, expected_cloud, output_cloud
        );

        // Verify cloud_upload_bucket if present in expected entry and if cloud is r2 or b2
        if expected_entry.get("cloud_upload_bucket").is_some()
            && (expected_cloud == "r2" || expected_cloud == "b2")
        {
            let expected_bucket = expected_entry["cloud_upload_bucket"].as_str().unwrap();

            let output_bucket = output_entries[filename]["cloud_upload_bucket"]
                .as_str()
                .unwrap();
            assert_eq!(
                output_bucket, expected_bucket,
                "Mismatch for file {}: expected bucket {}, got {}",
                filename, expected_bucket, output_bucket
            );
        }
    }
}
