use crate::secrets::error::Result;
use crate::secrets::file_details::FileDetails;
use aws_config::retry::RetryConfig;
use aws_credential_types::Credentials;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::Region;
use aws_sdk_s3::primitives::ByteStream;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Storage provider type
pub enum S3Provider {
    BackblazeB2,
    CloudflareR2,
}

/// Configuration for an S3-compatible storage client
pub struct S3Config {
    pub key_id: String,
    pub application_key: String,
    pub bucket: String,
    pub provider: S3Provider,
    pub account_id: Option<String>, // Only needed for R2
}

/// Represents the result of an S3 upload operation
pub struct S3UploadResult {
    pub hash: String,
    pub id: String,
    pub bucket_id: String,
    pub name: String,
    pub created: String,
    pub updated: String,
}

/// Base client for S3-compatible storage providers
pub struct S3StorageClient {
    bucket_name: String,
    client: Client,
    runtime: tokio::runtime::Runtime,
    provider_type: S3Provider,
    is_real_client: bool,
    pub verbose: u32, // Using u32 for multiple verbosity levels
}

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

            // Build the S3 configuration
            aws_sdk_s3::Config::builder()
                .region(region)
                .endpoint_url(endpoint)
                .credentials_provider(credentials)
                .retry_config(RetryConfig::standard().with_max_attempts(3))
                .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
                .build()
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

    /// Ensure bucket exists, create it if it doesn't
    fn ensure_bucket_exists(&self, bucket_name: &str) -> Result<()> {
        // Use the client's runtime instead of creating a new one
        self.runtime.block_on(async {
            // Check if bucket exists
            let bucket_exists = self.client.head_bucket()
                .bucket(bucket_name)
                .send()
                .await
                .is_ok();
                
            // Create bucket if it doesn't exist
            if !bucket_exists {
                println!("Bucket '{}' doesn't exist, creating it...", bucket_name);
                
                // Create bucket without location constraint (works for both B2 and R2)
                let result = self.client.create_bucket()
                    .bucket(bucket_name)
                    .send()
                    .await;
                    
                match result {
                    Ok(_) => println!("Successfully created bucket '{}'", bucket_name),
                    Err(e) => {
                        // If error is because bucket already exists, that's fine
                        let err_str = format!("{}", e);
                        println!("Bucket creation error details: {}", err_str);
                        
                        if err_str.contains("BucketAlreadyExists") || 
                           err_str.contains("BucketAlreadyOwnedByYou") || 
                           err_str.contains("already exists") {
                            println!("Bucket '{}' already exists, proceeding...", bucket_name);
                            return Ok(());
                        }
                        
                        // For S3 providers, we'll assume the bucket already exists in production and continue
                        // This is because some accounts may not have bucket creation permissions
                        // but still have upload permissions to existing buckets
                        println!("WARNING: Failed to create bucket '{}', but will attempt to use it anyway", bucket_name);
                        
                        // Instead of failing, we'll try to continue
                        // If the bucket truly doesn't exist, operations will fail later naturally
                        return Ok(());
                    }
                }
            }
            
            Ok(())
        })
    }

    /// Check if a file exists in storage
    pub fn file_exists(&self, file_key: &str) -> Result<bool> {
        // For non-real clients, return false to indicate file doesn't exist
        if !self.is_real_client {
            if self.verbose >= 1 {
                println!(
                    "info: Using mock S3-compatible storage client - would have checked if {} exists",
                    file_key
                );
            }
            return Ok(false);
        }

        // Use the client's runtime instead of creating a new one
        let result = self.runtime.block_on(async {
            let resp = self
                .client
                .head_object()
                .bucket(&self.bucket_name)
                .key(file_key)
                .send()
                .await;

            Ok::<bool, Box<dyn std::error::Error>>(resp.is_ok())
        })?;

        Ok(result)
    }

    /// Get file metadata
    pub fn get_file_metadata(&self, file_key: &str) -> Result<Option<HashMap<String, String>>> {
        // For non-real clients, return None to indicate file doesn't exist
        if !self.is_real_client {
            if self.verbose >= 1 {
                println!(
                    "info: Using mock S3-compatible storage client - would have retrieved metadata for {}",
                    file_key
                );
            }
            return Ok(None);
        }

        // Check if file exists first
        let exists = self.file_exists(file_key)?;
        if !exists {
            return Ok(None);
        }

        // Use the client's runtime instead of creating a new one
        let metadata = self.runtime.block_on(async {
            let resp = self
                .client
                .head_object()
                .bucket(&self.bucket_name)
                .key(file_key)
                .send()
                .await
                .map_err(|e| {
                    Box::<dyn std::error::Error>::from(format!(
                        "Failed to get file metadata: {}",
                        e
                    ))
                })?;

            let mut result = HashMap::new();

            // Get standard metadata
            if let Some(content_type) = resp.content_type() {
                result.insert("content_type".to_string(), content_type.to_string());
            }

            // Content length is an Option<i64>
            if let Some(content_length) = resp.content_length() {
                result.insert("content_length".to_string(), content_length.to_string());
            }

            if let Some(etag) = resp.e_tag() {
                result.insert("etag".to_string(), etag.to_string());
            }

            if let Some(last_modified) = resp.last_modified() {
                result.insert("last_modified".to_string(), format!("{:?}", last_modified));
            }

            // Get user metadata
            if let Some(metadata) = resp.metadata() {
                for (key, value) in metadata {
                    result.insert(key.to_string(), value.to_string());
                }
            }

            Ok::<HashMap<String, String>, Box<dyn std::error::Error>>(result)
        })?;

        Ok(Some(metadata))
    }

    /// Check if a file exists and get its metadata for specific path
    pub fn check_file_exists_with_details(
        &self,
        hash: &str,
        bucket_name: Option<&str>,
        prefix: Option<&str>,
    ) -> Result<Option<(bool, String, String)>> {
        // Construct the object key based on whether a prefix is provided
        let object_key = if let Some(prefix_path) = prefix {
            format!("{}/{}", prefix_path, hash)
        } else {
            // No prefix, use hash directly as the key
            hash.to_string()
        };

        // For non-real clients, return mock data
        if !self.is_real_client {
            if self.verbose >= 1 {
                println!(
                    "info: Using mock S3-compatible storage client - would have checked if file with hash {} exists",
                    hash
                );
            }
            // Return mock data for testing: (exists, created_time, updated_time)
            return Ok(Some((
                true,
                "2025-01-01T00:00:00Z".to_string(),
                "2025-01-01T00:00:00Z".to_string(),
            )));
        }

        // Determine which bucket to use
        let bucket_to_use = if let Some(b) = bucket_name {
            b
        } else {
            // Check if we need to use a placeholder bucket
            if self.bucket_name == "placeholder_bucket_will_be_provided_from_json" {
                // We can't check for existence if the bucket name is a placeholder
                if self.verbose >= 1 {
                    println!(
                        "info: Can't check if file exists because bucket name is a placeholder. Please provide bucket name in JSON."
                    );
                }
                return Ok(None);
            }
            &self.bucket_name
        };

        if self.verbose >= 2 {
            println!(
                "dbg: Checking if object '{}' exists in bucket '{}'",
                object_key, bucket_to_use
            );
        }

        // Use the client's runtime to check if the object exists
        let result =
            self.runtime.block_on(async {
                let resp = self
                    .client
                    .head_object()
                    .bucket(bucket_to_use)
                    .key(&object_key)
                    .send()
                    .await;

                if resp.is_err() {
                    if self.verbose >= 2 {
                        println!(
                            "dbg: Object does not exist or error occurred: {:?}",
                            resp.err()
                        );
                    }
                    return Ok::<Option<(bool, String, String)>, Box<dyn std::error::Error>>(Some(
                        (false, "".to_string(), "".to_string()),
                    ));
                }

                let response = resp.unwrap();

                // Get last_modified as a string
                let last_modified = if let Some(lm) = response.last_modified() {
                    // Use standard DateTime formatting for AWS DateTime
                    format!("{}", lm)
                } else {
                    "Unknown".to_string()
                };

                if self.verbose >= 2 {
                    println!(
                        "dbg: Object exists with last modified time: {}",
                        last_modified
                    );
                }

                // For S3 compatible services, we don't have created time, so use last_modified
                Ok(Some((true, last_modified.clone(), last_modified)))
            })?;

        Ok(result)
    }

    /// Upload a file to storage
    pub fn upload_file(
        &self,
        local_path: &str,
        object_key: &str,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<S3UploadResult> {
        // For non-real clients, return a mock response without trying to upload
        if !self.is_real_client {
            // Generate a hash from the local_path for consistency in tests
            let hash = format!("mock-hash-{}", local_path);

            if self.verbose >= 1 {
                println!(
                    "info: Using mock S3-compatible storage client - would have uploaded {} to {}",
                    local_path, object_key
                );
            }

            return Ok(S3UploadResult {
                hash: hash.clone(),
                id: hash,
                bucket_id: self.bucket_name.clone(),
                name: object_key.to_string(),
                created: "2025-01-01T00:00:00Z".to_string(),
                updated: "2025-01-01T00:00:00Z".to_string(),
            });
        }

        // Check for placeholder bucket - this method should never be called with a placeholder bucket
        // The upload_file_with_details method handles placeholders specially
        if self.bucket_name == "placeholder_bucket_will_be_provided_from_json" {
            return Err(Box::<dyn std::error::Error>::from(
                "Cannot upload directly with a placeholder bucket - use upload_file_with_details instead",
            ));
        }

        // Check if the file exists locally
        if !Path::new(local_path).exists() {
            return Err(Box::<dyn std::error::Error>::from(format!(
                "Local file does not exist: {}",
                local_path
            )));
        }

        // Use the client's runtime instead of creating a new one
        let result = self.runtime.block_on(async {
            // Create ByteStream directly from file path - no loading into memory
            let body = ByteStream::from_path(Path::new(local_path))
                .await
                .map_err(|e| {
                    Box::<dyn std::error::Error>::from(format!(
                        "Failed to create ByteStream from path: {}",
                        e
                    ))
                })?;

            // Build the put_object request
            let mut put_request = self
                .client
                .put_object()
                .bucket(&self.bucket_name)
                .key(object_key)
                .body(body);

            // Add metadata if provided
            if let Some(meta) = metadata {
                for (key, value) in meta {
                    put_request = put_request.metadata(key, value);
                }
            }

            // Send the request
            let response = put_request.send().await.map_err(|e| {
                Box::<dyn std::error::Error>::from(format!("Failed to upload to storage: {}", e))
            })?;

            // Get the ETag (hash) from the response
            let etag = response
                .e_tag()
                .ok_or_else(|| Box::<dyn std::error::Error>::from("No ETag in response"))?
                .replace("\"", ""); // Remove quotes from ETag

            // Get current time for timestamps
            let current_time = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

            Ok::<S3UploadResult, Box<dyn std::error::Error>>(S3UploadResult {
                hash: etag.clone(),
                id: etag.clone(),
                bucket_id: self.bucket_name.clone(),
                name: object_key.to_string(),
                created: current_time.clone(),
                updated: current_time,
            })
        })?;

        Ok(result)
    }

    /// Upload a file with details from FileDetails struct
    pub fn upload_file_with_details(&self, file_details: &FileDetails) -> Result<S3UploadResult> {
        // Check if cloud_upload_bucket is provided for B2/R2 storage
        if (matches!(self.provider_type, S3Provider::BackblazeB2)
            || matches!(self.provider_type, S3Provider::CloudflareR2))
            && file_details.cloud_upload_bucket.is_none()
        {
            return Err(Box::<dyn std::error::Error>::from(
                "cloud_upload_bucket is required in JSON for B2/R2 storage",
            ));
        }

        // For non-real clients, return mock data
        if !self.is_real_client {
            let hash = format!("mock-hash-{}", file_details.hash);

            // Determine the object key based on whether a prefix is provided
            let name = if let Some(prefix) = &file_details.cloud_prefix {
                format!("{}/{}", prefix, file_details.hash)
            } else {
                // No prefix, use hash directly as the key
                file_details.hash.clone()
            };

            if self.verbose >= 1 {
                println!(
                    "info: Using mock S3-compatible storage client - would have uploaded {} using hash {}",
                    file_details.file_path, file_details.hash
                );
            }

            return Ok(S3UploadResult {
                hash: hash.clone(),
                id: hash,
                bucket_id: file_details
                    .cloud_upload_bucket
                    .clone()
                    .unwrap_or_else(|| "mock-bucket".to_string()),
                name,
                created: "2025-01-01T00:00:00Z".to_string(),
                updated: "2025-01-01T00:00:00Z".to_string(),
            });
        }

        // Determine which file to use - original or base64 encoded version
        let file_to_use = if file_details.encoding == "base64" {
            format!("{}.base64", file_details.file_path)
        } else {
            file_details.file_path.clone()
        };

        // Create metadata for the file
        let mut metadata = HashMap::new();
        metadata.insert("original-path".to_string(), file_details.file_path.clone());
        metadata.insert("hash".to_string(), file_details.hash.clone());
        metadata.insert("hash-algo".to_string(), file_details.hash_algo.clone());
        metadata.insert("encoding".to_string(), file_details.encoding.clone());
        metadata.insert("size".to_string(), file_details.file_size.to_string());

        // Check if we need to use a real bucket name from the JSON for upload
        let using_placeholder_bucket =
            self.bucket_name == "placeholder_bucket_will_be_provided_from_json";

        // When using a placeholder bucket, ensure a real bucket name is provided in the JSON
        let upload_bucket = if using_placeholder_bucket {
            // We need a real bucket name from the JSON
            match &file_details.cloud_upload_bucket {
                Some(bucket) if !bucket.is_empty() => bucket.clone(),
                _ => {
                    return Err(Box::<dyn std::error::Error>::from(
                        "cloud_upload_bucket must be provided in JSON when using placeholder bucket configuration",
                    ));
                }
            }
        } else {
            // Use the bucket configured in the client
            self.bucket_name.clone()
        };

        // Add bucket to metadata
        metadata.insert("bucket".to_string(), upload_bucket.clone());

        // Determine the object key using the prefix if provided
        let object_key = if let Some(prefix) = &file_details.cloud_prefix {
            format!("{}/{}", prefix, file_details.hash)
        } else {
            // No prefix, use hash directly as the key
            file_details.hash.clone()
        };

        // If we're using a placeholder bucket, we need to ensure the real bucket exists before upload
        if using_placeholder_bucket {
            if self.verbose >= 1 {
                println!(
                    "info: Using real bucket name '{}' from JSON for upload",
                    upload_bucket
                );
            }
            // Ensure the real bucket exists
            self.ensure_bucket_exists(&upload_bucket)?;
        }

        // Upload the file - special handling for placeholder bucket
        if using_placeholder_bucket {
            // For placeholder bucket, use the actual bucket from JSON for upload
            // This requires a custom implementation because the client was configured with a placeholder bucket
            let result = self.runtime.block_on(async {
                // Create ByteStream from file path
                let body = ByteStream::from_path(Path::new(&file_to_use))
                    .await
                    .map_err(|e| {
                        Box::<dyn std::error::Error>::from(format!(
                            "Failed to create ByteStream from path: {}",
                            e
                        ))
                    })?;

                // Build the put_object request with the real bucket
                let mut put_request = self
                    .client
                    .put_object()
                    .bucket(&upload_bucket) // Use real bucket from JSON
                    .key(&object_key)
                    .body(body);

                // Add metadata
                for (key, value) in &metadata {
                    put_request = put_request.metadata(key, value);
                }

                // Send the request
                let response = put_request.send().await.map_err(|e| {
                    Box::<dyn std::error::Error>::from(format!(
                        "Failed to upload to storage: {}",
                        e
                    ))
                })?;

                // Get the ETag (hash) from the response
                let etag = response
                    .e_tag()
                    .ok_or_else(|| Box::<dyn std::error::Error>::from("No ETag in response"))?
                    .replace("\"", ""); // Remove quotes from ETag

                // Get current time for timestamps
                let current_time = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

                Ok::<S3UploadResult, Box<dyn std::error::Error>>(S3UploadResult {
                    hash: etag.clone(),
                    id: etag.clone(),
                    bucket_id: upload_bucket.clone(),
                    name: object_key.to_string(),
                    created: current_time.clone(),
                    updated: current_time,
                })
            })?;

            Ok(result)
        } else {
            // For non-placeholder bucket, use the regular upload method
            self.upload_file(&file_to_use, &object_key, Some(metadata))
        }
    }

    /// Download a file from storage
    pub fn download_file(&self, object_key: &str) -> Result<Vec<u8>> {
        // For non-real clients, return a mock response
        if !self.is_real_client {
            if self.verbose >= 1 {
                println!(
                    "info: Using mock S3-compatible storage client - would have downloaded {}",
                    object_key
                );
            }
            // Return a small mock content
            return Ok(b"mock content for testing".to_vec());
        }

        // Use the client's runtime instead of creating a new one
        let content = self.runtime.block_on(async {
            let resp = self
                .client
                .get_object()
                .bucket(&self.bucket_name)
                .key(object_key)
                .send()
                .await
                .map_err(|e| {
                    Box::<dyn std::error::Error>::from(format!(
                        "Failed to download from storage: {}",
                        e
                    ))
                })?;

            // Get content as bytes
            let bytes = resp.body.collect().await.map_err(|e| {
                Box::<dyn std::error::Error>::from(format!("Failed to read response body: {}", e))
            })?;

            Ok::<Vec<u8>, Box<dyn std::error::Error>>(bytes.to_vec())
        })?;

        Ok(content)
    }

    /// Save downloaded content to a file
    pub fn save_to_file(&self, object_key: &str, local_path: &str) -> Result<()> {
        // For non-real clients, create a mock file
        if !self.is_real_client {
            if self.verbose >= 1 {
                println!(
                    "info: Using mock S3-compatible storage client - would have downloaded {} to {}",
                    object_key, local_path
                );
            }
            // Create a parent directory if it doesn't exist
            if let Some(parent) = Path::new(local_path).parent() {
                std::fs::create_dir_all(parent)?;
            }
            // Write a mock file
            std::fs::write(local_path, b"mock content for testing")?;
            return Ok(());
        }

        // Get content
        let content = self.download_file(object_key)?;

        // Write to local file
        std::fs::write(local_path, content)?;

        Ok(())
    }

    /// Get the provider type
    pub fn provider_type(&self) -> &S3Provider {
        &self.provider_type
    }
}

/// Helper function to read a value from a file
pub fn read_value_from_file(file_path: &Path) -> Result<String> {
    let mut value = String::new();
    File::open(file_path)
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to open file: {}", e)))?
        .read_to_string(&mut value)
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to read file: {}", e)))?;

    // Trim whitespace and newlines
    Ok(value.trim().to_string())
}

/// Helper function to get a bucket name, either from a direct value or from a file
pub fn get_bucket_name(bucket_path: &str) -> Result<String> {
    // Check if this is a file path
    if Path::new(bucket_path).exists() {
        // Read bucket name from file
        read_value_from_file(Path::new(bucket_path))
    } else {
        // Use value directly
        Ok(bucket_path.to_string())
    }
}
