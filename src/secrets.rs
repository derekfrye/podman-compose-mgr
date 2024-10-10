use crate::args::Args;

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

    for entry in json_values {
        // let string_representation = serde_json::to_string(&entry).unwrap();
        // dbg!(&string_representation);

        let az_id = entry["az_id"]
            .as_str()
            .ok_or("az_id missing in entry")
            .unwrap();
        let filenm = entry["filenm"]
            .as_str()
            .ok_or("filenm missing in entry")
            .unwrap();
        let az_name = entry["az_name"]
            .as_str()
            .ok_or("az_name missing in entry")
            .unwrap();

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

        let rt = Runtime::new().unwrap();
        let secret_value = rt.block_on(get_secret_value(az_name, &client)).unwrap();
        let md5 = calculate_md5(&secret_value.value);
        let md5_of_file = calculate_md5(&fs::read_to_string(filenm).unwrap_or_else(|err| {
            panic!("md5 prb: {}, file {}", err, filenm);
        }));
        if md5 != md5_of_file {
            eprintln!("MD5 mismatch for file: {}", filenm);
        } else {
            println!("MD5 match for file: {}", filenm);
        }
        if az_id != secret_value.id {
            eprintln!(
                "Azure ID mismatch: id from azure {}, id from file {}",
                az_id, secret_value.id
            );
        } else {
            println!("Azure ID match for file: {}", filenm);
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
