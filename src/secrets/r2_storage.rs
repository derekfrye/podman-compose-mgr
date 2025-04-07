use crate::args::Args;
use crate::secrets::error::Result;
use crate::secrets::file_details::FileDetails;
use crate::secrets::s3_storage_base::{S3StorageClient, S3Config, S3Provider, read_value_from_file};
use std::collections::HashMap;

/// Configuration for the R2 client (for backwards compatibility)
pub struct R2Config {
    pub key_id: String,
    pub application_key: String,
    pub bucket: String,
    pub account_id: String,
}

/// Represents the result of an R2 upload operation (for backwards compatibility)
pub struct R2UploadResult {
    pub hash: String,
    pub id: String,
    pub bucket_id: String,
    pub name: String,
    pub created: String,
    pub updated: String,
}

/// R2 client wrapper around the base S3StorageClient
pub struct R2Client {
    client: S3StorageClient,
}

impl R2Client {
    /// Create a new R2 client from the provided config
    pub fn new(config: R2Config) -> Result<Self> {
        // Convert R2Config to S3Config
        let s3_config = S3Config {
            key_id: config.key_id,
            application_key: config.application_key,
            bucket: config.bucket,
            provider: S3Provider::CloudflareR2,
            account_id: Some(config.account_id), // Required for R2
        };
        
        // Create the base S3 client - default to non-verbose
        let client = S3StorageClient::new(s3_config, false)?;
        
        Ok(Self { client })
    }
    
    /// Create a new R2 client from the Args struct
    pub fn from_args(args: &Args) -> Result<Self> {
        // Use the unified S3 parameters
        if let (Some(access_key_id_filepath), Some(secret_key_filepath), Some(endpoint_filepath)) = 
            (&args.s3_account_id_filepath, &args.s3_secret_key_filepath, &args.s3_endpoint_filepath) {
            
            // Read access key ID from file
            let access_key_id = read_value_from_file(access_key_id_filepath)?;
            
            // Read secret key from file
            let access_key = read_value_from_file(secret_key_filepath)?;
            
            // Read Cloudflare account ID (endpoint) from file
            let account_id = read_value_from_file(endpoint_filepath)?;
            
            // R2 bucket is now required to be in the input JSON
            // Use a placeholder here that will be replaced during upload
            let bucket_name = "placeholder_bucket_will_be_provided_from_json".to_string();
            
            let config = R2Config {
                key_id: access_key_id,
                application_key: access_key,
                bucket: bucket_name,
                account_id,
            };
            
            // Create a client and then update the verbose flag
            let mut client = Self::new(config)?;
            // Set verbose flag based on args
            client.client.verbose = args.verbose > 0;
            return Ok(client);
        }
        
        // If parameters are missing, return an error
        Err(Box::<dyn std::error::Error>::from(
            "S3-compatible credentials are required for R2 storage (s3_account_id_filepath, s3_secret_key_filepath, and s3_endpoint_filepath)"
        ))
    }
    
    /// Check if a file exists in R2 storage - delegated to S3 client
    pub fn file_exists(&self, file_key: &str) -> Result<bool> {
        self.client.file_exists(file_key)
    }
    
    /// Check if a file exists in R2 storage with detailed information
    pub fn check_file_exists_with_details(&self, hash: &str, bucket_name: Option<&str>) -> Result<Option<(bool, String, String)>> {
        self.client.check_file_exists_with_details(hash, bucket_name, None)
    }
    
    /// Get file metadata from R2 - delegated to S3 client
    pub fn get_file_metadata(&self, file_key: &str) -> Result<Option<HashMap<String, String>>> {
        self.client.get_file_metadata(file_key)
    }
    
    /// Upload a file to R2 storage - delegated to S3 client
    pub fn upload_file(&self, local_path: &str, object_key: &str, metadata: Option<HashMap<String, String>>) -> Result<R2UploadResult> {
        // Use the underlying S3 client to upload
        let s3_result = self.client.upload_file(local_path, object_key, metadata)?;
        
        // Convert to R2UploadResult for backwards compatibility
        Ok(R2UploadResult {
            hash: s3_result.hash,
            id: s3_result.id,
            bucket_id: s3_result.bucket_id,
            name: s3_result.name,
            created: s3_result.created,
            updated: s3_result.updated,
        })
    }
    
    /// Upload a file with details from FileDetails struct - delegated to S3 client
    pub fn upload_file_with_details(&self, file_details: &FileDetails) -> Result<R2UploadResult> {
        // Use the underlying S3 client to upload
        let s3_result = self.client.upload_file_with_details(file_details)?;
        
        // Convert to R2UploadResult for backwards compatibility
        Ok(R2UploadResult {
            hash: s3_result.hash,
            id: s3_result.id,
            bucket_id: s3_result.bucket_id,
            name: s3_result.name,
            created: s3_result.created,
            updated: s3_result.updated,
        })
    }
    
    /// Download a file from R2 storage - delegated to S3 client
    pub fn download_file(&self, object_key: &str) -> Result<Vec<u8>> {
        self.client.download_file(object_key)
    }
    
    /// Save downloaded content to a file - delegated to S3 client
    pub fn save_to_file(&self, object_key: &str, local_path: &str) -> Result<()> {
        self.client.save_to_file(object_key, local_path)
    }
}