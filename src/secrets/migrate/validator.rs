use crate::secrets::models::JsonOutput;
use crate::utils::error_utils::ErrorFromStr;

/// Validate that a single JsonOutput entry has the required fields
pub fn validate_input_json(entry: &JsonOutput) -> Result<(), Box<dyn std::error::Error>> {
    // Check if entry is empty
    if entry.is_empty() {
        return Err(Box::new(ErrorFromStr(
            "Entry is empty, missing required fields".to_string(),
        )));
    }

    // We no longer need to convert to JsonEntry for validation

    // Check if entry has hostname
    if entry.hostname.is_empty() {
        return Err(Box::new(ErrorFromStr(
            "Entry is missing the hostname field".to_string()
        )));
    }

    // Check if entry has file_name (file_nm in JsonOutput)
    if entry.file_nm.is_empty() {
        return Err(Box::new(ErrorFromStr(format!(
            "Entry for hostname '{}' is missing the file_nm field",
            entry.hostname
        ))));
    }

    // Check if entry has destination_cloud
    if entry.destination_cloud.is_empty() {
        return Err(Box::new(ErrorFromStr(format!(
            "Entry for file '{}' is missing the destination_cloud field",
            entry.file_nm
        ))));
    }

    // Ensure destination_cloud is one of the supported types
    match entry.destination_cloud.as_str() {
        "azure_kv" | "b2" | "r2" => {}
        _ => {
            return Err(Box::new(ErrorFromStr(format!(
                "Entry for file '{}' has an unsupported destination_cloud: '{}'. Must be 'azure_kv', 'b2', or 'r2'",
                entry.file_nm, entry.destination_cloud
            ))));
        }
    }

    Ok(())
}