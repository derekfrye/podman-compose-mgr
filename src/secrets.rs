use crate::args::Args;
use crate::read_val::{self, GrammarFragment, GrammarType};

use chrono::{DateTime, Local, TimeZone, Utc};
use md5::{Digest, Md5};
use regex::Regex;
use reqwest::{Client, Url};
use serde::Serialize;
use serde_json::{Value, json};
use std::error::Error;
use std::fs::File;
use std::path::PathBuf;
use std::{fs, path};
use std::io::{Read, Write};
use azure_identity::ClientSecretCredential;
use azure_security_keyvault::{KeyvaultClient, SecretClient};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::runtime::Runtime;
use walkdir::WalkDir;
use time::OffsetDateTime;

#[derive(Debug, Error)]
pub enum SecretError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Azure Key Vault error: {0}")]
    KeyVault(String),
    
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
    
    #[error("Runtime error: {0}")]
    Runtime(String),
    
    #[error("Field missing in JSON: {0}")]
    MissingField(String),
    
    #[error("Time parse error: {0}")]
    TimeError(String),
    
    #[error("Hostname error: {0}")]
    HostnameError(String),
    
    #[error("Path error: {0}")]
    PathError(String),
    
    #[error("Url parse error: {0}")]
    UrlError(String),
    
    #[error("Generic error: {0}")]
    Other(String),
}

// Use Box<dyn Error> for public functions for backward compatibility
pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

struct SetSecretResponse {
    created: OffsetDateTime,
    updated: OffsetDateTime,
    name: String,
    id: String,
    value: String,
}

#[derive(Serialize)]
struct JsonOutput {
    file_nm: String,
    md5: String,
    ins_ts: String,
    az_id: String,
    az_create: String,
    az_updated: String,
    az_name: String,
    hostname: String,
}

struct JsonOutputControl {
    json_output: JsonOutput,
    validate_all: bool,
}

impl JsonOutputControl {
    fn new() -> JsonOutputControl {
        JsonOutputControl {
            json_output: JsonOutput {
                file_nm: String::new(),
                md5: String::new(),
                ins_ts: String::new(),
                az_id: String::new(),
                az_create: String::new(),
                az_updated: String::new(),
                az_name: String::new(),
                hostname: String::new(),
            },
            validate_all: false,
        }
    }
}

pub fn update_mode(args: &Args) -> std::result::Result<(), Box<dyn Error>> {
    let mut output_entries = vec![];

    // Regex to replace non-alphanumeric characters
    let re = Regex::new(r"[^a-zA-Z0-9-]")?;

    let client_id = args.secrets_client_id.as_ref()
        .ok_or_else(|| Box::<dyn Error>::from("Client ID is required"))?;
    let client_secret = args.secrets_client_secret_path.as_ref()
        .ok_or_else(|| Box::<dyn Error>::from("Client secret path is required"))?;
    let tenant_id = args.secrets_tenant_id.as_ref()
        .ok_or_else(|| Box::<dyn Error>::from("Tenant ID is required"))?;
    let key_vault_name = args.secrets_vault_name.as_ref()
        .ok_or_else(|| Box::<dyn Error>::from("Key vault name is required"))?;

    let client = get_keyvault_secret_client(client_id, client_secret, tenant_id, key_vault_name)?;

    let rt = Runtime::new()?;

    for entry in WalkDir::new(args.path.clone())
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_name() == ".env" && entry.file_type().is_file() {
            let full_path = entry.path().to_string_lossy().to_string();
            // strip out the platform-dependent path separator
            let stripped_path = full_path.trim_start_matches(std::path::MAIN_SEPARATOR);

            // Translate non-alphanumeric characters to '-'
            let secret_name = re.replace_all(stripped_path, "-");

            // Read file content
            let content = fs::read_to_string(&full_path)
                .map_err(|e| Box::<dyn Error>::from(format!("Failed to read file {}: {}", full_path, e)))?;
            let md5_checksum = calculate_md5(content.as_str());

            // Insert secret into Azure Key Vault
            let azure_response = rt
                .block_on(set_secret_value(&secret_name, &client, &content))
                .map_err(|e| Box::<dyn Error>::from(format!("Failed to set secret: {}", e)))?;

            // Get current timestamp
            let start = SystemTime::now();
            let ins_ts = start.duration_since(UNIX_EPOCH)
                .map_err(|e| Box::<dyn Error>::from(format!("Failed to get timestamp: {}", e)))?.as_secs();

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
    }

    // Make sure output path exists
    let output_path = args.secret_mode_output_json.as_ref()
        .ok_or_else(|| Box::<dyn Error>::from("Output JSON path is required"))?;
    
    // Create parent directory if it doesn't exist
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            Box::<dyn Error>::from(format!("Failed to create directory {}: {}", parent.display(), e))
        })?;
    }

    // Append entries to output_file.txt
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_path)
        .map_err(|e| Box::<dyn Error>::from(format!("Failed to open output file: {}", e)))?;

    for entry in output_entries {
        serde_json::to_writer(&mut file, &entry)
            .map_err(|e| Box::<dyn Error>::from(format!("Failed to write JSON: {}", e)))?;
        writeln!(file)
            .map_err(|e| Box::<dyn Error>::from(format!("Failed to write newline: {}", e)))?;
    }

    Ok(())
}

/// Validates secrets stored in Azure KeyVault against local files
///
/// # Errors
///
/// Returns an error if:
/// - Unable to read the input JSON file
/// - JSON parsing fails
/// - Required arguments are missing
/// - KeyVault client creation fails
pub fn validate(args: &Args) -> std::result::Result<(), Box<dyn Error>> {
    // Get input file path
    let input_path = args.secret_mode_input_json.as_ref()
        .ok_or_else(|| Box::<dyn Error>::from("Input JSON path is required"))?;
    
    // Read and validate JSON entries
    let mut file = File::open(input_path)
        .map_err(|e| Box::<dyn Error>::from(format!("Failed to open input JSON file: {}", e)))?;
    
    let mut file_content = String::new();
    file.read_to_string(&mut file_content)
        .map_err(|e| Box::<dyn Error>::from(format!("Failed to read input JSON file: {}", e)))?;
    
    let json_values: Vec<Value> = serde_json::from_str(&file_content)
        .map_err(|e| Box::<dyn Error>::from(format!("Failed to parse JSON: {}", e)))?;

    // Handle client ID (from arg or file)
    let client_id_arg = args.secrets_client_id.as_ref()
        .ok_or_else(|| Box::<dyn Error>::from("Client ID is required"))?;
    
    let client_id = if client_id_arg.contains(path::MAIN_SEPARATOR) {
        get_content_from_file(client_id_arg)?
    } else {
        client_id_arg.clone()
    };
    
    // Handle client secret path
    let client_secret = args.secrets_client_secret_path.as_ref()
        .ok_or_else(|| Box::<dyn Error>::from("Client secret path is required"))?;
    
    // Handle tenant ID (from arg or file)
    let tenant_id_arg = args.secrets_tenant_id.as_ref()
        .ok_or_else(|| Box::<dyn Error>::from("Tenant ID is required"))?;
    
    let tenant_id = if tenant_id_arg.contains(path::MAIN_SEPARATOR) {
        get_content_from_file(tenant_id_arg)?
    } else {
        tenant_id_arg.clone()
    };
    
    // Handle key vault name (from arg or file)
    let key_vault_name_arg = args.secrets_vault_name.as_ref()
        .ok_or_else(|| Box::<dyn Error>::from("Key vault name is required"))?;
    
    let key_vault_name = if key_vault_name_arg.contains(path::MAIN_SEPARATOR) {
        get_content_from_file(key_vault_name_arg)?
    } else {
        key_vault_name_arg.clone()
    };

    // Get KeyVault client
    let client = get_keyvault_secret_client(&client_id, client_secret, &tenant_id, &key_vault_name)?;
    let mut json_outputs: Vec<JsonOutput> = vec![];

    // Process each entry
    let mut loop_result: JsonOutputControl = JsonOutputControl::new();
    for entry in json_values {
        if loop_result.validate_all {
            match validate_entry(entry, &client, args) {
                Ok(z) => json_outputs.push(z),
                Err(e) => eprintln!("Error validating entry: {}", e),
            }
            continue;
        } else {
            match read_val_loop(entry, &client, args) {
                Ok(result) => {
                    json_outputs.push(result.json_output);
                    loop_result.validate_all = result.validate_all;
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                }
            }
        }
    }

    // Write output if we have results
    if !json_outputs.is_empty() {
        if let Some(output_path) = args.secret_mode_output_json.as_ref() {
            if let Some(output_dir) = output_path.parent() {
                fs::create_dir_all(output_dir)
                    .map_err(|e| Box::<dyn Error>::from(format!("Failed to create output directory: {}", e)))?;
            }
            
            let output_str = output_path.to_str()
                .ok_or_else(|| Box::<dyn Error>::from("Invalid UTF-8 in output path"))?;
                
            write_json_output(&json_outputs, output_str)?;
        } else {
            return Err(Box::<dyn Error>::from("Output JSON path is required"));
        }
    }

    Ok(())
}

fn read_val_loop(
    entry: Value,
    client: &SecretClient,
    args: &Args,
) -> Result<JsonOutputControl> {
    let mut grammars: Vec<GrammarFragment> = vec![];
    let mut output_control: JsonOutputControl = JsonOutputControl {
        json_output: JsonOutput {
            file_nm: String::new(),
            md5: String::new(),
            ins_ts: String::new(),
            az_id: String::new(),
            az_create: String::new(),
            az_updated: String::new(),
            az_name: String::new(),
            hostname: String::new(),
        },
        validate_all: false,
    };
    let static_prompt_grammar = GrammarFragment {
        original_val_for_prompt: Some("Check".to_string()),
        shortened_val_for_prompt: None,
        pos: 0,
        prefix: None,
        suffix: Some(" ".to_string()),
        grammar_type: GrammarType::Verbiage,
        can_shorten: false,
        display_at_all: true,
    };
    grammars.push(static_prompt_grammar);

    let file_name = crate::utils::json_utils::extract_string_field(&entry, "file_nm")?;

    let file_nm_grammar = GrammarFragment {
        original_val_for_prompt: Some(file_name.to_string()),
        shortened_val_for_prompt: None,
        pos: 1,
        prefix: None,
        suffix: Some("? ".to_string()),
        grammar_type: GrammarType::FileName,
        can_shorten: true,
        display_at_all: true,
    };
    grammars.push(file_nm_grammar);

    let choices = ["d", "N", "v", "a", "?"];
    for i in 0..choices.len() {
        let mut choice_separator = Some("/".to_string());
        if i == choices.len() - 1 {
            choice_separator = Some(": ".to_string());
        }
        let choice_grammar = GrammarFragment {
            original_val_for_prompt: Some(choices[i].to_string()),
            shortened_val_for_prompt: None,
            pos: (i + 2) as u8,
            prefix: None,
            suffix: choice_separator,
            grammar_type: GrammarType::UserChoice,
            can_shorten: false,
            display_at_all: true,
        };
        grammars.push(choice_grammar);
    }

    // let mut validate_all = false;
    loop {
        if output_control.validate_all {
            let validated_entry = validate_entry(entry, client, args)?;
            output_control.json_output = validated_entry;
            break;
        } else {
            let result = read_val::read_val_from_cmd_line_and_proceed_default(&mut grammars);

            match result.user_entered_val {
                None => {
                    break;
                }
                Some(user_entered_val) => match user_entered_val.as_str() {
                    "d" => {
                        if let Err(e) = details_about_entry(&entry) {
                            eprintln!("Error displaying entry details: {}", e);
                        }
                    }
                    "v" => {
                        let validated_entry = validate_entry(entry, client, args)?;
                        output_control.json_output = validated_entry;
                        break;
                    }
                    "?" => {
                        println!("N = Do nothing, skip this secret.");
                        println!(
                            "d = Display info (file name, Azure KV name, upstream secret create date, and file name modify date)."
                        );
                        println!("v = Validate on-disk item matches the Azure Key Vault secret.");
                        println!("a = Validate all items.");
                        println!("? = Display this help.");
                    }
                    "a" => {
                        output_control.validate_all = true;
                    }
                    "N" => {
                        break;
                    }
                    _ => {
                        eprintln!("Invalid choice: {}", user_entered_val);
                    }
                },
            }
        }
    }
    Ok(output_control)
}

fn details_about_entry(entry: &Value) -> Result<()> {
    let file_nm = crate::utils::json_utils::extract_string_field(entry, "file_nm")?;
    let az_name = crate::utils::json_utils::extract_string_field(entry, "az_name")?;
    let az_create = crate::utils::json_utils::extract_string_field(entry, "az_create")?;
    let az_updated = crate::utils::json_utils::extract_string_field(entry, "az_updated")?;

    println!("File: {}", file_nm);
    println!("Azure Key Vault Name: {}", az_name);

    let datetime_entries = vec![
        vec![az_create, "az create dt".to_string()],
        vec![az_updated, "az update dt".to_string()],
    ];

    for entry in datetime_entries {
        match entry[0].parse::<u64>() {
            Ok(az_create) => {
                println!(
                    "{}: {:?}",
                    entry[1],
                    OffsetDateTime::from_unix_timestamp(az_create as i64)
                );
            }
            Err(e) => {
                eprintln!("{} parse error: {}", entry[1], entry[0]);
                return Err(Box::<dyn Error>::from(format!(
                    "Failed to parse timestamp: {}",
                    e
                )));
            }
        }
    }
    Ok(())
}

/// Validates an entry by checking MD5 checksums and Azure IDs
///
/// # Errors
///
/// Returns an error if:
/// - Required fields are missing from the input JSON
/// - Unable to create a runtime
/// - Unable to retrieve the secret from Azure
/// - Unable to get system time
/// - Unable to get hostname
fn validate_entry(
    entry: Value,
    client: &SecretClient,
    args: &Args,
) -> std::result::Result<JsonOutput, Box<dyn Error>> {
    // Create a default output structure
    let mut output = JsonOutput {
        file_nm: String::new(),
        md5: String::new(),
        ins_ts: String::new(),
        az_id: String::new(),
        az_create: String::new(),
        az_updated: String::new(),
        az_name: String::new(),
        hostname: String::new(),
    };
    
    // Extract required fields from JSON
    let az_id = entry["az_id"]
        .as_str()
        .ok_or_else(|| Box::<dyn Error>::from("az_id missing in input json"))?;
        
    let file_nm = entry["file_nm"]
        .as_str()
        .ok_or_else(|| Box::<dyn Error>::from("file_nm missing in input json"))?;
        
    let az_name = entry["az_name"]
        .as_str()
        .ok_or_else(|| Box::<dyn Error>::from("az_name missing in input json"))?;

    // Create runtime and get secret from Azure
    let rt = Runtime::new()
        .map_err(|e| Box::<dyn Error>::from(format!("Failed to create runtime: {}", e)))?;
        
    let secret_value = rt.block_on(get_secret_value(az_name, client))?;

    // Calculate MD5 checksums
    let azure_md5 = calculate_md5(&secret_value.value);
    
    // Read local file and calculate MD5
    let md5_of_file = match fs::read_to_string(file_nm) {
        Ok(content) => calculate_md5(&content),
        Err(e) => {
            eprintln!("Error reading file to calculate md5: {} - {}", file_nm, e);
            // Return early with empty output if file can't be read
            return Ok(output);
        }
    };
    
    // Compare checksums
    if azure_md5 != md5_of_file {
        eprintln!("MD5 mismatch for file: {}", file_nm);
    } else if args.verbose {
        println!("MD5 match for file: {}", file_nm);
    }
    
    // Compare Azure IDs
    if az_id != secret_value.id {
        eprintln!(
            "Azure ID mismatch: id from azure {}, id from file {}",
            secret_value.id, az_id
        );
    } else if args.verbose {
        println!("Azure ID match for file: {}", file_nm);
    }

    // Get current timestamp
    let start = SystemTime::now();
    let duration = start.duration_since(UNIX_EPOCH)
        .map_err(|e| Box::<dyn Error>::from(format!("Failed to get timestamp: {}", e)))?;
    
    let datetime_utc = Utc
        .timestamp_opt(duration.as_secs() as i64, duration.subsec_nanos())
        .single()
        .ok_or_else(|| Box::<dyn Error>::from("Failed to create DateTime from timestamp"))?;
        
    let datetime_local: DateTime<Local> = datetime_utc.with_timezone(&Local);
    let formatted_date = datetime_local.to_rfc3339();

    // Get hostname
    let hostname = hostname::get()
        .map_err(|e| Box::<dyn Error>::from(format!("Failed to get hostname: {}", e)))?
        .into_string()
        .map_err(|_| Box::<dyn Error>::from("Hostname contains non-UTF8 characters"))?;

    // Create output structure
    output = JsonOutput {
        file_nm: file_nm.to_string(),
        md5: azure_md5,
        ins_ts: formatted_date,
        az_id: secret_value.id.to_string(),
        az_create: secret_value.created.to_string(),
        az_updated: secret_value.updated.to_string(),
        az_name: secret_value.name.to_string(),
        hostname,
    };

    Ok(output)
}

/// Writes JSON output to a file
///
/// # Errors
///
/// Returns an error if:
/// - Unable to open the output file
/// - JSON serialization fails
/// - Unable to write to the file
fn write_json_output(input: &[JsonOutput], output_file: &str) -> std::result::Result<(), Box<dyn Error>> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(output_file)
        .map_err(|e| Box::<dyn Error>::from(format!("Failed to open output file '{}': {}", output_file, e)))?;
    
    let json = serde_json::to_string_pretty(input)
        .map_err(|e| Box::<dyn Error>::from(format!("Failed to serialize JSON: {}", e)))?;
    
    file.write_all(json.as_bytes())
        .map_err(|e| Box::<dyn Error>::from(format!("Failed to write JSON to file: {}", e)))?;
    
    Ok(())
}

/// Retrieves a secret from Azure KeyVault
///
/// # Errors
///
/// Returns an error if the Azure API call fails
async fn get_secret_value(
    secret_name: &str,
    kv_client: &SecretClient,
) -> std::result::Result<SetSecretResponse, Box<dyn Error>> {
    let secret = kv_client.get(secret_name).await
        .map_err(|e| Box::<dyn Error>::from(format!("Failed to get secret '{}': {}", secret_name, e)))?;

    let created = secret.attributes.created_on;
    let updated = secret.attributes.updated_on;
    let id = secret.id;

    Ok(SetSecretResponse {
        created,
        updated,
        name: secret_name.to_string(),
        id: id.to_string(),
        value: secret.value,
    })
}

/// Sets a secret value in Azure KeyVault
///
/// # Errors
///
/// Returns an error if:
/// - The secret set operation fails
/// - Retrieving the secret after setting fails
async fn set_secret_value(
    secret_name: &str,
    kv_client: &SecretClient,
    secret_value: &str,
) -> std::result::Result<SetSecretResponse, Box<dyn Error>> {
    // Set the secret in KeyVault
    kv_client.set(secret_name, secret_value).await
        .map_err(|e| Box::<dyn Error>::from(format!("Failed to set secret '{}': {}", secret_name, e)))?;
    
    // Get the secret we just set to return its details
    get_secret_value(secret_name, kv_client).await
}

fn calculate_md5(content: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}

/// Get a KeyVault secret client for Azure operations
///
/// # Errors
///
/// Returns an error if:
/// - Unable to read the client secret file
/// - Unable to parse the authority URL
/// - Unable to create the KeyVault client
fn get_keyvault_secret_client(
    client_id: &str,
    client_secret: &PathBuf,
    tenant_id: &str,
    key_vault_name: &str,
) -> std::result::Result<SecretClient, Box<dyn Error>> {
    // Read client secret from file
    let mut secret = String::new();
    let mut file = File::open(client_secret)
        .map_err(|e| Box::<dyn Error>::from(format!("Failed to open client secret file: {}", e)))?;
    
    file.read_to_string(&mut secret)
        .map_err(|e| Box::<dyn Error>::from(format!("Failed to read client secret: {}", e)))?;
    
    // Remove newlines from secret
    secret = secret.trim().to_string();

    // Create HTTP client and Azure authority URL
    let http_client = Arc::new(Client::new());
    let authority_host = Url::parse("https://login.microsoftonline.com/")
        .map_err(|e| Box::<dyn Error>::from(format!("Failed to parse authority URL: {}", e)))?;
    
    // Create credential for Azure
    let credential = Arc::new(ClientSecretCredential::new(
        http_client,
        authority_host,
        tenant_id.to_string(),
        client_id.to_string(),
        secret,
    ));
    
    // Create KeyVault client
    let keyvault_client = KeyvaultClient::new(key_vault_name, credential)
        .map_err(|e| Box::<dyn Error>::from(format!("Failed to create KeyVault client: {}", e)))?;
    
    Ok(keyvault_client.secret_client())
}

/// Read content from a file 
///
/// # Errors
///
/// Returns an error if the file cannot be read
fn get_content_from_file(file_path: &str) -> std::result::Result<String, Box<dyn Error>> {
    fs::read_to_string(file_path)
        .map_err(|e| Box::<dyn Error>::from(format!("Failed to read file '{}': {}", file_path, e)))
}
