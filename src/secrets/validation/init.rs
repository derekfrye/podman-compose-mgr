use crate::args::Args;
use crate::interfaces::AzureKeyVaultClient;
use crate::secrets::azure::get_content_from_file;
use crate::secrets::azure::get_keyvault_client;
use crate::secrets::error::Result;
use serde_json::Value;
use std::fs::File;
use std::io::Read;

/// Prepare for validation by reading the input file and creating a KeyVault client
pub fn prepare_validation(args: &Args) -> Result<(Box<dyn AzureKeyVaultClient>, Vec<Value>)> {
    // Get input file path
    let input_path = args
        .input_json
        .as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Input JSON path is required"))?;

    // Read and validate JSON entries
    let mut file = File::open(input_path).map_err(|e| {
        Box::<dyn std::error::Error>::from(format!("Failed to open input JSON file: {}", e))
    })?;

    let mut file_content = String::new();
    file.read_to_string(&mut file_content).map_err(|e| {
        Box::<dyn std::error::Error>::from(format!("Failed to read input JSON file: {}", e))
    })?;

    let json_values: Vec<Value> = serde_json::from_str(&file_content)
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to parse JSON: {}", e)))?;

    // Get Azure credentials
    let client_id_path = args
        .secrets_client_id
        .as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Client ID is required"))?;
    let client_secret = args
        .secrets_client_secret_path
        .as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Client secret path is required"))?;
    let tenant_id_path = args
        .secrets_tenant_id
        .as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Tenant ID is required"))?;
    let vault_name_path = args
        .secrets_vault_name
        .as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Key vault name is required"))?;

    // Get KeyVault client
    let client = get_keyvault_client(client_id_path, client_secret, tenant_id_path, vault_name_path)?;

    Ok((client, json_values))
}

/// Get client ID from args or file
pub fn get_client_id(args: &Args) -> Result<String> {
    let client_id_path = args
        .secrets_client_id
        .as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Client ID is required"))?;

    // Read content from file path
    client_id_path.to_str()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Invalid client ID path"))
        .and_then(|path| get_content_from_file(path))
}

/// Get tenant ID from args or file
pub fn get_tenant_id(args: &Args) -> Result<String> {
    let tenant_id_path = args
        .secrets_tenant_id
        .as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Tenant ID is required"))?;

    // Read content from file path
    tenant_id_path.to_str()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Invalid tenant ID path"))
        .and_then(|path| get_content_from_file(path))
}

/// Get key vault name from args or file
pub fn get_key_vault_name(args: &Args) -> Result<String> {
    let vault_name_path = args
        .secrets_vault_name
        .as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Key vault name is required"))?;

    // Read content from file path
    vault_name_path.to_str()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Invalid vault name path"))
        .and_then(|path| get_content_from_file(path))
}
