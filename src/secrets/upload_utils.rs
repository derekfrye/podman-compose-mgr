use std::path::MAIN_SEPARATOR;

/// Create a secret name from a file hash
///
/// This function creates a suitable name for either Azure Key Vault or B2 storage
/// using the file's SHA-1 hash
pub fn create_secret_name(hash: &str) -> String {
    // Use a prefix for clarity and add the hash
    format!("file-{}", hash)
}

/// Legacy function to create an encoded secret name from a file path
///
/// This function takes a file path and converts it to a name suitable for
/// Azure Key Vault secrets:
/// 1. Replace path separators with dashes
/// 2. Replace periods (.) with dashes
/// 3. Encode any other special characters using hex encoding
#[deprecated(since = "0.2.0", note = "Please use create_secret_name instead")]
pub fn create_encoded_secret_name(file_path: &str) -> String {
    // First replace path separators and periods with dashes
    let secret_name = file_path.replace([MAIN_SEPARATOR, '.'], "-");

    // Replace spaces and other problematic characters with URL-encoding
    // Note: In Azure Key Vault, secret names can only contain alphanumeric characters and dashes
    let mut encoded_name = String::new();

    for c in secret_name.chars() {
        if c.is_alphanumeric() || c == '-' {
            encoded_name.push(c);
        } else {
            // For space and other special characters, convert to percent encoding
            // but use their hex value directly
            for byte in c.to_string().as_bytes() {
                encoded_name.push_str(&format!("-{:02X}", byte));
            }
        }
    }

    encoded_name
}

/// Determines which storage backend to use based on file size and explicit destination
pub fn determine_storage_backend(
    destination_cloud: Option<&str>,
    encoded_size: u64,
) -> &'static str {
    match destination_cloud {
        Some("b2") => "b2",
        Some("azure_kv") => "azure_kv",
        _ => {
            // If no explicit destination, decide based on size
            if encoded_size > 24000 {
                "b2"
            } else {
                "azure_kv"
            }
        }
    }
}

/// Test utilities for cloud storage operations
pub mod test_utils {
    use crate::secrets::models::SetSecretResponse;
    use time::OffsetDateTime;

    /// Create a mock response for Azure KeyVault
    pub fn get_mock_secret_response(name: &str, value: &str) -> SetSecretResponse {
        let now = OffsetDateTime::now_utc();
        SetSecretResponse {
            created: now,
            updated: now,
            name: name.to_string(),
            id: format!("https://keyvault.vault.azure.net/secrets/{}", name),
            value: value.to_string(),
        }
    }
}
