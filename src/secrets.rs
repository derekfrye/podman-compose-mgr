use crate::args::Args;
use crate::read_val::{self, Grammar, GrammerType};

use md5::{Digest, Md5};
use regex::Regex;
// use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;
use std::fs::File;
use std::{env, fs};
// use std::io::{BufRead, BufReader};
use std::io::{Read, Write};
// use std::path::PathBuf;
use azure_identity::DefaultAzureCredentialBuilder;
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

pub fn update_mode(args: &Args) -> Result<(), Box<dyn Error>> {
    let mut output_entries = vec![];

    // Regex to replace non-alphanumeric characters
    let re = Regex::new(r"[^a-zA-Z0-9-]").unwrap();

    let client_id = args.secrets_client_id.as_ref().unwrap();
    let client_secret = args.secrets_client_secret_path.as_ref().unwrap();
    let tenant_id = args.secrets_tenant_id.as_ref().unwrap();
    let kev_vault_name = args.secrets_vault_name.as_ref().unwrap();

    env::set_var("AZURE_CLIENT_ID", client_id);
    dbg!(&client_id);
    env::set_var("AZURE_TENANT_ID", tenant_id);
    env::set_var("AZURE_SUBSCRIPTION_ID", tenant_id);
    dbg!(&tenant_id);
    let mut secret = String::new();
    let mut file = File::open(client_secret).unwrap();
    file.read_to_string(&mut secret).unwrap();
    // remove newlines from secret
    secret = secret.trim().to_string();
    dbg!(&secret);
    env::set_var("AZURE_CLIENT_SECRET", secret);

    let credential = Arc::new(DefaultAzureCredentialBuilder::new().build().unwrap());
    // let credential = azure_identity::create_credential().unwrap();
    let client = KeyvaultClient::new(kev_vault_name, credential)
        .unwrap()
        .secret_client();
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
                .block_on(set_secret_value(
                    // args.secrets_client_id.as_ref().unwrap().as_str(),
                    // args.secrets_client_secret_path.as_ref().unwrap(),
                    // args.secrets_tenant_id.as_ref().unwrap().as_str(),
                    // args.secrets_vault_name.as_ref().unwrap().as_str(),
                    &secret_name,
                    &client,
                    &content,
                ))
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

pub fn retrieve_mode(args: &Args) -> Result<(), Box<dyn Error>> {
    // Read and validate JSON entries
    let mut file = File::open(args.secrets_input_json.as_ref().unwrap().clone()).unwrap();
    let mut file_content = String::new();
    file.read_to_string(&mut file_content).unwrap();
    let json_values: Vec<Value> = serde_json::from_str(&file_content).unwrap(); // Deserialize the entire JSON array

    let client_id = args.secrets_client_id.as_ref().unwrap();
    let client_secret = args.secrets_client_secret_path.as_ref().unwrap();
    let tenant_id = args.secrets_tenant_id.as_ref().unwrap();
    let kev_vault_name = args.secrets_vault_name.as_ref().unwrap();

    env::set_var("AZURE_CLIENT_ID", client_id);
    // dbg!(&client_id);
    env::set_var("AZURE_TENANT_ID", tenant_id);
    env::set_var("AZURE_SUBSCRIPTION_ID", tenant_id);
    // dbg!(&tenant_id);
    let mut secret = String::new();
    let mut file = File::open(client_secret).unwrap();
    file.read_to_string(&mut secret).unwrap();
    // remove newlines from secret
    secret = secret.trim().to_string();
    // dbg!(&secret);
    env::set_var("AZURE_CLIENT_SECRET", secret);

    let credential = Arc::new(DefaultAzureCredentialBuilder::new().build().unwrap());
    let client = KeyvaultClient::new(kev_vault_name, credential)
        .unwrap()
        .secret_client();

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
    let grm1 = Grammar {
        original_val_for_prompt: Some("Check".to_string()),
        shortend_val_for_prompt: None,
        pos: 0,
        prefix: None,
        suffix: Some(" ".to_string()),
        grammer_type: GrammerType::Verbiage,
        part_of_static_prompt: true,
        display_at_all: true,
    };
    grammars.push(grm1);

    let docker_compose_pth = entry["filenm"]
        .as_str()
        .ok_or("filenm missing in input json")
        .unwrap();

    let docker_compose_pth_fmtted = format!("{}", docker_compose_pth);
    let grm4 = Grammar {
        original_val_for_prompt: Some(docker_compose_pth_fmtted.clone()),
        shortend_val_for_prompt: None,
        pos: 1,
        prefix: None,
        suffix: Some("? ".to_string()),
        grammer_type: GrammerType::AzureSecretName,
        part_of_static_prompt: false,
        display_at_all: true,
    };
    grammars.push(grm4);

    let choices = vec!["N", "v", "?"];
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
            GrammerType::AzureSecretName,
            GrammerType::None,
        );

        match result.user_entered_val {
            None => {
                break;
            }
            Some(user_entered_val) => match user_entered_val.as_str() {
                "v" => {
                    let az_id = entry["az_id"]
                        .as_str()
                        .ok_or("az_id missing in input json")
                        .unwrap();
                    let filenm = entry["filenm"]
                        .as_str()
                        .ok_or("filenm missing in input json")
                        .unwrap();
                    let az_name = entry["az_name"]
                        .as_str()
                        .ok_or("az_name missing in input json")
                        .unwrap();

                    let rt = Runtime::new().unwrap();
                    let secret_value = rt.block_on(get_secret_value(az_name, &client)).unwrap();

                    // might be a good practice to zero these out?
                    // env::set_var("AZURE_CLIENT_ID", "");
                    // // dbg!(&client_id);
                    // env::set_var("AZURE_TENANT_ID", "");
                    // env::set_var("AZURE_SUBSCRIPTION_ID", "");
                    // env::set_var("AZURE_CLIENT_SECRET", "");

                    let md5 = calculate_md5(&secret_value.value);
                    let md5_of_file =
                        calculate_md5(&fs::read_to_string(filenm).unwrap_or_else(|err| {
                            panic!("md5 can't be calculated: {}, for file {}", err, filenm);
                        }));
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

    Ok(())
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
