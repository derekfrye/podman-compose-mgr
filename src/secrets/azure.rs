use crate::args::Args;
use crate::interfaces::{AzureKeyVaultClient, DefaultAzureKeyVaultClient};
use crate::secrets::error::Result;
use crate::secrets::models::SetSecretResponse;

use azure_core::credentials::{Secret, TokenCredential};
use azure_identity::ClientSecretCredential;
use azure_security_keyvault_secrets::{SecretClient, SecretClientOptions};
use md5::Digest;
use regex::Regex;
use reqwest::Client;
use serde_json::{Value, json};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;
use walkdir::WalkDir;

/// Update mode for secrets management
///
/// Finds .env files, uploads them to Azure Key Vault, and generates a JSON record
pub fn update_mode(args: &Args) -> Result<()> {
    let mut output_entries = vec![];

    // Regex to replace non-alphanumeric characters
    let re = Regex::new(r"[^a-zA-Z0-9-]")?;

    let client_id = args
        .azure_client_id_path
        .as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Client ID is required"))?;
    let client_secret = args
        .azure_client_secret_path
        .as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Client secret path is required"))?;
    let tenant_id = args
        .azure_tenant_id_path
        .as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Tenant ID is required"))?;
    let key_vault_name = args
        .azure_vault_name_path
        .as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Key vault name is required"))?;

    let client = get_keyvault_client(client_id, client_secret, tenant_id, key_vault_name)?;
    let rt = Runtime::new()?;

    for entry in WalkDir::new(args.path.clone())
        .into_iter()
        .filter_map(|e| e.ok())
    {
        process_env_file(&mut output_entries, entry, &re, client.as_ref(), &rt)?;
    }

    write_output_entries(args, output_entries)?;

    Ok(())
}

/// Process a single .env file
fn process_env_file(
    output_entries: &mut Vec<Value>,
    entry: walkdir::DirEntry,
    re: &Regex,
    client: &dyn AzureKeyVaultClient,
    _rt: &Runtime,
) -> Result<()> {
    if entry.file_name() == ".env" && entry.file_type().is_file() {
        let full_path = entry.path().to_string_lossy().to_string();
        // strip out the platform-dependent path separator
        let stripped_path = full_path.trim_start_matches(std::path::MAIN_SEPARATOR);

        // Translate non-alphanumeric characters to '-'
        let secret_name = re.replace_all(stripped_path, "-");

        // Read file content
        let content = fs::read_to_string(&full_path).map_err(|e| {
            Box::<dyn std::error::Error>::from(format!("Failed to read file {}: {}", full_path, e))
        })?;
        let md5_checksum = calculate_md5(content.as_str());

        // Insert secret into Azure Key Vault using the interface
        let azure_response = client
            .set_secret_value(&secret_name, &content)
            .map_err(|e| {
                Box::<dyn std::error::Error>::from(format!("Failed to set secret: {}", e))
            })?;

        // Get current timestamp
        let start = SystemTime::now();
        let ins_ts = start
            .duration_since(UNIX_EPOCH)
            .map_err(|e| {
                Box::<dyn std::error::Error>::from(format!("Failed to get timestamp: {}", e))
            })?
            .as_secs();

        // Build output entry
        let output_entry = json!({
            "file_nm": full_path,
            "md5": md5_checksum,
            "ins_ts": ins_ts,
            "az_id": azure_response.id,
            "az_create": azure_response.created,
            "az_updated": azure_response.updated,
            "az_name": azure_response.name
        });

        output_entries.push(output_entry);
    }

    Ok(())
}

/// Write output entries to the specified JSON file
fn write_output_entries(args: &Args, output_entries: Vec<Value>) -> Result<()> {
    // Make sure output path exists
    let output_path = args
        .output_json
        .as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Output JSON path is required"))?;

    // Create parent directory if it doesn't exist
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            Box::<dyn std::error::Error>::from(format!(
                "Failed to create directory {}: {}",
                parent.display(),
                e
            ))
        })?;
    }

    // Append entries to output_file.txt
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_path)
        .map_err(|e| {
            Box::<dyn std::error::Error>::from(format!("Failed to open output file: {}", e))
        })?;

    for entry in output_entries {
        serde_json::to_writer(&mut file, &entry).map_err(|e| {
            Box::<dyn std::error::Error>::from(format!("Failed to write JSON: {}", e))
        })?;
        writeln!(file).map_err(|e| {
            Box::<dyn std::error::Error>::from(format!("Failed to write newline: {}", e))
        })?;
    }

    Ok(())
}

/// Sets a secret value in Azure KeyVault
///
/// # Errors
///
/// Returns an error if:
/// - The secret set operation fails
///
/// This function is used by the DefaultAzureKeyVaultClient implementation
pub async fn set_secret_value(
    secret_name: &str,
    secret_client: &SecretClient,
    secret_value: &str,
) -> Result<SetSecretResponse> {
    // Get the secret value as a raw string
    let set_response = secret_client
        .set_secret(secret_name, secret_value)
        .await
        .map_err(|e| {
            Box::<dyn std::error::Error>::from(format!(
                "Failed to set secret '{}': {}",
                secret_name, e
            ))
        })?;

    // Extract data from the response
    let now = time::OffsetDateTime::now_utc();
    
    // Access properties from the response
    let created = set_response.properties.created_on.unwrap_or(now);
    let updated = set_response.properties.updated_on.unwrap_or(now);
    
    // Create our SetSecretResponse struct
    Ok(SetSecretResponse {
        created,
        updated,
        name: set_response.name,
        id: set_response.id,
        value: secret_value.to_string(),
    })
}

/// Retrieves a secret from Azure KeyVault
///
/// # Errors
///
/// Returns an error if the Azure API call fails
///
/// This function is used by the DefaultAzureKeyVaultClient implementation
pub async fn get_secret_value(
    secret_name: &str,
    secret_client: &SecretClient,
) -> Result<SetSecretResponse> {
    // Get the secret using the v0.2 API
    let get_response = secret_client
        .get_secret(secret_name, None)
        .await
        .map_err(|e| {
            Box::<dyn std::error::Error>::from(format!(
                "Failed to get secret '{}': {}",
                secret_name, e
            ))
        })?;

    // Extract data from the response
    let now = time::OffsetDateTime::now_utc();
    
    // Access properties from the response
    let created = get_response.properties.created_on.unwrap_or(now);
    let updated = get_response.properties.updated_on.unwrap_or(now);
    
    // Create our SetSecretResponse struct
    Ok(SetSecretResponse {
        created,
        updated,
        name: get_response.name,
        id: get_response.id,
        value: get_response.value,
    })
}

/// Get a KeyVault client for Azure operations
///
/// # Errors
///
/// Returns an error if:
/// - Unable to read the client secret file
/// - Unable to create the Azure credential
/// - Unable to create the KeyVault client
pub fn get_keyvault_client(
    client_id_path: &Path,
    client_secret_path: &Path,
    tenant_id: &Path,
    key_vault_name: &Path,
) -> Result<Box<dyn AzureKeyVaultClient>> {
    // Read client secret from file
    let mut secret = String::new();
    let mut file = File::open(client_secret_path).map_err(|e| {
        Box::<dyn std::error::Error>::from(format!("Failed to open client secret file: {}", e))
    })?;

    file.read_to_string(&mut secret).map_err(|e| {
        Box::<dyn std::error::Error>::from(format!("Failed to read client secret: {}", e))
    })?;

    // Remove newlines from secret
    secret = secret.trim().to_string();

    // Read client ID from file
    let actual_client_id = match client_id_path.to_str() {
        Some(path) => get_content_from_file(path)?,
        None => return Err(Box::<dyn std::error::Error>::from("Invalid client ID path")),
    };

    // Read tenant ID from file
    let actual_tenant_id = match tenant_id.to_str() {
        Some(path) => get_content_from_file(path)?,
        None => return Err(Box::<dyn std::error::Error>::from("Invalid tenant ID path")),
    };

    // Read key vault name from file
    let actual_key_vault_name = match key_vault_name.to_str() {
        Some(path) => get_content_from_file(path)?,
        None => {
            return Err(Box::<dyn std::error::Error>::from(
                "Invalid key vault name path",
            ));
        }
    };

    // Strip out any URL components from the key vault name to get just the vault name
    let actual_key_vault_name = if actual_key_vault_name.contains("vault.azure.net") {
        // Extract the vault name from the URL
        let parts: Vec<&str> = actual_key_vault_name.split("//").collect();
        if parts.len() > 1 {
            let domain_parts: Vec<&str> = parts[1].split('.').collect();
            if !domain_parts.is_empty() {
                domain_parts[0].to_string()
            } else {
                actual_key_vault_name
            }
        } else {
            actual_key_vault_name
        }
    } else {
        actual_key_vault_name
    };

    // Create credential for Azure using ClientSecretCredential with v0.23 API
    let credential = Arc::new(ClientSecretCredential::new(
        actual_tenant_id.trim(),
        actual_client_id.trim(),
        Secret::new(secret),
        None, // Default options
    )) as Arc<dyn TokenCredential>;

    // Create KeyVault client URL
    let vault_url = format!("https://{}.vault.azure.net", actual_key_vault_name);

    // Create the concrete SecretClient from the SDK v0.2
    let secret_client = SecretClient::new(
        &vault_url,
        credential,
        SecretClientOptions::default(),
    ).map_err(|e| {
        Box::<dyn std::error::Error>::from(format!("Failed to create KeyVault SecretClient: {}", e))
    })?;

    // Wrap in our interface implementation
    let client = DefaultAzureKeyVaultClient::new(secret_client);

    Ok(Box::new(client))
}

/// Calculate MD5 hash for a string
pub fn calculate_md5(content: &str) -> String {
    let mut hasher = md5::Md5::new();
    md5::Digest::update(&mut hasher, content);
    format!("{:x}", md5::Digest::finalize(hasher))
}

/// Read content from a file
///
/// # Errors
///
/// Returns an error if the file cannot be read
pub fn get_content_from_file(file_path: &str) -> Result<String> {
    fs::read_to_string(file_path).map_err(|e| {
        Box::<dyn std::error::Error>::from(format!("Failed to read file '{}': {}", file_path, e))
    })
}
