use crate::args::Args;
use crate::interfaces::AzureKeyVaultClient;
use crate::secrets::error::Result;
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

/// Decode base64 content to a NamedTempFile
///
/// Returns a NamedTempFile with the decoded content written to it
pub fn decode_base64_to_tempfile(base64_content: &[u8]) -> Result<NamedTempFile> {
    // Convert content to string
    let content_str = String::from_utf8_lossy(base64_content).to_string();

    // Decode the base64 content
    let decoded_content = base64::decode(&content_str).map_err(|e| {
        Box::<dyn std::error::Error>::from(format!("Error decoding base64 content: {}", e))
    })?;

    // Create a temporary file
    let mut temp_file = NamedTempFile::new().map_err(|e| {
        Box::<dyn std::error::Error>::from(format!("Failed to create temporary file: {}", e))
    })?;

    // Write the decoded content to the temp file
    temp_file.write_all(&decoded_content).map_err(|e| {
        Box::<dyn std::error::Error>::from(format!("Failed to write to temporary file: {}", e))
    })?;

    Ok(temp_file)
}

/// Compare a downloaded file with a local file
///
/// Returns true if files are identical, false if they differ
pub fn compare_files(
    downloaded_path: &str,
    local_path: &str,
    encoding: &str,
    _azure_client: &dyn AzureKeyVaultClient, // Not used but kept for clarity
    _entry: &Value,                          // Not used but kept for clarity
    _args: &Args,                            // Not used but kept for clarity
) -> Result<bool> {
    // Read both files
    let downloaded_content = fs::read(downloaded_path).map_err(|e| {
        Box::<dyn std::error::Error>::from(format!("Failed to read downloaded file: {}", e))
    })?;

    let local_content = fs::read(local_path).map_err(|e| {
        Box::<dyn std::error::Error>::from(format!("Failed to read local file: {}", e))
    })?;

    // For base64 encoding, we need to decode the downloaded content before comparison
    if encoding == "base64" {
        // Decode the downloaded content
        let temp_file = decode_base64_to_tempfile(&downloaded_content)?;

        // Read the decoded content from the temp file
        let decoded_content = fs::read(temp_file.path()).map_err(|e| {
            Box::<dyn std::error::Error>::from(format!("Failed to read from temp file: {}", e))
        })?;

        // Compare decoded content with local file
        Ok(decoded_content == local_content)
    } else {
        // Direct comparison for UTF-8 files
        Ok(downloaded_content == local_content)
    }
}

/// Check if a file exists at the specified path
pub fn check_file_exists(path: &str) -> bool {
    Path::new(path).exists()
}
