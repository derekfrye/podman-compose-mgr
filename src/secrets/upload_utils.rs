use std::path::MAIN_SEPARATOR;

/// Create an encoded secret name from a file path
/// 
/// This function takes a file path and converts it to a name suitable for
/// Azure Key Vault secrets:
/// 1. Replace path separators with dashes
/// 2. Replace periods (.) with dashes
/// 3. Encode any other special characters using hex encoding
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

/// Test utilities for Azure Key Vault operations
pub mod test_utils {
    use crate::secrets::models::SetSecretResponse;
    use time::OffsetDateTime;

    // For testing, we can override the get_secret_value and set_secret_value functions
    // This function is used to get a mock SecretResponse for testing
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