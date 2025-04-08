pub mod cloud_storage;
pub mod file_ops;
pub mod init;
pub mod output;
pub mod process;
pub mod ui;

// test_helpers moved to crate::testing::validation_helpers

use crate::args::Args;
use crate::secrets::error::Result;
use crate::secrets::models::JsonOutput;
use crate::utils::log_utils::Logger;

// Re-export main functions
pub use self::init::prepare_validation;
pub use self::process::process_retrieve_entry;

/// Validates secrets stored in Azure KeyVault or cloud storage against local files
///
/// # Errors
///
/// Returns an error if:
/// - Unable to read the input JSON file
/// - JSON parsing fails
/// - Required arguments are missing
/// - KeyVault client creation fails
pub fn validate(args: &Args, logger: &Logger) -> Result<()> {
    // This function now delegates to validation_retrieve, which implements the new behavior
    validation_retrieve(args, logger)
}

/// Retrieves and compares secrets from cloud storage
///
/// This is the new implementation that:
/// 1. Downloads files from Azure KeyVault, B2, or R2 storage
/// 2. Compares with local files (if they exist)
/// 3. Shows diffs and file details if requested
///
/// # Errors
///
/// Returns an error if:
/// - Unable to read the input JSON file
/// - JSON parsing fails
/// - Required arguments are missing
/// - KeyVault or storage client creation fails
/// - File operations fail
pub fn validation_retrieve(args: &Args, logger: &Logger) -> Result<()> {
    // Get client for Azure KeyVault (we still need this for entries using KeyVault)
    let (azure_client, json_values) = prepare_validation(args)?;

    // Create R2 client for cloud storage entries (used for both R2 and B2)
    let r2_client = crate::secrets::r2_storage::R2Client::from_args(args).map_err(|e| {
        Box::<dyn std::error::Error>::from(format!("Failed to create R2 client: {}", e))
    })?;

    // Get current hostname
    let hostname = crate::secrets::utils::get_hostname()?;
    let mut json_outputs: Vec<JsonOutput> = vec![];

    // Process each entry
    for entry in json_values {
        // Skip entries that don't match the current hostname
        if hostname != entry["hostname"].as_str().unwrap_or("") {
            logger.info(&format!(
                "Skipping entry {} for hostname: {}",
                entry["filenm"], entry["hostname"]
            ));
            continue;
        }

        // Process this entry
        match process_retrieve_entry(&entry, azure_client.as_ref(), &r2_client, args, logger) {
            Ok(Some(output)) => json_outputs.push(output),
            Ok(None) => {} // No output to add (skipped or error)
            Err(e) => eprintln!("Error processing entry: {}", e),
        }
    }

    // Write output if we have results
    if !json_outputs.is_empty() {
        output::write_validation_results(args, &json_outputs)?;
    }

    Ok(())
}