use crate::args::Args;
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

/// Configuration for the R2 client
pub struct R2Config {
    pub key_id: String,
    pub application_key: String,
    pub bucket: String,
    pub account_id: String,
}

/// Represents the result of an R2 upload operation
pub struct R2UploadResult {
    pub hash: String,
    pub id: String,
    pub bucket_id: String,
    pub name: String,
}

/// Represents an R2 storage client using S3-compatible API
pub struct R2Client {
    bucket_name: String,
    client: Client,
}

impl R2Client {
    /// Create a new R2 client from the provided config
    pub fn new(config: R2Config) -> Result<Self> {
        // Cloudflare R2 S3-compatible endpoint
        let endpoint = format!("https://{}.r2.cloudflarestorage.com", config.account_id);
        
        // Create runtime for async operation
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to create runtime: {}", e)))?;
        
        // Use the tokio runtime to build the AWS config
        let aws_config = runtime.block_on(async {
            // Create credentials from the R2 key ID and application key
            let credentials = Credentials::new(
                config.key_id.clone(),
                config.application_key.clone(),
                None, // No session token
                None, // No expiry
                "R2StaticCredentials",
            );
            
            // Build the S3 configuration - R2 uses auto region 
            aws_sdk_s3::Config::builder()
                .region(Region::new("auto"))
                .endpoint_url(endpoint)
                .credentials_provider(credentials)
                .retry_config(RetryConfig::standard().with_max_attempts(3))
                .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
                .build()
        });
        
        // Create client
        let client = Client::from_conf(aws_config);
        
        let r2_client = Self {
            bucket_name: config.bucket.clone(),
            client,
        };
        
        // Ensure bucket exists
        r2_client.ensure_bucket_exists(&config.bucket)?;
        
        Ok(r2_client)
    }
    
    /// Ensure bucket exists, create it if it doesn't
    fn ensure_bucket_exists(&self, bucket_name: &str) -> Result<()> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to create runtime: {}", e)))?;
            
        runtime.block_on(async {
            // Check if bucket exists
            let bucket_exists = self.client.head_bucket()
                .bucket(bucket_name)
                .send()
                .await
                .is_ok();
                
            // Create bucket if it doesn't exist
            if !bucket_exists {
                println!("Bucket '{}' doesn't exist, creating it...", bucket_name);
                // For R2, no need to specify location constraint
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
                        
                        // For R2, we'll assume the bucket already exists in production and continue
                        // This is because some R2 accounts may not have bucket creation permissions
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
    
    /// Create a new R2 client from the Args struct
    pub fn from_args(args: &Args) -> Result<Self> {
        // Check for R2 specific arguments
        let account_id = args.r2_account_id.as_ref()
            .ok_or_else(|| Box::<dyn std::error::Error>::from("R2 account ID is required"))?;
            
        // Prioritize file-based credentials (new approach)
        if let (Some(access_key_id_filepath), Some(access_key_filepath)) = 
            (&args.r2_access_key_id_filepath, &args.r2_access_key_filepath) {
            
            // Read access key ID from file
            let mut access_key_id = String::new();
            File::open(access_key_id_filepath)
                .map_err(|e| Box::<dyn std::error::Error>::from(
                    format!("Failed to open R2 access key ID file: {}", e)
                ))?
                .read_to_string(&mut access_key_id)
                .map_err(|e| Box::<dyn std::error::Error>::from(
                    format!("Failed to read R2 access key ID file: {}", e)
                ))?;
            
            // Read access key from file
            let mut access_key = String::new();
            File::open(access_key_filepath)
                .map_err(|e| Box::<dyn std::error::Error>::from(
                    format!("Failed to open R2 access key file: {}", e)
                ))?
                .read_to_string(&mut access_key)
                .map_err(|e| Box::<dyn std::error::Error>::from(
                    format!("Failed to read R2 access key file: {}", e)
                ))?;
            
            // Trim whitespace and newlines
            let access_key_id = access_key_id.trim().to_string();
            let access_key = access_key.trim().to_string();
            
            // Get bucket name from file if it's a filepath
            let bucket_name = if let Some(bucket_path) = args.r2_bucket_for_upload.as_ref() {
                // Check if this is a file path
                if Path::new(bucket_path).exists() {
                    // Read bucket name from file
                    let mut bucket_name = String::new();
                    File::open(bucket_path)
                        .map_err(|e| Box::<dyn std::error::Error>::from(
                            format!("Failed to open R2 bucket name file: {}", e)
                        ))?
                        .read_to_string(&mut bucket_name)
                        .map_err(|e| Box::<dyn std::error::Error>::from(
                            format!("Failed to read R2 bucket name file: {}", e)
                        ))?;
                    
                    // Trim whitespace and newlines
                    bucket_name.trim().to_string()
                } else {
                    // Use value directly
                    bucket_path.clone()
                }
            } else {
                return Err(Box::<dyn std::error::Error>::from("R2 bucket name is required"));
            };
            
            let config = R2Config {
                key_id: access_key_id,
                application_key: access_key,
                bucket: bucket_name.clone(),
                account_id: account_id.clone(),
            };
            
            return Self::new(config);
        }
        
        // Fall back to direct parameters (legacy approach)
        if let (Some(access_key_id), Some(access_key)) = 
            (&args.r2_access_key_id, &args.r2_access_key) {
            
            // Get bucket name from file if it's a filepath
            let bucket_name = if let Some(bucket_path) = args.r2_bucket_for_upload.as_ref() {
                // Check if this is a file path
                if Path::new(bucket_path).exists() {
                    // Read bucket name from file
                    let mut bucket_name = String::new();
                    File::open(bucket_path)
                        .map_err(|e| Box::<dyn std::error::Error>::from(
                            format!("Failed to open R2 bucket name file: {}", e)
                        ))?
                        .read_to_string(&mut bucket_name)
                        .map_err(|e| Box::<dyn std::error::Error>::from(
                            format!("Failed to read R2 bucket name file: {}", e)
                        ))?;
                    
                    // Trim whitespace and newlines
                    bucket_name.trim().to_string()
                } else {
                    // Use value directly
                    bucket_path.clone()
                }
            } else {
                return Err(Box::<dyn std::error::Error>::from("R2 bucket name is required"));
            };
            
            let config = R2Config {
                key_id: access_key_id.clone(),
                application_key: access_key.clone(),
                bucket: bucket_name.clone(),
                account_id: account_id.clone(),
            };
            
            return Self::new(config);
        }
        
        // If neither approach worked, return an error
        Err(Box::<dyn std::error::Error>::from(
            "R2 credentials are required (either via files or direct parameters)"
        ))
    }
    
    /// Check if a file exists in R2 storage
    pub fn file_exists(&self, file_key: &str) -> Result<bool> {
        // We need to use tokio runtime for this
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to create runtime: {}", e)))?;
        
        let result = runtime.block_on(async {
            let resp = self.client.head_object()
                .bucket(&self.bucket_name)
                .key(file_key)
                .send()
                .await;
                
            Ok::<bool, Box<dyn std::error::Error>>(resp.is_ok())
        })?;
        
        Ok(result)
    }
    
    /// Get file metadata from R2
    pub fn get_file_metadata(&self, file_key: &str) -> Result<Option<HashMap<String, String>>> {
        // Check if file exists first
        let exists = self.file_exists(file_key)?;
        if !exists {
            return Ok(None);
        }
        
        // We need to use tokio runtime for this
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to create runtime: {}", e)))?;
        
        let metadata = runtime.block_on(async {
            let resp = self.client.head_object()
                .bucket(&self.bucket_name)
                .key(file_key)
                .send()
                .await
                .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to get R2 file metadata: {}", e)))?;
            
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
    
    /// Upload a file to R2 storage
    pub fn upload_file(&self, local_path: &str, object_key: &str, metadata: Option<HashMap<String, String>>) -> Result<R2UploadResult> {
        // Check if the file exists locally
        if !Path::new(local_path).exists() {
            return Err(Box::<dyn std::error::Error>::from(
                format!("Local file does not exist: {}", local_path)
            ));
        }
        
        // We need to use tokio runtime for this
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to create runtime: {}", e)))?;
        
        let result = runtime.block_on(async {
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
                .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to upload to R2: {}", e)))?;
            
            // Get the ETag (hash) from the response
            let etag = response.e_tag()
                .ok_or_else(|| Box::<dyn std::error::Error>::from("No ETag in response"))?
                .replace("\"", ""); // Remove quotes from ETag
            
            Ok::<R2UploadResult, Box<dyn std::error::Error>>(R2UploadResult {
                hash: etag.clone(),
                id: etag.clone(),
                bucket_id: self.bucket_name.clone(),
                name: object_key.to_string(),
            })
        })?;
        
        Ok(result)
    }
    
    /// Upload a file with details from FileDetails struct
    pub fn upload_file_with_details(&self, file_details: &FileDetails) -> Result<R2UploadResult> {
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
        
        // Determine the path in R2 based on bucket details
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
    
    /// Download a file from R2 storage
    pub fn download_file(&self, object_key: &str) -> Result<Vec<u8>> {
        // We need to use tokio runtime for this
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to create runtime: {}", e)))?;
        
        let content = runtime.block_on(async {
            let resp = self.client.get_object()
                .bucket(&self.bucket_name)
                .key(object_key)
                .send()
                .await
                .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to download from R2: {}", e)))?;
            
            // Get content as bytes
            let bytes = resp.body.collect().await
                .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to read response body: {}", e)))?;
                
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
}