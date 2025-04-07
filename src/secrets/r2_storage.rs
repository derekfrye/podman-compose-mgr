use crate::args::Args;
use crate::secrets::error::Result;
use crate::secrets::file_details::FileDetails;
use crate::secrets::s3_storage_base::{S3StorageClient, S3Config, S3Provider, read_value_from_file, get_bucket_name};
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
        
        // Create the base S3 client
        let client = S3StorageClient::new(s3_config)?;
        
        Ok(Self { client })
    }
    
    /// Create a new R2 client from the Args struct
    pub fn from_args(args: &Args) -> Result<Self> {
        // Get Cloudflare Account ID - prioritize file-based if provided
        let account_id = if let Some(account_id_filepath) = &args.r2_account_id_filepath {
            read_value_from_file(account_id_filepath)?
        } else if let Some(account_id) = &args.r2_account_id {
            account_id.clone()
        } else {
            return Err(Box::<dyn std::error::Error>::from("R2 account ID is required"));
        };
            
        // Prioritize file-based credentials (new approach)
        if let (Some(access_key_id_filepath), Some(access_key_filepath)) = 
            (&args.r2_access_key_id_filepath, &args.r2_access_key_filepath) {
            
            // Read access keys from files
            let access_key_id = read_value_from_file(access_key_id_filepath)?;
            let access_key = read_value_from_file(access_key_filepath)?;
            
            // Get bucket name from args
            let bucket_name = match args.r2_bucket_for_upload.as_ref() {
                Some(bucket_path) => get_bucket_name(bucket_path)?,
                None => return Err(Box::<dyn std::error::Error>::from("R2 bucket name is required")),
            };
            
            let config = R2Config {
                key_id: access_key_id,
                application_key: access_key,
                bucket: bucket_name,
                account_id: account_id,
            };
            
            return Self::new(config);
        }
        
        // Fall back to direct parameters (legacy approach)
        if let (Some(access_key_id), Some(access_key)) = 
            (&args.r2_access_key_id, &args.r2_access_key) {
            
            // Get bucket name from args
            let bucket_name = match args.r2_bucket_for_upload.as_ref() {
                Some(bucket_path) => get_bucket_name(bucket_path)?,
                None => return Err(Box::<dyn std::error::Error>::from("R2 bucket name is required")),
            };
            
            let config = R2Config {
                key_id: access_key_id.clone(),
                application_key: access_key.clone(),
                bucket: bucket_name,
                account_id: account_id,
            };
            
            return Self::new(config);
        }
        
        // If neither approach worked, return an error
        Err(Box::<dyn std::error::Error>::from(
            "R2 credentials are required (either via files or direct parameters)"
        ))
    }
    
    /// Check if a file exists in R2 storage - delegated to S3 client
    pub fn file_exists(&self, file_key: &str) -> Result<bool> {
        self.client.file_exists(file_key)
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