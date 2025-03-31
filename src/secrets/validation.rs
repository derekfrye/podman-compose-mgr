use crate::args::Args;
use crate::read_val::{self, GrammarFragment, GrammarType};
use crate::secrets::azure::{calculate_md5, get_content_from_file, get_keyvault_secret_client, get_secret_value};
use crate::secrets::error::Result;
use crate::secrets::models::{JsonOutput, JsonOutputControl};

use azure_security_keyvault::SecretClient;
use chrono::{DateTime, Local, TimeZone, Utc};
use serde_json::Value;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;
use time::OffsetDateTime;

/// Validates secrets stored in Azure KeyVault against local files
///
/// # Errors
///
/// Returns an error if:
/// - Unable to read the input JSON file
/// - JSON parsing fails
/// - Required arguments are missing
/// - KeyVault client creation fails
pub fn validate(args: &Args) -> Result<()> {
    // Get client for Azure KeyVault
    let (client, json_values) = prepare_validation(args)?;
    
    // Process each entry
    let json_outputs = process_validation_entries(&client, &json_values, args)?;

    // Write output if we have results
    if !json_outputs.is_empty() {
        write_validation_results(args, &json_outputs)?;
    }

    Ok(())
}

/// Prepare for validation by reading the input file and creating a KeyVault client
fn prepare_validation(args: &Args) -> Result<(SecretClient, Vec<Value>)> {
    // Get input file path
    let input_path = args.secret_mode_input_json.as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Input JSON path is required"))?;
    
    // Read and validate JSON entries
    let mut file = File::open(input_path)
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to open input JSON file: {}", e)))?;
    
    let mut file_content = String::new();
    file.read_to_string(&mut file_content)
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to read input JSON file: {}", e)))?;
    
    let json_values: Vec<Value> = serde_json::from_str(&file_content)
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to parse JSON: {}", e)))?;

    // Get Azure credentials
    let client_id = get_client_id(args)?;
    let client_secret = args.secrets_client_secret_path.as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Client secret path is required"))?;
    let tenant_id = get_tenant_id(args)?;
    let key_vault_name = get_key_vault_name(args)?;

    // Get KeyVault client
    let client = get_keyvault_secret_client(&client_id, client_secret, &tenant_id, &key_vault_name)?;
    
    Ok((client, json_values))
}

/// Get client ID from args or file
fn get_client_id(args: &Args) -> Result<String> {
    let client_id_arg = args.secrets_client_id.as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Client ID is required"))?;
    
    if client_id_arg.contains(std::path::MAIN_SEPARATOR) {
        get_content_from_file(client_id_arg)
    } else {
        Ok(client_id_arg.clone())
    }
}

/// Get tenant ID from args or file
fn get_tenant_id(args: &Args) -> Result<String> {
    let tenant_id_arg = args.secrets_tenant_id.as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Tenant ID is required"))?;
    
    if tenant_id_arg.contains(std::path::MAIN_SEPARATOR) {
        get_content_from_file(tenant_id_arg)
    } else {
        Ok(tenant_id_arg.clone())
    }
}

/// Get key vault name from args or file
fn get_key_vault_name(args: &Args) -> Result<String> {
    let key_vault_name_arg = args.secrets_vault_name.as_ref()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Key vault name is required"))?;
    
    if key_vault_name_arg.contains(std::path::MAIN_SEPARATOR) {
        get_content_from_file(key_vault_name_arg)
    } else {
        Ok(key_vault_name_arg.clone())
    }
}

/// Process each validation entry, either directly or interactively
fn process_validation_entries(
    client: &SecretClient,
    json_values: &Vec<Value>,
    args: &Args
) -> Result<Vec<JsonOutput>> {
    let mut json_outputs: Vec<JsonOutput> = vec![];
    
    // Process each entry
    let mut loop_result: JsonOutputControl = JsonOutputControl::new();
    for entry in json_values {
        if loop_result.validate_all {
            match validate_entry(entry.clone(), client, args) {
                Ok(z) => json_outputs.push(z),
                Err(e) => eprintln!("Error validating entry: {}", e),
            }
            continue;
        } else {
            match read_val_loop(entry.clone(), client, args) {
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
    
    Ok(json_outputs)
}

/// Write validation results to the output file
fn write_validation_results(args: &Args, json_outputs: &[JsonOutput]) -> Result<()> {
    if let Some(output_path) = args.secret_mode_output_json.as_ref() {
        if let Some(output_dir) = output_path.parent() {
            fs::create_dir_all(output_dir)
                .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to create output directory: {}", e)))?;
        }
        
        let output_str = output_path.to_str()
            .ok_or_else(|| Box::<dyn std::error::Error>::from("Invalid UTF-8 in output path"))?;
            
        write_json_output(json_outputs, output_str)?;
    } else {
        return Err(Box::<dyn std::error::Error>::from("Output JSON path is required"));
    }
    
    Ok(())
}

/// Interactive validation loop for a single entry
pub fn read_val_loop(
    entry: Value,
    client: &SecretClient,
    args: &Args,
) -> Result<JsonOutputControl> {
    let mut grammars: Vec<GrammarFragment> = vec![];
    let mut output_control: JsonOutputControl = JsonOutputControl::new();
    
    setup_validation_prompt(&mut grammars, &entry)?;
    
    loop {
        if output_control.validate_all {
            let validated_entry = validate_entry(entry.clone(), client, args)?;
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
                        let validated_entry = validate_entry(entry.clone(), client, args)?;
                        output_control.json_output = validated_entry;
                        break;
                    }
                    "?" => {
                        display_validation_help();
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

/// Setup the interactive prompt for validation
fn setup_validation_prompt(grammars: &mut Vec<GrammarFragment>, entry: &Value) -> Result<()> {
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

    let file_name = crate::utils::json_utils::extract_string_field(entry, "file_nm")?;

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

    add_choice_options(grammars);
    
    Ok(())
}

/// Add user choice options to the prompt
fn add_choice_options(grammars: &mut Vec<GrammarFragment>) {
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
}

/// Display help for validation options
fn display_validation_help() {
    println!("N = Do nothing, skip this secret.");
    println!(
        "d = Display info (file name, Azure KV name, upstream secret create date, and file name modify date)."
    );
    println!("v = Validate on-disk item matches the Azure Key Vault secret.");
    println!("a = Validate all items.");
    println!("? = Display this help.");
}

/// Display details about a validation entry
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
                return Err(Box::<dyn std::error::Error>::from(format!(
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
pub fn validate_entry(
    entry: Value,
    client: &SecretClient,
    args: &Args,
) -> Result<JsonOutput> {
    // Extract required fields
    let (az_id, file_nm, az_name) = extract_validation_fields(&entry)?;
    
    // Get the secret from Azure KeyVault
    let secret_value = get_secret_from_azure(az_name, client)?;
    
    // Validate checksums and IDs
    validate_checksums(&file_nm, &secret_value.value, args)?;
    validate_azure_ids(&az_id, &secret_value.id, args)?;
    
    // Create timestamp and get hostname
    let formatted_date = get_current_timestamp()?;
    let hostname = get_hostname()?;
    
    // Create and return output structure
    Ok(JsonOutput {
        file_nm,
        md5: calculate_md5(&secret_value.value),
        ins_ts: formatted_date,
        az_id: secret_value.id.to_string(),
        az_create: secret_value.created.to_string(),
        az_updated: secret_value.updated.to_string(),
        az_name: secret_value.name.to_string(),
        hostname,
    })
}

/// Extract required fields for validation
fn extract_validation_fields(entry: &Value) -> Result<(String, String, String)> {
    let az_id = entry["az_id"]
        .as_str()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("az_id missing in input json"))?
        .to_string();
        
    let file_nm = entry["file_nm"]
        .as_str()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("file_nm missing in input json"))?
        .to_string();
        
    let az_name = entry["az_name"]
        .as_str()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("az_name missing in input json"))?
        .to_string();
        
    Ok((az_id, file_nm, az_name))
}

/// Get a secret from Azure KeyVault
fn get_secret_from_azure(az_name: String, client: &SecretClient) -> Result<crate::secrets::models::SetSecretResponse> {
    let rt = Runtime::new()
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to create runtime: {}", e)))?;
        
    rt.block_on(get_secret_value(&az_name, client))
}

/// Validate MD5 checksums match
fn validate_checksums(file_nm: &str, secret_value: &str, args: &Args) -> Result<()> {
    let azure_md5 = calculate_md5(secret_value);
    
    // Read local file and calculate MD5
    match fs::read_to_string(file_nm) {
        Ok(content) => {
            let md5_of_file = calculate_md5(&content);
            
            if azure_md5 != md5_of_file {
                eprintln!("MD5 mismatch for file: {}", file_nm);
            } else if args.verbose {
                println!("MD5 match for file: {}", file_nm);
            }
            Ok(())
        },
        Err(e) => {
            eprintln!("Error reading file to calculate md5: {} - {}", file_nm, e);
            Ok(())
        }
    }
}

/// Validate Azure IDs match
fn validate_azure_ids(az_id: &str, secret_id: &str, args: &Args) -> Result<()> {
    if az_id != secret_id {
        eprintln!(
            "Azure ID mismatch: id from azure {}, id from file {}",
            secret_id, az_id
        );
    } else if args.verbose {
        println!("Azure ID match");
    }
    Ok(())
}

/// Get current timestamp formatted as RFC3339
fn get_current_timestamp() -> Result<String> {
    let start = SystemTime::now();
    let duration = start.duration_since(UNIX_EPOCH)
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to get timestamp: {}", e)))?;
    
    let datetime_utc = Utc
        .timestamp_opt(duration.as_secs() as i64, duration.subsec_nanos())
        .single()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Failed to create DateTime from timestamp"))?;
        
    let datetime_local: DateTime<Local> = datetime_utc.with_timezone(&Local);
    Ok(datetime_local.to_rfc3339())
}

/// Get hostname of the current machine
fn get_hostname() -> Result<String> {
    hostname::get()
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to get hostname: {}", e)))?
        .into_string()
        .map_err(|_| Box::<dyn std::error::Error>::from("Hostname contains non-UTF8 characters"))
}

/// Writes JSON output to a file
///
/// # Errors
///
/// Returns an error if:
/// - Unable to open the output file
/// - JSON serialization fails
/// - Unable to write to the file
pub fn write_json_output(input: &[JsonOutput], output_file: &str) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(output_file)
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to open output file '{}': {}", output_file, e)))?;
    
    let json = serde_json::to_string_pretty(input)
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to serialize JSON: {}", e)))?;
    
    file.write_all(json.as_bytes())
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to write JSON to file: {}", e)))?;
    
    Ok(())
}