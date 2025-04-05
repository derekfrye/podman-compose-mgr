use std::fs;
use azure_identity::DefaultAzureCredentialBuilder;
use azure_security_keyvault_secrets::SecretClient;

// This test explicitly tests the Azure Identity v0.22 authentication
// It's designed to isolate and debug Azure credential issues
#[test]
#[ignore]
fn test_azure_credential_v022() -> Result<(), Box<dyn std::error::Error>> {
    // Read credentials from test files
    let client_id = fs::read_to_string("tests/personal_testing_data/client_id.txt")?.trim().to_string();
    let tenant_id = fs::read_to_string("tests/personal_testing_data/tenant_id.txt")?.trim().to_string();
    let client_secret = fs::read_to_string("tests/personal_testing_data/secrets.txt")?.trim().to_string();
    
    // Read and parse vault name
    let vault_url_or_name = fs::read_to_string("tests/personal_testing_data/vault_name.txt")?.trim().to_string();
    
    // Format the vault URL correctly
    let vault_url = if vault_url_or_name.contains("vault.azure.net") {
        // Already a URL, use as is
        vault_url_or_name
    } else {
        // Just a vault name, format as URL
        format!("https://{}.vault.azure.net", vault_url_or_name)
    };
    
    println!("Test setup:");
    println!("Client ID: {}", client_id);
    println!("Tenant ID: {}", tenant_id);
    println!("Client Secret length: {}", client_secret.len());
    println!("Vault URL: {}", vault_url);
    
    // Set environment variables for authentication - using unsafe for newer Rust
    unsafe {
        std::env::set_var("AZURE_TENANT_ID", &tenant_id);
        std::env::set_var("AZURE_CLIENT_ID", &client_id);
        std::env::set_var("AZURE_CLIENT_SECRET", &client_secret);
    }
    
    // Confirm environment variables are set correctly
    println!("Environment variables set:");
    println!("AZURE_TENANT_ID={}", std::env::var("AZURE_TENANT_ID")?);
    println!("AZURE_CLIENT_ID={}", std::env::var("AZURE_CLIENT_ID")?);
    println!("AZURE_CLIENT_SECRET length={}", std::env::var("AZURE_CLIENT_SECRET")?.len());
    
    println!("Building credential...");
    let credential = DefaultAzureCredentialBuilder::new()
        .exclude_azure_cli_credential()
        .build()?;
    
    println!("Credential created successfully");
    
    println!("Creating KeyVault client...");
    let client = SecretClient::new(&vault_url, credential, None)?;
    println!("KeyVault client created successfully");
    
    // Test retrieving a secret to verify full authentication works
    println!("Testing secret retrieval...");
    let rt = tokio::runtime::Runtime::new()?;
    let secret_result = rt.block_on(client.get_secret("test-secret", "", None));
    
    match secret_result {
        Ok(_) => println!("Successfully retrieved secret!"),
        Err(e) => println!("Failed to retrieve secret: {}", e),
    }
    
    // Clean up environment variables - using unsafe for newer Rust
    unsafe {
        std::env::remove_var("AZURE_TENANT_ID");
        std::env::remove_var("AZURE_CLIENT_ID");
        std::env::remove_var("AZURE_CLIENT_SECRET");
    }
    
    // If we got here, we successfully created the credential
    Ok(())
}