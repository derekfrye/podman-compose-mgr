use crate::args::types::Args;
use crate::secrets::models::{JsonOutput, JsonOutputCollection};
use crate::utils::error_utils::ErrorFromStr;
use std::fs::File;
use std::io::Read;

use super::validator::validate_input_json;
use super::migrate_process::migrate;

/// Initialize the secrets migration process
pub fn init_migrate(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    // Get the input JSON file path
    let input_json_path = args
        .input_json
        .as_ref()
        .ok_or_else(|| ErrorFromStr("Input JSON path is required".to_string()))?;

    // Read the input JSON file
    let mut file = File::open(input_json_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    // Try to parse the JSON content as an array first
    let entries_result: Result<JsonOutputCollection, _> = serde_json::from_str(&content);
    
    match entries_result {
        Ok(entries_collection) => {
            // Successfully parsed as an array of JsonOutput objects
            if entries_collection.is_empty() {
                return Err(Box::new(ErrorFromStr("Input JSON is empty, no entries to migrate".to_string())));
            }
            
            // Validate each entry in the collection
            for (i, entry) in entries_collection.iter().enumerate() {
                validate_input_json(entry)
                    .map_err(|e| Box::new(ErrorFromStr(format!("Entry at index {}: {}", i, e))))?;
            }
            
            // Process each entry in the collection
            for entry in &entries_collection {
                migrate(args, entry)?;
            }
        },
        Err(_) => {
            // If it's not an array, try parsing as a single JsonOutput object
            let entry: JsonOutput = serde_json::from_str(&content)?;
            
            // Validate the single entry
            validate_input_json(&entry)?;
            
            // Proceed with migration for the single entry
            migrate(args, &entry)?;
        }
    }

    Ok(())
}