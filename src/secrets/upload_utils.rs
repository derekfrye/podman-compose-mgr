// Function removed - now using hash directly

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
