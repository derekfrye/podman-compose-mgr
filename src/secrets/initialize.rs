use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use chrono::{DateTime, Local};
use md5::{Digest, Md5};
use serde_json::json;

use crate::Args;
use crate::secrets::error::Result;
use crate::utils::cmd_utils;

/// Process the initialization of secrets
///
/// Reads the secrets from the input file, creates JSON entries for each file,
/// and writes the results to the input.json file in the specified directory.
pub fn process(args: &Args) -> Result<()> {
    // Get the required file paths from args
    let init_filepath = args.secrets_init_filepath.as_ref().unwrap();
    let output_filepath = args.secret_mode_input_json.as_ref().unwrap();
    
    // Read the input file containing the file paths
    let mut file = File::open(init_filepath)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    
    // Parse the JSON - it's either a map or an array of objects with filenm key
    let files: Vec<String> = if contents.trim().starts_with('{') {
        // Handle map format
        let files_map: HashMap<String, String> = serde_json::from_str(&contents)?;
        files_map.values().cloned().collect()
    } else {
        // Handle array format with objects that have filenm field
        let files_array: Vec<serde_json::Value> = serde_json::from_str(&contents)?;
        files_array.iter()
            .filter_map(|obj| obj.get("filenm").and_then(|v| v.as_str()).map(String::from))
            .collect()
    };
    
    // Get the hostname
    let hostname = hostname()?;
    
    // Create new entries for each file
    let mut new_entries = Vec::new();
    
    for filenm in files {
        // Check if file exists
        if !Path::new(&filenm).exists() {
            return Err(Box::<dyn std::error::Error>::from(
                format!("File {} does not exist", filenm),
            ));
        }
        
        // Calculate MD5
        let md5 = calculate_md5(&filenm)?;
        
        // Get current timestamp
        let now: DateTime<Local> = Local::now();
        let ins_ts = now.to_rfc3339();
        
        // Create JSON entry
        let entry = json!({
            "filenm": filenm,
            "md5": md5,
            "ins_ts": ins_ts,
            "az_id": "", // These will be filled in later by Azure Key Vault
            "az_create": "",
            "az_updated": "",
            "az_name": "",
            "hostname": hostname
        });
        
        new_entries.push(entry);
    }
    
    // Create or read existing entries
    let mut all_entries = Vec::new();
    
    // Check if output file already exists and read its contents if it does
    if Path::new(output_filepath).exists() {
        let mut existing_file = File::open(output_filepath)?;
        let mut existing_content = String::new();
        existing_file.read_to_string(&mut existing_content)?;
        
        if !existing_content.trim().is_empty() {
            // Parse existing JSON entries
            let existing_entries: Vec<serde_json::Value> = serde_json::from_str(&existing_content)?;
            
            // Add existing entries to all_entries
            all_entries.extend(existing_entries);
        }
    }
    
    // Store the count of new entries
    let new_entries_count = new_entries.len();
    
    // Add new entries to all_entries
    all_entries.extend(new_entries);
    
    // Write all entries to the output file
    let output_content = serde_json::to_string_pretty(&all_entries)?;
    let mut output_file = File::create(output_filepath)?;
    output_file.write_all(output_content.as_bytes())?;
    
    if args.verbose {
        println!("Successfully updated input.json with {} new entries", new_entries_count);
    }
    
    Ok(())
}

/// Calculate MD5 hash for a file
fn calculate_md5(filepath: &str) -> Result<String> {
    let mut file = File::open(filepath)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    
    let mut hasher = Md5::new();
    hasher.update(&buffer);
    let result = hasher.finalize();
    
    Ok(format!("{:x}", result))
}

/// Get the system hostname
fn hostname() -> Result<String> {
    let hostname = cmd_utils::run_command_with_output("hostname", &[])?
        .trim()
        .to_string();
    
    Ok(hostname)
}