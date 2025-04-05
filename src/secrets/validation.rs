use crate::args::Args;
use crate::read_val::{self, GrammarFragment};
use crate::secrets::azure::{calculate_md5, get_content_from_file, get_keyvault_client, get_secret_value};
use crate::secrets::error::Result;
use crate::secrets::models::{JsonOutput, JsonOutputControl, SetSecretResponse};
use crate::secrets::prompt::{setup_validation_prompt, display_validation_help};
use crate::secrets::utils::{
    extract_validation_fields, details_about_entry, get_current_timestamp, 
    get_hostname, write_json_output
};

use azure_security_keyvault::KeyvaultClient;
use serde_json::Value;
use std::fs::{self, File};
use std::io::Read;
use tokio::runtime::Runtime;

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
pub fn prepare_validation(args: &Args) -> Result<(KeyvaultClient, Vec<Value>)> {
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
    let client = get_keyvault_client(&client_id, client_secret, &tenant_id, &key_vault_name)?;
    
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
    client: &KeyvaultClient,
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

/// Process user choice for validation
fn process_validation_choice(
    choice: &str,
    entry: &Value,
    client: &KeyvaultClient,
    args: &Args,
    output_control: &mut JsonOutputControl
) -> Result<bool> {
    match choice {
        // Display entry details
        "d" => {
            if let Err(e) = details_about_entry(entry) {
                eprintln!("Error displaying entry details: {}", e);
            }
            Ok(false) // Continue the loop
        }
        // Validate entry
        "v" => {
            let validated_entry = validate_entry(entry.clone(), client, args)?;
            output_control.json_output = validated_entry;
            Ok(true) // Exit the loop
        }
        // Display help
        "?" => {
            display_validation_help();
            Ok(false) // Continue the loop
        }
        // Validate all entries
        "a" => {
            output_control.validate_all = true;
            Ok(false) // Continue the loop but will exit in the next iteration
        }
        // Skip this entry
        "N" => {
            Ok(true) // Exit the loop
        }
        // Invalid choice
        _ => {
            eprintln!("Invalid choice: {}", choice);
            Ok(false) // Continue the loop
        }
    }
}

/// Interactive validation loop for a single entry
pub fn read_val_loop(
    entry: Value,
    client: &KeyvaultClient,
    args: &Args,
) -> Result<JsonOutputControl> {
    let mut grammars: Vec<GrammarFragment> = vec![];
    let mut output_control: JsonOutputControl = JsonOutputControl::new();
    
    setup_validation_prompt(&mut grammars, &entry)?;
    
    loop {
        // If validate_all flag is set, validate immediately
        if output_control.validate_all {
            let validated_entry = validate_entry(entry.clone(), client, args)?;
            output_control.json_output = validated_entry;
            break;
        }
        
        // Display prompt and get user input
        let result = read_val::read_val_from_cmd_line_and_proceed_default(&mut grammars);

        match result.user_entered_val {
            None => break, // Empty input
            Some(user_entered_val) => {
                // Process user choice and determine if we should exit the loop
                let should_exit = process_validation_choice(
                    &user_entered_val,
                    &entry,
                    client,
                    args,
                    &mut output_control
                )?;
                
                if should_exit {
                    break;
                }
            }
        }
    }
    
    Ok(output_control)
}

/// Get a secret from Azure KeyVault
fn get_secret_from_azure(az_name: String, client: &KeyvaultClient) -> Result<SetSecretResponse> {
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

/// Create output JSON structure from validation results
fn create_validation_output(
    file_nm: String,
    secret_value: &SetSecretResponse,
    formatted_date: String,
    hostname: String,
) -> JsonOutput {
    JsonOutput {
        file_nm,
        md5: calculate_md5(&secret_value.value),
        ins_ts: formatted_date,
        az_id: secret_value.id.to_string(),
        az_create: secret_value.created.to_string(),
        az_updated: secret_value.updated.to_string(),
        az_name: secret_value.name.to_string(),
        hostname,
    }
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
    client: &KeyvaultClient,
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
    Ok(create_validation_output(file_nm, &secret_value, formatted_date, hostname))
}