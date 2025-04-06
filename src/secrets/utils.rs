use crate::secrets::error::Result;
use crate::secrets::models::JsonOutput;
use chrono::{DateTime, Local, TimeZone, Utc};
use hex;
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::fs;
use std::io::{BufReader, Read, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use time::OffsetDateTime;

/// Extract required fields for validation
pub fn extract_validation_fields(entry: &Value) -> Result<(String, String, String, String, String)> {
    // Support both old and new field names
    let cloud_id = entry["cloud_id"]
        .as_str()
        .or_else(|| entry["az_id"].as_str())
        .ok_or_else(|| Box::<dyn std::error::Error>::from("cloud_id missing in input json"))?
        .to_string();

    let file_nm = entry["file_nm"]
        .as_str()
        .or_else(|| entry["filenm"].as_str())
        .ok_or_else(|| Box::<dyn std::error::Error>::from("file_nm missing in input json"))?
        .to_string();

    let secret_name = entry["secret_name"]
        .as_str()
        .or_else(|| entry["az_name"].as_str())
        .ok_or_else(|| Box::<dyn std::error::Error>::from("secret_name missing in input json"))?
        .to_string();

    // Get encoding, defaulting to "utf8" for backward compatibility
    let encoding = entry["encoding"].as_str().unwrap_or("utf8").to_string();
    
    // Get storage type (azure_kv or b2)
    let storage_type = entry["destination_cloud"]
        .as_str()
        .or_else(|| entry["cloud_type"].as_str())
        .unwrap_or("azure_kv")
        .to_string();

    Ok((cloud_id, file_nm, secret_name, encoding, storage_type))
}

/// Display details about a validation entry
pub fn details_about_entry(entry: &Value) -> Result<()> {
    // Support both old and new field names
    let file_nm = crate::utils::json_utils::extract_string_field_or(entry, "file_nm", "filenm")?;
    
    // Get secret name (from either az_name or secret_name)
    let secret_name = entry["secret_name"]
        .as_str()
        .or_else(|| entry["az_name"].as_str())
        .unwrap_or("unknown");
        
    // Get cloud timestamps
    let cloud_created = entry["cloud_cr_ts"]
        .as_str()
        .or_else(|| entry["az_create"].as_str())
        .unwrap_or("");
        
    let cloud_updated = entry["cloud_upd_ts"]
        .as_str()
        .or_else(|| entry["az_updated"].as_str())
        .unwrap_or("");
        
    // Get encoding with default value for backward compatibility
    let encoding = entry["encoding"].as_str().unwrap_or("utf8");
    
    // Get storage type
    let storage_type = entry["destination_cloud"]
        .as_str()
        .or_else(|| entry["cloud_type"].as_str())
        .unwrap_or("azure_kv");

    println!("File: {}", file_nm);
    println!("Secret Name: {}", secret_name);
    println!("Storage Type: {}", storage_type);
    println!("Encoding: {}", encoding);
    
    // Show hash information if available
    if let Some(hash) = entry["hash"].as_str() {
        let hash_algo = entry["hash_algo"].as_str().unwrap_or("sha1");
        println!("Hash ({}):{}", hash_algo, hash);
    }

    let datetime_entries = vec![
        vec![cloud_created, "Cloud created"],
        vec![cloud_updated, "Cloud updated"],
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

/// Get current timestamp formatted as RFC3339
pub fn get_current_timestamp() -> Result<String> {
    let start = SystemTime::now();
    let duration = start.duration_since(UNIX_EPOCH).map_err(|e| {
        Box::<dyn std::error::Error>::from(format!("Failed to get timestamp: {}", e))
    })?;

    let datetime_utc = Utc
        .timestamp_opt(duration.as_secs() as i64, duration.subsec_nanos())
        .single()
        .ok_or_else(|| {
            Box::<dyn std::error::Error>::from("Failed to create DateTime from timestamp")
        })?;

    let datetime_local: DateTime<Local> = datetime_utc.with_timezone(&Local);
    Ok(datetime_local.to_rfc3339())
}

/// Get hostname of the current machine
pub fn get_hostname() -> Result<String> {
    hostname::get()
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to get hostname: {}", e)))?
        .into_string()
        .map_err(|_| Box::<dyn std::error::Error>::from("Hostname contains non-UTF8 characters"))
}

/// Calculate SHA-1 hash for a file using streaming to handle large files
pub fn calculate_hash(filepath: &str) -> Result<String> {
    let file = fs::File::open(filepath).map_err(|e| {
        Box::<dyn std::error::Error>::from(format!("Failed to open file {}: {}", filepath, e))
    })?;

    let mut reader = BufReader::new(file);
    let mut hasher = Sha1::new();
    let mut buffer = [0; 8192]; // 8KB buffer

    loop {
        let bytes_read = reader.read(&mut buffer).map_err(|e| {
            Box::<dyn std::error::Error>::from(format!("Failed to read file {}: {}", filepath, e))
        })?;

        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    Ok(hex::encode(result))
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
        .map_err(|e| {
            Box::<dyn std::error::Error>::from(format!(
                "Failed to open output file '{}': {}",
                output_file, e
            ))
        })?;

    let json = serde_json::to_string_pretty(input).map_err(|e| {
        Box::<dyn std::error::Error>::from(format!("Failed to serialize JSON: {}", e))
    })?;

    file.write_all(json.as_bytes()).map_err(|e| {
        Box::<dyn std::error::Error>::from(format!("Failed to write JSON to file: {}", e))
    })?;

    Ok(())
}
