use crate::args::types::Args;
use crate::secrets::models::JsonOutput;
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

    // Parse the JSON content
    let entries: JsonOutput = serde_json::from_str(&content)?;

    // Validate the input JSON structure
    validate_input_json(&entries)?;

    // Proceed with migration
    migrate(args, &entries)?;

    Ok(())
}