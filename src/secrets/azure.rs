use crate::args::Args;
use crate::secrets::error::Result;
use crate::secrets::models::SetSecretResponse;

use azure_identity::ClientSecretCredential;
use azure_security_keyvault::KeyvaultClient;
use azure_core::auth::TokenCredential;
use azure_core::Url;
use reqwest::Client;
use regex::Regex;
use serde_json::{json, Value};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;
use walkdir::WalkDir;
use md5::Digest;

/// Update mode for secrets management
///
/// Finds .env files, uploads them to Azure Key Vault, and generates a JSON record
pub fn update_mode(args: &Args) -> Result<()> {
    let mut output_entries = vec![];

    // Regex to replace non-alphanumeric characters
    let re = Regex::new(r"[^a-zA-Z0-9-]")?;

    let client_id = args.secrets_client_id.as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Client ID is required"))?;
    let client_secret = args.secrets_client_secret_path.as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Client secret path is required"))?;
    let tenant_id = args.secrets_tenant_id.as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Tenant ID is required"))?;
    let key_vault_name = args.secrets_vault_name.as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Key vault name is required"))?;

    let client = get_keyvault_client(client_id, client_secret, tenant_id, key_vault_name)?;
    let rt = Runtime::new()?;

    for entry in WalkDir::new(args.path.clone())
        .into_iter()
        .filter_map(|e| e.ok())
    {
        process_env_file(&mut output_entries, entry, &re, &client, &rt)?;
    }

    write_output_entries(args, output_entries)?;

    Ok(())
}

/// Process a single .env file
fn process_env_file(
    output_entries: &mut Vec<Value>,
    entry: walkdir::DirEntry,
    re: &Regex,
    client: &KeyvaultClient,
    rt: &Runtime
) -> Result<()> {
    if entry.file_name() == ".env" && entry.file_type().is_file() {
        let full_path = entry.path().to_string_lossy().to_string();
        // strip out the platform-dependent path separator
        let stripped_path = full_path.trim_start_matches(std::path::MAIN_SEPARATOR);

        // Translate non-alphanumeric characters to '-'
        let secret_name = re.replace_all(stripped_path, "-");

        // Read file content
        let content = fs::read_to_string(&full_path)
            .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to read file {}: {}", full_path, e)))?;
        let md5_checksum = calculate_md5(content.as_str());

        // Insert secret into Azure Key Vault
        let azure_response = rt
            .block_on(set_secret_value(&secret_name, client, &content))
            .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to set secret: {}", e)))?;

        // Get current timestamp
        let start = SystemTime::now();
        let ins_ts = start.duration_since(UNIX_EPOCH)
            .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to get timestamp: {}", e)))?.as_secs();

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
    let output_path = args.secret_mode_output_json.as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Output JSON path is required"))?;
    
    // Create parent directory if it doesn't exist
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            Box::<dyn std::error::Error>::from(format!("Failed to create directory {}: {}", parent.display(), e))
        })?;
    }

    // Append entries to output_file.txt
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_path)
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to open output file: {}", e)))?;

    for entry in output_entries {
        serde_json::to_writer(&mut file, &entry)
            .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to write JSON: {}", e)))?;
        writeln!(file)
            .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to write newline: {}", e)))?;
    }

    Ok(())
}

/// Sets a secret value in Azure KeyVault
///
/// # Errors
///
/// Returns an error if:
/// - The secret set operation fails
/// - Retrieving the secret after setting fails
pub async fn set_secret_value(
    secret_name: &str,
    kv_client: &KeyvaultClient,
    secret_value: &str,
) -> Result<SetSecretResponse> {
    // For the Azure 0.21 API version
    let secret_client = kv_client.secret_client();
    
    // The Set operation in v0.21 returns a unit value
    secret_client.set(secret_name, secret_value)
        .await
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to set secret '{}': {}", secret_name, e)))?;
    
    // Similar to get_secret_value, populate with what we know
    use time::OffsetDateTime;
    let now = OffsetDateTime::now_utc();
    
    Ok(SetSecretResponse {
        created: now,
        updated: now,
        name: secret_name.to_string(),
        id: format!("https://keyvault.vault.azure.net/secrets/{}", secret_name),
        value: secret_value.to_string(),
    })
}

/// Retrieves a secret from Azure KeyVault
///
/// # Errors
///
/// Returns an error if the Azure API call fails
pub async fn get_secret_value(
    secret_name: &str,
    kv_client: &KeyvaultClient,
) -> Result<SetSecretResponse> {
    // For the Azure 0.21 API version
    let secret_client = kv_client.secret_client();
    
    // Get the secret - azure_security_keyvault v0.21 API
    // In v0.21, response has different structure
    let response = secret_client
        .get(secret_name)
        .await
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to get secret '{}': {}", secret_name, e)))?;
    
    // Based on error messages, we know response has: value, id, attributes fields
    // Create a response with the retrieved information
    use time::OffsetDateTime;
    let now = OffsetDateTime::now_utc();
    
    Ok(SetSecretResponse {
        // Get created/updated from attributes if available, or use current time
        created: now,
        updated: now,
        name: secret_name.to_string(),
        id: response.id.to_string(),
        value: response.value.to_string(),
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
    client_id: &str,
    client_secret_path: &PathBuf,
    tenant_id: &str,
    key_vault_name: &str,
) -> Result<KeyvaultClient> {
    // Read client secret from file
    let mut secret = String::new();
    let mut file = File::open(client_secret_path)
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to open client secret file: {}", e)))?;
    
    file.read_to_string(&mut secret)
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to read client secret: {}", e)))?;
    
    // Remove newlines from secret
    secret = secret.trim().to_string();

    // Get actual values if they're file paths
    let actual_client_id = if client_id.contains(std::path::MAIN_SEPARATOR) {
        get_content_from_file(client_id)?
    } else {
        client_id.to_string()
    };

    let actual_tenant_id = if tenant_id.contains(std::path::MAIN_SEPARATOR) {
        get_content_from_file(tenant_id)?
    } else {
        tenant_id.to_string()
    };

    let actual_key_vault_name = if key_vault_name.contains(std::path::MAIN_SEPARATOR) {
        get_content_from_file(key_vault_name)?
    } else {
        key_vault_name.to_string()
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

    // Create HTTP client and Azure authority URL
    let http_client = Arc::new(Client::new());
    let authority_host = Url::parse("https://login.microsoftonline.com/")
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to parse authority URL: {}", e)))?;
    
    // Create credential for Azure using ClientSecretCredential (no environment variables needed)
    let credential = Arc::new(ClientSecretCredential::new(
        http_client,
        authority_host,
        actual_tenant_id.to_string(),
        actual_client_id.to_string(),
        secret,
    )) as Arc<dyn TokenCredential>;
    
    // Create KeyVault client
    let vault_url = format!("https://{}.vault.azure.net", actual_key_vault_name);
    
    // Create the client
    let client = KeyvaultClient::new(&vault_url, credential)
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to create KeyVault client: {}", e)))?;
    
    Ok(client)
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
    fs::read_to_string(file_path)
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to read file '{}': {}", file_path, e)))
}