use podman_compose_mgr::{
    args::{self, initialization, Mode},
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
    assert!(result.is_ok(), "Failed to process input file: {:?}", result.err());
    
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
    
    // Create mappings from filename to destination_cloud for comparison
    let processed_map: std::collections::HashMap<String, String> = processed_json
        .iter()
        .map(|entry| {
            (
                entry["filenm"].as_str().unwrap().to_string(),
                entry["destination_cloud"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    
    let expected_map: std::collections::HashMap<String, String> = expected_json
        .iter()
        .map(|entry| {
            (
                entry["filenm"].as_str().unwrap().to_string(),
                entry["destination_cloud"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    
    // Verify that each file in the expected output has the correct destination_cloud in the processed output
    for (filename, expected_cloud) in &expected_map {
        assert!(
            processed_map.contains_key(filename),
            "Missing file {} in processed output",
            filename
        );
        
        assert_eq!(
            processed_map[filename], *expected_cloud,
            "Mismatch for file {}: expected cloud {}, got {}",
            filename, expected_cloud, processed_map[filename]
        );
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
    
    // Create a mapping from filename to destination_cloud for comparison
    let output_map: std::collections::HashMap<String, String> = output_json
        .iter()
        .map(|entry| {
            (
                entry["file_nm"].as_str().unwrap().to_string(),
                entry["destination_cloud"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    
    // Verify that each file in the expected output has the correct destination_cloud in the processed output
    for (filename, expected_cloud) in &expected_map {
        assert!(
            output_map.contains_key(filename),
            "Missing file {} in final output",
            filename
        );
        
        assert_eq!(
            output_map[filename], *expected_cloud,
            "Mismatch for file {}: expected cloud {}, got {}",
            filename, expected_cloud, output_map[filename]
        );
    }
}