use crate::args::Args;
use crate::interfaces::AzureKeyVaultClient;
use crate::secrets::error::Result;
use serde_json::Value;
use std::fs;
use std::path::Path;

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
        // Decode the downloaded content (if it's base64 encoded text)
        let downloaded_str = String::from_utf8_lossy(&downloaded_content).to_string();
        let decoded_content = match base64::decode(downloaded_str) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Error decoding base64 content: {}", e);
                // We can't decode, so compare the raw content
                downloaded_content
            }
        };

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