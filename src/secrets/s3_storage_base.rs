use crate::secrets::error::Result;
use crate::secrets::file_details::FileDetails;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::Region;
use aws_sdk_s3::primitives::ByteStream;
use aws_credential_types::Credentials;
use aws_config::retry::RetryConfig;
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
}

/// Base client for S3-compatible storage providers
pub struct S3StorageClient {
    bucket_name: String,
    client: Client,
    runtime: tokio::runtime::Runtime,
    provider_type: S3Provider,
}

impl S3StorageClient {
    /// Create a new S3-compatible client from the provided config
    pub fn new(config: S3Config) -> Result<Self> {
        // Create the endpoint based on the provider
        let endpoint = match config.provider {
            S3Provider::BackblazeB2 => "https://s3.us-west-004.backblazeb2.com".to_string(),
            S3Provider::CloudflareR2 => {
                let account_id = config.account_id.ok_or_else(|| {
                    Box::<dyn std::error::Error>::from("Account ID is required for R2 storage")
                })?;
                format!("https://{}.r2.cloudflarestorage.com", account_id)
            }
        };
        
        // Create the region based on the provider
        let region = match config.provider {
            S3Provider::BackblazeB2 => Region::new("us-west-004"),
            S3Provider::CloudflareR2 => Region::new("auto"),
        };
        
        // Create runtime for async operations - will be reused for all operations
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to create runtime: {}", e)))?;
        
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
        };
        
        // Ensure bucket exists
        storage_client.ensure_bucket_exists(&config.bucket)?;
        
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
        // Use the client's runtime instead of creating a new one
        let result = self.runtime.block_on(async {
            let resp = self.client.head_object()
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
        // Check if file exists first
        let exists = self.file_exists(file_key)?;
        if !exists {
            return Ok(None);
        }
        
        // Use the client's runtime instead of creating a new one
        let metadata = self.runtime.block_on(async {
            let resp = self.client.head_object()
                .bucket(&self.bucket_name)
                .key(file_key)
                .send()
                .await
                .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to get file metadata: {}", e)))?;
            
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
    
    /// Upload a file to storage
    pub fn upload_file(&self, local_path: &str, object_key: &str, metadata: Option<HashMap<String, String>>) -> Result<S3UploadResult> {
        // Check if the file exists locally
        if !Path::new(local_path).exists() {
            return Err(Box::<dyn std::error::Error>::from(
                format!("Local file does not exist: {}", local_path)
            ));
        }
        
        // Use the client's runtime instead of creating a new one
        let result = self.runtime.block_on(async {
            // Create ByteStream directly from file path - no loading into memory
            let body = ByteStream::from_path(Path::new(local_path)).await
                .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to create ByteStream from path: {}", e)))?;
            
            // Build the put_object request
            let mut put_request = self.client.put_object()
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
            let response = put_request.send().await
                .map_err(|e| Box::<dyn std::error::Error>::from(
                    format!("Failed to upload to storage: {}", e)
                ))?;
            
            // Get the ETag (hash) from the response
            let etag = response.e_tag()
                .ok_or_else(|| Box::<dyn std::error::Error>::from("No ETag in response"))?
                .replace("\"", ""); // Remove quotes from ETag
            
            Ok::<S3UploadResult, Box<dyn std::error::Error>>(S3UploadResult {
                hash: etag.clone(),
                id: etag.clone(),
                bucket_id: self.bucket_name.clone(),
                name: object_key.to_string(),
            })
        })?;
        
        Ok(result)
    }
    
    /// Upload a file with details from FileDetails struct
    pub fn upload_file_with_details(&self, file_details: &FileDetails) -> Result<S3UploadResult> {
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
        
        // If a specific bucket is set in the file details, add it to metadata
        if let Some(bucket) = &file_details.cloud_upload_bucket {
            metadata.insert("bucket".to_string(), bucket.clone());
        }
        
        // Determine the path in storage based on bucket details
        let prefix = if let Some(bucket) = &file_details.cloud_upload_bucket {
            if !bucket.is_empty() {
                format!("{}/secrets", bucket)
            } else {
                "secrets".to_string()
            }
        } else {
            "secrets".to_string()
        };
        
        // Use hash as the object key for deduplication and consistent naming
        let object_key = format!("{}/{}", prefix, file_details.hash);
        
        // Upload the file
        self.upload_file(&file_to_use, &object_key, Some(metadata))
    }
    
    /// Download a file from storage
    pub fn download_file(&self, object_key: &str) -> Result<Vec<u8>> {
        // Use the client's runtime instead of creating a new one
        let content = self.runtime.block_on(async {
            let resp = self.client.get_object()
                .bucket(&self.bucket_name)
                .key(object_key)
                .send()
                .await
                .map_err(|e| Box::<dyn std::error::Error>::from(
                    format!("Failed to download from storage: {}", e)
                ))?;
            
            // Get content as bytes
            let bytes = resp.body.collect().await
                .map_err(|e| Box::<dyn std::error::Error>::from(
                    format!("Failed to read response body: {}", e)
                ))?;
                
            Ok::<Vec<u8>, Box<dyn std::error::Error>>(bytes.to_vec())
        })?;
        
        Ok(content)
    }
    
    /// Save downloaded content to a file
    pub fn save_to_file(&self, object_key: &str, local_path: &str) -> Result<()> {
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
        .map_err(|e| Box::<dyn std::error::Error>::from(
            format!("Failed to open file: {}", e)
        ))?
        .read_to_string(&mut value)
        .map_err(|e| Box::<dyn std::error::Error>::from(
            format!("Failed to read file: {}", e)
        ))?;
    
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