use crate::args::Args;
use crate::read_val::{self, Grammar, GrammerType};

use chrono::{DateTime, Local, TimeZone, Utc};
use md5::{Digest, Md5};
use regex::Regex;
use reqwest::{Client, Url};
use serde::Serialize;
// use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;
use std::fs;
use std::fs::File;
use std::path::PathBuf;
// use std::io::{BufRead, BufReader};
use std::io::{Read, Write};
// use std::path::PathBuf;
use azure_identity::ClientSecretCredential;
use azure_security_keyvault::{KeyvaultClient, SecretClient};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;
use walkdir::WalkDir;
// use chrono::{DateTime, FixedOffset};
use time::OffsetDateTime;
// use url::Url;

struct SetSecretResponse {
    created: OffsetDateTime,
    updated: OffsetDateTime,
    name: String,
    id: String,
    value: String,
}

#[derive(Serialize)]
struct JsonOutput {
    filenm: String,
    md5: String,
    ins_ts: String,
    az_id: String,
    az_create: String,
    az_updated: String,
    az_name: String,
}

pub fn update_mode(args: &Args) -> Result<(), Box<dyn Error>> {
    let mut output_entries = vec![];

    // Regex to replace non-alphanumeric characters
    let re = Regex::new(r"[^a-zA-Z0-9-]").unwrap();

    let client_id = args.secrets_client_id.as_ref().unwrap();
    let client_secret = args.secrets_client_secret_path.as_ref().unwrap();
    let tenant_id = args.secrets_tenant_id.as_ref().unwrap();
    let kev_vault_name = args.secrets_vault_name.as_ref().unwrap();

    let client = get_keyvault_secret_client(client_id, client_secret, tenant_id, kev_vault_name);

    let rt = Runtime::new().unwrap();

    for entry in WalkDir::new(args.path.clone())
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.file_name() == ".env" && entry.file_type().is_file() {
            let full_path = entry.path().to_string_lossy().to_string();
            // strip out the platform-dependent path separator
            let stripped_path = full_path.trim_start_matches(std::path::MAIN_SEPARATOR);

            // Translate non-alphanumeric characters to '-'
            let secret_name = re.replace_all(stripped_path, "-");

            // Read file content
            let content = fs::read_to_string(&full_path).unwrap();
            let md5_checksum = calculate_md5(content.as_str());

            // Insert secret into Azure Key Vault
            let azure_response = rt
                .block_on(set_secret_value(&secret_name, &client, &content))
                .unwrap();

            // Get current timestamp
            let start = SystemTime::now();
            let ins_ts = start.duration_since(UNIX_EPOCH).unwrap().as_secs();

            // Build output entry
            let output_entry = json!({
                "filenm": full_path,
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

    // Append entries to output_file.txt
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(args.secrets_output_json.as_ref().unwrap().clone())
        .unwrap();

    for entry in output_entries {
        serde_json::to_writer(&mut file, &entry).unwrap();
        writeln!(file).unwrap(); // Write a newline after each JSON object
    }

    Ok(())
}

pub fn validate(args: &Args) -> Result<(), Box<dyn Error>> {
    // Read and validate JSON entries
    let mut file = File::open(args.secrets_input_json.as_ref().unwrap().clone()).unwrap();
    let mut file_content = String::new();
    file.read_to_string(&mut file_content).unwrap();
    let json_values: Vec<Value> = serde_json::from_str(&file_content).unwrap(); // Deserialize the entire JSON array

    let client_id = args.secrets_client_id.as_ref().unwrap();
    let client_secret = args.secrets_client_secret_path.as_ref().unwrap();
    let tenant_id = args.secrets_tenant_id.as_ref().unwrap();
    let kev_vault_name = args.secrets_vault_name.as_ref().unwrap();

    let client = get_keyvault_secret_client(client_id, client_secret, tenant_id, kev_vault_name);

    for entry in json_values {
        // let string_representation = serde_json::to_string(&entry).unwrap();
        // dbg!(&string_representation);

        let t = read_val_loop(entry, &client, args);
        match t {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }

    Ok(())
}

fn read_val_loop(entry: Value, client: &SecretClient, args: &Args) -> Result<(), Box<dyn Error>> {
    let mut grammars: Vec<Grammar> = vec![];
    let mut json_outputs: Vec<JsonOutput> = vec![];
    let static_prompt_grammar = Grammar {
        original_val_for_prompt: Some("Check".to_string()),
        shortend_val_for_prompt: None,
        pos: 0,
        prefix: None,
        suffix: Some(" ".to_string()),
        grammer_type: GrammerType::Verbiage,
        part_of_static_prompt: true,
        display_at_all: true,
    };
    grammars.push(static_prompt_grammar);

    let file_name = entry["filenm"]
        .as_str()
        .ok_or("filenm missing in input json")
        .unwrap();

    let filenm_grammar = Grammar {
        original_val_for_prompt: Some(file_name.to_string()),
        shortend_val_for_prompt: None,
        pos: 1,
        prefix: None,
        suffix: Some("? ".to_string()),
        grammer_type: GrammerType::FileName,
        part_of_static_prompt: false,
        display_at_all: true,
    };
    grammars.push(filenm_grammar);

    let choices = vec!["d", "N", "v", "?"];
    for i in 0..choices.len() {
        let mut choice_separator = Some("/".to_string());
        if i == choices.len() - 1 {
            choice_separator = Some(": ".to_string());
        }
        let choice_grammar = Grammar {
            original_val_for_prompt: Some(choices[i].to_string()),
            shortend_val_for_prompt: None,
            pos: (i + 2) as u8,
            prefix: None,
            suffix: choice_separator,
            grammer_type: GrammerType::UserChoice,
            part_of_static_prompt: true,
            display_at_all: true,
        };
        grammars.push(choice_grammar);
    }

    loop {
        let result = read_val::read_val_from_cmd_line_and_proceed(
            &mut grammars,
            GrammerType::FileName,
            GrammerType::None,
        );

        match result.user_entered_val {
            None => {
                break;
            }
            Some(user_entered_val) => match user_entered_val.as_str() {
                "d" => {
                    details_about_entry(&entry);
                }
                "v" => {
                    let z = validate_entry(entry, client, args).unwrap();
                    json_outputs.push(z);
                    break;
                }
                "?" => {
                    println!("N = Do nothing, skip this secret.");
                    println!(
                            "d = Display info (file name, Azure KV name, upstream secret create date, and file name modify date)."
                        );
                    println!("v = Validate on-disk item matches the Azure Key Vault secret.");
                    println!("? = Display this help.");
                }
                "N" | _ => {
                    break;
                }
            },
        }
    }
    if json_outputs.len() > 0 {
        write_json_output(
            &json_outputs,
            args.secrets_output_json.as_ref().unwrap().to_str().unwrap(),
        );
    }

    Ok(())
}

fn details_about_entry(entry: &Value) {
    let filenm = entry["filenm"]
        .as_str()
        .ok_or("filenm missing in input json")
        .unwrap();
    let az_name = entry["az_name"]
        .as_str()
        .ok_or("az_name missing in input json")
        .unwrap();
    let az_create = entry["az_create"]
        .as_str()
        .ok_or("az_create missing in input json")
        .unwrap();
    let az_updated = entry["az_updated"]
        .as_str()
        .ok_or("az_updated missing in input json")
        .unwrap();

    println!("File: {}", filenm);
    println!("Azure Key Vault Name: {}", az_name);

    let x = vec![
        vec![az_create, "az create dt"],
        vec![az_updated, "az update dt"],
    ];

    for y in x {
        match y[0].parse::<u64>() {
            Ok(az_create) => {
                println!(
                    "{}: {:?}",
                    y[1],
                    OffsetDateTime::from_unix_timestamp(az_create as i64)
                );
            }
            Err(_) => {
                eprintln!("{} parse error: {}", y[1], y[0]);
            }
        }
    }
}

fn validate_entry(
    entry: Value,
    client: &SecretClient,
    args: &Args,
) -> Result<JsonOutput, Box<dyn Error>> {
    let mut output = JsonOutput {
        filenm: String::new(),
        md5: String::new(),
        ins_ts: String::new(),
        az_id: String::new(),
        az_create: String::new(),
        az_updated: String::new(),
        az_name: String::new(),
    };
    let mut az_id = entry["az_id"]
        .as_str()
        .ok_or("az_id missing in input json")
        .unwrap();
    let filenm = entry["filenm"]
        .as_str()
        .ok_or("filenm missing in input json")
        .unwrap();
    let mut az_name = entry["az_name"]
        .as_str()
        .ok_or("az_name missing in input json")
        .unwrap();
    let az_create; // = entry["az_create"].as_str().ok_or("az_create missing in input json").unwrap();
    let az_updated; // = entry["az_updated"].as_str().ok_or("az_updated missing in input json").unwrap();

    let rt = Runtime::new().unwrap();
    let secret_value = rt.block_on(get_secret_value(az_name, &client)).unwrap();

    az_name = &secret_value.name;
    az_create = secret_value.created.to_string();
    az_updated = secret_value.updated.to_string();

    let md5 = calculate_md5(&secret_value.value);
    let md5_of_file = match fs::read_to_string(filenm) {
        Ok(content) => calculate_md5(&content),
        Err(_) => {
            eprintln!("Error reading file to calculate md5: {}", filenm);
            return Ok(output);
        }
    };
    if md5 != md5_of_file {
        eprintln!("MD5 mismatch for file: {}", filenm);
    } else if args.verbose {
        println!("MD5 match for file: {}", filenm);
    }
    if az_id != secret_value.id {
        eprintln!(
            "Azure ID mismatch: id from azure {}, id from file {}",
            secret_value.id, az_id
        );
    } else if args.verbose {
        println!("Azure ID match for file: {}", filenm);
    }

    az_id = &secret_value.id;
    let start = SystemTime::now();
    let duration = start.duration_since(UNIX_EPOCH).unwrap();
    let datetime_utc = Utc
        .timestamp_opt(duration.as_secs() as i64, duration.subsec_nanos())
        .single()
        .unwrap();
    let datetime_local: DateTime<Local> = datetime_utc.with_timezone(&Local);
    let formatted_date = datetime_local.to_rfc3339();
    output = JsonOutput {
        filenm: filenm.to_string(),
        md5: md5,
        ins_ts: formatted_date,
        az_id: az_id.to_string(),
        az_create: az_create,
        az_updated: az_updated,
        az_name: az_name.to_string(),
    };
    // write_json_output(
    //     args.secrets_output_json.as_ref().unwrap().clone(),
    //     az_id,
    //     az_name,
    //     &az_create,
    //     &az_updated,
    //     md5,
    //     filenm
    // );

    Ok(output)
}

fn write_json_output(input: &Vec<JsonOutput>, output_file: &str) {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_file)
        .unwrap();
    // let json = serde_json::to_string(&input).unwrap();
    let json = serde_json::to_string_pretty(&input).unwrap();
    file.write_all(json.as_bytes()).unwrap();
}

async fn get_secret_value(
    secret_name: &str,
    kv_client: &SecretClient,
) -> Result<SetSecretResponse, Box<dyn Error>> {
    let secret = kv_client.get(secret_name).await.unwrap();
    // dbg!(&secret);

    let created = secret.attributes.created_on;
    let updated = secret.attributes.updated_on;
    let id = secret.id;
    let name = secret_name;

    Ok(SetSecretResponse {
        created,
        updated,
        name: name.to_string(),
        id: id.to_string(),
        value: secret.value,
    })
}

async fn set_secret_value(
    secret_name: &str,
    kv_client: &SecretClient,
    secret_value: &str,
) -> Result<SetSecretResponse, Box<dyn Error>> {
    kv_client.set(secret_name, secret_value).await.unwrap();
    Ok(get_secret_value(secret_name, kv_client).await.unwrap())
}

fn calculate_md5(content: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}

fn get_keyvault_secret_client(
    client_id: &str,
    client_secret: &PathBuf,
    tenant_id: &str,
    kev_vault_name: &str,
) -> SecretClient {
    let mut secret = String::new();
    let mut file = File::open(client_secret).unwrap();
    file.read_to_string(&mut secret).unwrap();
    // remove newlines from secret
    secret = secret.trim().to_string();

    let http_client = Arc::new(Client::new());
    let authority_host = Url::parse("https://login.microsoftonline.com/").unwrap();
    let credential = Arc::new(ClientSecretCredential::new(
        http_client,
        authority_host,
        tenant_id.to_string(),
        client_id.to_string(),
        secret.to_string(),
    ));
    let client = KeyvaultClient::new(kev_vault_name, credential)
        .unwrap()
        .secret_client();
    client
}
