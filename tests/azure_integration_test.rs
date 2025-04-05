use podman_compose_mgr::args::Args;
use podman_compose_mgr::secrets::validation;
use podman_compose_mgr::secrets::azure::get_content_from_file;
use clap::Parser;

// This is a true integration test that requires real Azure credentials
// Since it requires real credentials, it's marked as ignored by default
// To run it: cargo test --test azure_integration_test -- --ignored
#[test]
#[ignore]
fn test_azure_connection() -> Result<(), Box<dyn std::error::Error>> {
    // Create command line arguments similar to what would be passed on the command line
    let args = vec![
        "dummy_binary".to_string(),
        "--path".to_string(),
        "~/docker".to_string(),
        "--mode".to_string(),
        "secret-retrieve".to_string(),
        "--secrets-client-id".to_string(),
        "tests/personal_testing_data/client_id.txt".to_string(),
        "--secrets-client-secret-path".to_string(),
        "tests/personal_testing_data/secrets.txt".to_string(),
        "--secrets-tenant-id".to_string(),
        "tests/personal_testing_data/tenant_id.txt".to_string(),
        "--secrets-vault-name".to_string(),
        "tests/personal_testing_data/vault_name.txt".to_string(),
        "--secret-mode-output-json".to_string(),
        "tests/personal_testing_data/outfile.json".to_string(),
        "--secret-mode-input-json".to_string(),
        "tests/personal_testing_data/input.json".to_string(),
        "--verbose".to_string()
    ];

    // Parse the arguments
    let args = Args::parse_from(args);

    // Call prepare_validation function that creates the Azure client
    let result = validation::prepare_validation(&args);

    // Check if the result is Ok, this implies the Azure connection was successful
    assert!(result.is_ok(), "prepare_validation failed: {:?}", result.err());

    // If we got a successful result, unwrap it to get the client and JSON values
    let (client, json_values) = result.unwrap();

    // Verify we got the JSON values from the input file
    assert!(!json_values.is_empty(), "No JSON values were loaded from the input file");

    // Try to retrieve a secret to verify the client works
    let first_entry = &json_values[0];
    
    // Extract the Azure secret name from the first entry
    let az_name = first_entry["az_name"].as_str().unwrap();
    
    // Create a tokio runtime to run the async get_secret_value function
    let rt = tokio::runtime::Runtime::new().unwrap();
    let secret_result = rt.block_on(podman_compose_mgr::secrets::azure::get_secret_value(
        az_name,
        &client,
    ));
    
    // Verify that the secret retrieval was successful
    assert!(secret_result.is_ok(), "Failed to retrieve secret: {:?}", secret_result.err());

    // Success! The test passes if we make it here
    Ok(())
}

// Test individual functions from the azure module for better isolation
#[test]
fn test_file_content_reading() {
    // Test reading client ID from file
    let client_id_file = "tests/personal_testing_data/client_id.txt";
    let client_id = get_content_from_file(client_id_file).expect("Failed to read client ID");
    assert!(!client_id.is_empty(), "Client ID should not be empty");
    // Client ID should be approximately UUID length (allowing for some variation)
    assert!(client_id.len() >= 32 && client_id.len() <= 40, 
            "Client ID should be a valid ID, got length: {}", client_id.len());

    // Test reading tenant ID from file
    let tenant_id_file = "tests/personal_testing_data/tenant_id.txt";
    let tenant_id = get_content_from_file(tenant_id_file).expect("Failed to read tenant ID");
    assert!(!tenant_id.is_empty(), "Tenant ID should not be empty");
    // Tenant ID should be approximately UUID length (allowing for some variation)
    assert!(tenant_id.len() >= 32 && tenant_id.len() <= 40, 
            "Tenant ID should be a valid ID, got length: {}", tenant_id.len());

    // Test reading vault name from file
    let vault_name_file = "tests/personal_testing_data/vault_name.txt";
    let vault_name = get_content_from_file(vault_name_file).expect("Failed to read vault name");
    assert!(!vault_name.is_empty(), "Vault name should not be empty");
    // Note: The vault name could be just the name part without the domain
    assert!(vault_name.len() > 3, "Vault name should be a valid name");
}