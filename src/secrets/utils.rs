use crate::secrets::error::Result;
use crate::secrets::models::JsonOutput;
use chrono::{DateTime, Local, TimeZone, Utc};
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};
use time::OffsetDateTime;

/// Extract required fields for validation
pub fn extract_validation_fields(entry: &Value) -> Result<(String, String, String)> {
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

/// Display details about a validation entry
pub fn details_about_entry(entry: &Value) -> Result<()> {
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

/// Get current timestamp formatted as RFC3339
pub fn get_current_timestamp() -> Result<String> {
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
pub fn get_hostname() -> Result<String> {
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