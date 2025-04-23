use crate::secrets::error::Result;
use crate::secrets::s3::models::{S3Config, S3Provider, S3StorageClient};
use aws_config::retry::RetryConfig;
use aws_credential_types::Credentials;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::{BehaviorVersion, Region};

impl S3StorageClient {
    /// Create a new S3-compatible client from the provided config
    pub fn new(config: S3Config, verbose: u32) -> Result<Self> {
        // Check if this is a real client (non-mock values)
        let is_real_client = !(config.key_id == "dummy"
            && config.application_key == "dummy"
            && config.bucket == "dummy");

        // Create the endpoint based on the provider
        let endpoint = match config.provider {
            S3Provider::BackblazeB2 => "https://s3.us-west-004.backblazeb2.com".to_string(),
            S3Provider::CloudflareR2 => {
                // Handle mock client case specially
                if !is_real_client {
                    "https://mock-value.r2.cloudflarestorage.com".to_string()
                } else {
                    let account_id = config.account_id.ok_or_else(|| {
                        Box::<dyn std::error::Error>::from("Account ID is required for R2 storage")
                    })?;
                    format!("https://{}.r2.cloudflarestorage.com", account_id)
                }
            }
        };

        // Create the region based on the provider
        let region = match config.provider {
            S3Provider::BackblazeB2 => Region::new("us-west-004"),
            S3Provider::CloudflareR2 => Region::new("auto"),
        };

        // Create runtime for async operations - will be reused for all operations
        let runtime = tokio::runtime::Runtime::new().map_err(|e| {
            Box::<dyn std::error::Error>::from(format!("Failed to create runtime: {}", e))
        })?;

        // Use the tokio runtime to build the AWS config
        // If verbose, log connection details
        if verbose >= 2 {
            crate::utils::log_utils::debug(
                "Creating S3-compatible client with these parameters:",
                verbose as u8,
            );
            crate::utils::log_utils::debug(&format!("Endpoint: {}", endpoint), verbose as u8);
            crate::utils::log_utils::debug(&format!("Region: {:?}", region), verbose as u8);
            crate::utils::log_utils::debug(
                &format!(
                    "Provider type: {}",
                    match config.provider {
                        S3Provider::BackblazeB2 => "Backblaze B2",
                        S3Provider::CloudflareR2 => "Cloudflare R2",
                    }
                ),
                verbose as u8,
            );
            crate::utils::log_utils::debug(
                &format!(
                    "Key ID: {}****",
                    &config.key_id[..4.min(config.key_id.len())]
                ),
                verbose as u8,
            );
            crate::utils::log_utils::debug(
                &format!("Is real client: {}", is_real_client),
                verbose as u8,
            );
        }

        let aws_config = runtime.block_on(async {
            // Create credentials from the key ID and application key
            let credentials = Credentials::new(
                config.key_id.clone(),
                config.application_key.clone(),
                None, // No session token
                None, // No expiry
                match config.provider {
                    S3Provider::BackblazeB2 => "B2StaticCredentials",
                    S3Provider::CloudflareR2 => "R2StaticCredentials",
                },
            );

            // Build the S3 configuration with more verbose logging
            let builder = aws_sdk_s3::Config::builder()
                .region(region)
                .endpoint_url(endpoint)
                .credentials_provider(credentials)
                .retry_config(RetryConfig::standard().with_max_attempts(3))
                .behavior_version(BehaviorVersion::latest());

            // Build the final config
            builder.build()
        });

        // Create client
        let client = Client::from_conf(aws_config);

        let storage_client = Self {
            bucket_name: config.bucket.clone(),
            client,
            runtime,
            provider_type: config.provider,
            is_real_client,
            verbose,
        };

        // Special handling for placeholder bucket names and non-real clients
        let is_placeholder_bucket =
            config.bucket == "placeholder_bucket_will_be_provided_from_json";

        // Only ensure bucket exists for real clients with non-placeholder bucket names
        if is_real_client && !is_placeholder_bucket {
            // Ensure bucket exists for real clients
            storage_client.ensure_bucket_exists(&config.bucket)?;
        }

        Ok(storage_client)
    }

    /// Set the bucket name
    pub fn set_bucket_name(&mut self, bucket: String) {
        self.bucket_name = bucket;
    }

    /// Get the provider type
    pub fn provider_type(&self) -> &S3Provider {
        &self.provider_type
    }
}
