use crate::secrets::error::Result;
use azure_identity::DefaultAzureCredentialBuilder;
use azure_security_keyvault_secrets::SecretClient;
use std::fs;

/// Debug function to test Azure credential creation and connection
///
/// This function tries different methods of creating Azure credentials
/// and provides detailed output for debugging authentication issues.
pub fn debug_azure_credentials() -> Result<()> {
    println!("===== Azure KeyVault Connection Debugging =====");

    // Read credentials from test files
    println!("Reading credentials from test files...");
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
    
    println!("Credentials loaded:");
    println!("Client ID: {}", client_id);
    println!("Tenant ID: {}", tenant_id);
    println!("Client Secret length: {}", client_secret.len());
    println!("Vault URL: {}", vault_url);
    
    // ===== Method 1: Default Azure Credential with environment variables =====
    println!("\nMethod 1: DefaultAzureCredential with environment variables");
    
    // Set environment variables for authentication - using unsafe block for newer Rust
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
    
    println!("Creating DefaultAzureCredential...");
    match DefaultAzureCredentialBuilder::new()
        .exclude_azure_cli_credential()
        .build() {
            Ok(credential) => {
                println!("Credential created successfully");
                
                println!("Creating KeyVault client...");
                match SecretClient::new(&vault_url, credential, None) {
                    Ok(client) => {
                        println!("KeyVault client created successfully");
                        
                        // Test retrieving a secret
                        println!("Testing secret retrieval...");
                        let rt = tokio::runtime::Runtime::new()?;
                        match rt.block_on(client.get_secret("test-secret", "", None)) {
                            Ok(_) => println!("Successfully retrieved secret!"),
                            Err(e) => println!("Failed to retrieve secret: {}", e),
                        }
                    },
                    Err(e) => println!("Failed to create KeyVault client: {}", e),
                }
            },
            Err(e) => println!("Failed to create credential: {}", e),
        }
    
    // Method 2 removed since ClientSecretCredentialBuilder is not available in this Azure Identity version
    
    // Clean up environment variables - using unsafe block for newer Rust
    unsafe {
        std::env::remove_var("AZURE_TENANT_ID");
        std::env::remove_var("AZURE_CLIENT_ID");
        std::env::remove_var("AZURE_CLIENT_SECRET");
    }
    
    println!("\n===== Debug complete =====");
    Ok(())
}