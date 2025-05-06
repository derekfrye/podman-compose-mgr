use crate::secrets::models::JsonOutput;
use crate::utils::error_utils::ErrorFromStr;

/// Validate that the input JSON matches the expected JsonOutput structure
pub fn validate_input_json(entries: &JsonOutput) -> Result<(), Box<dyn std::error::Error>> {
    // Check if entries is empty
    if entries.is_empty() {
        return Err(Box::new(ErrorFromStr(
            "Input JSON is empty, no entries to migrate".to_string(),
        )));
    }

    // Check each entry for required fields
    for (index, entry) in entries.iter().enumerate() {
        // Check if entry has hostname
        if entry.hostname.is_empty() {
            return Err(Box::new(ErrorFromStr(format!(
                "Entry at index {} is missing the hostname field",
                index
            ))));
        }

        // Check if entry has file_name
        if entry.file_name.is_empty() {
            return Err(Box::new(ErrorFromStr(format!(
                "Entry at index {} for hostname '{}' is missing the file_name field",
                index, entry.hostname
            ))));
        }

        // Check if entry has destination_cloud
        if entry.destination_cloud.is_empty() {
            return Err(Box::new(ErrorFromStr(format!(
                "Entry at index {} for file '{}' is missing the destination_cloud field",
                index, entry.file_name
            ))));
        }

        // Ensure destination_cloud is one of the supported types
        match entry.destination_cloud.as_str() {
            "azure_kv" | "b2" | "r2" => {}
            _ => {
                return Err(Box::new(ErrorFromStr(format!(
                    "Entry at index {} for file '{}' has an unsupported destination_cloud: '{}'. Must be 'azure_kv', 'b2', or 'r2'",
                    index, entry.file_name, entry.destination_cloud
                ))));
            }
        }
    }

    Ok(())
}