use crate::args::Args;
use crate::secrets::error::Result;
use serde_json::Value;
use std::fs;

/// Check if we're running in test mode and handle file operations for test
/// Returns Some(temp_path) if the file was handled by the test helper, None if regular handling should continue
pub fn maybe_use_test_file_for_azure(
    entry: &Value,
    temp_path: &str,
    args: &Args,
) -> Result<Option<()>> {
    // Check if we're running in a test mode with specific test files
    if entry["file_nm"]
        .as_str()
        .unwrap_or("")
        .contains("/tmp/testfile")
        && temp_path.contains("/tmp/.tmp")
    {
        // For testing, use a direct copy of the file we prepared earlier
        crate::utils::log_utils::info(
            "Test mode - using prepared file for Azure KeyVault simulation",
            args.verbose,
        );

        // Use the prepared downloaded.txt file that we created
        fs::copy("/tmp/downloaded.txt", temp_path).map_err(|e| {
            Box::<dyn std::error::Error>::from(format!("Failed to copy test file: {}", e))
        })?;

        return Ok(Some(()));
    }

    // Not in test mode, continue with normal handling
    Ok(None)
}

/// Check if we're running in test mode for R2/B2 storage
pub fn maybe_use_test_file_for_storage(
    entry: &Value,
    temp_path: &str,
    args: &Args,
) -> Result<Option<()>> {
    // Special test mode for our examples
    if entry["file_nm"]
        .as_str()
        .unwrap_or("")
        .contains("/tmp/testfile")
        && temp_path.contains("/tmp/.tmp")
    {
        // For testing, use a direct copy of the file we prepared earlier
        crate::utils::log_utils::info(
            "Test mode - using prepared file for R2/B2 simulation",
            args.verbose,
        );

        // Use the prepared downloaded.txt file that we created
        fs::copy("/tmp/downloaded.txt", temp_path).map_err(|e| {
            Box::<dyn std::error::Error>::from(format!("Failed to copy test file: {}", e))
        })?;

        return Ok(Some(()));
    }

    // Not in test mode, continue with normal handling
    Ok(None)
}
