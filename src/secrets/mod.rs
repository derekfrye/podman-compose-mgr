pub mod azure;
pub mod debug;
pub mod error;
pub mod models;
pub mod prompt;
pub mod utils;
pub mod validation;

use crate::Args;
use crate::secrets::error::Result;

/// Process secrets mode
///
/// Handles the different secret-related modes.
pub fn process_secrets_mode(args: &Args) -> Result<()> {
    match args.mode {
        crate::args::Mode::SecretRefresh => {
            azure::update_mode(args)?;
        }
        crate::args::Mode::SecretRetrieve => {
            validation::validate(args)?;
        }
        _ => {
            return Err(Box::<dyn std::error::Error>::from("Unsupported mode for secrets processing"));
        }
    }
    Ok(())
}

/// Test connection to Azure KeyVault
///
/// This function is used for manual testing and debugging of Azure KeyVault connections.
/// It attempts to create a KeyVault client and retrieve a test secret.
///
/// # Errors
///
/// Returns an error if:
/// - Unable to create KeyVault client
/// - Unable to retrieve the test secret
pub fn test_azure_connection(args: &Args) -> Result<()> {
    println!("Testing connection to Azure KeyVault...");
    
    // Get client for Azure KeyVault
    let (client, _) = validation::prepare_validation(args)?;
    
    println!("Successfully created KeyVault client.");
    
    // Try to retrieve a test secret
    let rt = tokio::runtime::Runtime::new()?;
    let secret_result = rt.block_on(azure::get_secret_value("test-secret", &client));
    
    match secret_result {
        Ok(secret) => {
            println!("Successfully retrieved test secret: {}", secret.name);
            println!("Secret ID: {}", secret.id);
        },
        Err(e) => {
            println!("Failed to retrieve test secret: {}", e);
            return Err(e);
        }
    }
    
    println!("Azure KeyVault connection test completed successfully.");
    Ok(())
}