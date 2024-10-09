use crate::args::Args;

use md5::{Digest, Md5};
use regex::Regex;
use reqwest::Client;
use serde_json::json;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;
use tokio::runtime::Runtime;

pub async fn update_mode(args: &Args) -> Result<(), Box<dyn Error>> {
    let mut output_entries = vec![];

    // Regex to replace non-alphanumeric characters
    let re = Regex::new(r"[^a-zA-Z0-9-]")?;

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
            let content = fs::read_to_string(&full_path)?;

            // Calculate MD5 checksum
            let mut hasher = Md5::new();
            hasher.update(&content);
            let md5_checksum = format!("{:x}", hasher.finalize());

            // Insert secret into Azure Key Vault
            let azure_response = create_or_update_secret(
                args.secrets_client_id.as_ref().unwrap().as_str(),
                args.secrets_client_secret_path.as_ref().unwrap(),
                &secret_name,
                &content,
            )
            .await?;

            // Get current timestamp
            let start = SystemTime::now();
            let ins_ts = start.duration_since(UNIX_EPOCH)?.as_secs();

            // Build output entry
            let output_entry = json!({
                "filenm": full_path,
                "md5": md5_checksum,
                "ins_ts": ins_ts,
                "az_id": azure_response["id"],
                "az_create": azure_response["attributes"]["created"],
                "az_updated": azure_response["attributes"]["updated"],
            });

            output_entries.push(output_entry);
        }
    }

    // Append entries to output_file.txt
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(args.secrets_output_json.as_ref().unwrap().clone())?;

    for entry in output_entries {
        serde_json::to_writer(&mut file, &entry)?;
        writeln!(file)?; // Write a newline after each JSON object
    }

    Ok(())
}

pub  fn retrieve_mode(args: &Args) -> Result<(), Box<dyn Error>> {
    // Read and validate JSON entries
    let file = File::open(args.secrets_output_json.as_ref().unwrap().clone())?;
    let reader = BufReader::new(file);

    let rt = Runtime::new()?;
    let token = rt.block_on(get_azure_access_token(
        args.secrets_client_id.as_ref().unwrap().as_str(),
        args.secrets_client_secret_path.as_ref().unwrap(),
    ))?;

    for line in reader.lines() {
        let line = line?;
        let entry: serde_json::Value = serde_json::from_str(&line)?;

        let az_id = entry["az_id"].as_str().ok_or("az_id missing in entry")?;
        let filenm = entry["filenm"].as_str().ok_or("filenm missing in entry")?;

        // Retrieve the secret value
        let secret_value = get_secret_by_id(&token, az_id);

        println!("az_id: {}", az_id);
        println!("filenm: {}", filenm);
        println!("secret_value: {:?}", secret_value);
        println!("-----------------------------------");
    }

    Ok(())
}



async fn get_azure_access_token(
    client_id: &str,
    client_secret: &PathBuf,
) -> Result<String, Box<dyn Error>> {
    let client = Client::new();

    let mut secret = String::new();
    let mut file = File::open(client_secret)?;
    file.read_to_string(&mut secret)?;

    let params = [
        ("grant_type", "client_credentials"),
        ("client_id", client_id),
        ("client_secret", &secret),
        ("resource", "https://vault.azure.net"),
    ];

    let res = client
        .post("https://login.microsoftonline.com/<your-tenant-id>/oauth2/token")
        .form(&params)
        .send()
        ;

    let json_response = res.await?.json::<serde_json::Value>().await?;

    Ok(json_response["access_token"]
        .as_str()
        .ok_or("Failed to get access_token")?
        .to_string())
}

async fn create_or_update_secret(
    client_id: &str,
    client_secret: &PathBuf,
    secret_name: &str,
    secret_value: &str,
) -> Result<serde_json::Value, Box<dyn Error>> {
    let token = get_azure_access_token(client_id, client_secret)?;

    let client = Client::new();

    let url = format!(
        "https://<your-key-vault-name>.vault.azure.net/secrets/{}?api-version=7.2",
        secret_name
    );

    let res = client
        .put(&url)
        .bearer_auth(token)
        .json(&json!({ "value": secret_value }))
        .send()
        .await?;

    let json_response = res.json::<serde_json::Value>().await?;

    Ok(json_response)
}

 async fn get_secret_by_id(token: &str, az_id: &str) -> Result<String, Box<dyn Error>> {
    let client = Client::new();

    let res = client.get(az_id).bearer_auth(token).send();

    let json_response = res.await?.json::<serde_json::Value>().await?;

    let secret_value = json_response["value"]
        .as_str()
        .ok_or("Failed to retrieve secret value")?
        .to_string();

    Ok(secret_value)
}
