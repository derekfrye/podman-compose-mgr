use podman_compose_mgr::args::Args;
use podman_compose_mgr::secrets::file_details::check_encoding_and_size;
use podman_compose_mgr::secrets::utils::{calculate_content_hash, calculate_hash};
use podman_compose_mgr::secrets::validation::file_ops::decode_base64_to_tempfile;
use std::fs;

#[test]
fn test_file_encoding_detection() {
    // Test file 'a' which is ASCII and should not need base64 encoding
    let (encoding, orig_size, encoded_size) =
        check_encoding_and_size("tests/test3_and_test4/a").expect("Failed to check encoding");

    assert_eq!(encoding, "utf8", "File 'a' should be detected as utf8");
    assert_eq!(
        orig_size, encoded_size,
        "UTF-8 file should have same original and encoded size"
    );

    // Test file 'e e' which is binary data and should need base64 encoding
    let (encoding, orig_size, encoded_size) =
        check_encoding_and_size("tests/test3_and_test4/e e").expect("Failed to check encoding");

    assert_eq!(
        encoding, "base64",
        "File 'e e' should be detected as base64"
    );
    assert!(
        encoded_size > orig_size,
        "Base64 encoded file should be larger than original"
    );

    // Check that base64 file was created
    let base64_path = "tests/test3_and_test4/e e.base64";
    assert!(
        std::path::Path::new(base64_path).exists(),
        "Base64 file should exist"
    );
}

#[test]
fn test_base64_decode_and_hash_match() {
    // Create default Args instance with standard temp_file_path
    let args = Args::default();

    // Calculate content hash of original binary file
    let original_content_hash = calculate_content_hash("tests/test3_and_test4/e e")
        .expect("Failed to hash original file content");

    // Read the base64 file
    let base64_content =
        fs::read("tests/test3_and_test4/e e.base64").expect("Failed to read base64 file");

    // Decode to temporary file - passing args as second parameter
    let temp_file =
        decode_base64_to_tempfile(&base64_content, &args).expect("Failed to decode base64 content");

    // Calculate content hash of the decoded file
    let temp_path = temp_file.path().to_str().unwrap();
    let decoded_content_hash =
        calculate_content_hash(temp_path).expect("Failed to hash decoded file content");

    // The content hashes should match
    assert_eq!(
        original_content_hash, decoded_content_hash,
        "Original and decoded file content hashes should match"
    );

    // For demonstration, show the path hashes are different
    let original_path_hash =
        calculate_hash("tests/test3_and_test4/e e").expect("Failed to hash original file path");
    let temp_path_hash = calculate_hash(temp_path).expect("Failed to hash temp file path");

    assert_ne!(
        original_path_hash, temp_path_hash,
        "Original and temp file path hashes should not match"
    );

    println!("Original file path hash: {}", original_path_hash);
    println!("Temp file path hash: {}", temp_path_hash);
    println!("Content hash (same for both): {}", original_content_hash);

    // Also verify with direct content comparison
    let original_content =
        fs::read("tests/test3_and_test4/e e").expect("Failed to read original file");
    let decoded_content = fs::read(temp_path).expect("Failed to read decoded file");

    assert_eq!(
        original_content, decoded_content,
        "Original and decoded file contents should match"
    );
}
