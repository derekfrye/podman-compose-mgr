use crate::args::Args;
use crate::secrets::error::Result;
use crate::secrets::file_details::FileDetails;
use crate::secrets::s3_storage_base::{S3StorageClient, S3Config, S3Provider, read_value_from_file};

/// Configuration for the B2 client (for backwards compatibility)
pub struct B2Config {
    pub key_id: String,
    pub application_key: String,
    pub bucket: String,
}

/// B2 client wrapper around the base S3StorageClient
pub struct B2Client {
    client: S3StorageClient,
}

impl B2Client {
    /// Create a new B2 client from the provided config
    pub fn new(config: B2Config) -> Result<Self> {
        // Convert B2Config to S3Config
        let s3_config = S3Config {
            key_id: config.key_id,
            application_key: config.application_key,
            bucket: config.bucket,
            provider: S3Provider::BackblazeB2,
            account_id: None, // Not needed for B2
        };
        
        // Create the base S3 client
        let client = S3StorageClient::new(s3_config)?;
        
        Ok(Self { client })
    }
    
    /// Create a new B2 client from the Args struct
    pub fn from_args(args: &Args) -> Result<Self> {
        // Prioritize file-based credentials (new approach)
        if let (Some(account_id_filepath), Some(account_key_filepath)) = 
            (&args.b2_account_id_filepath, &args.b2_account_key_filepath) {
            
            // Read account ID and key from files
            let account_id = read_value_from_file(account_id_filepath)?;
            let account_key = read_value_from_file(account_key_filepath)?;
            
            // B2 bucket is now required to be in the input JSON
            // Use a placeholder here that will be replaced during upload
            let bucket_name = "placeholder_bucket_will_be_provided_from_json".to_string();
            
            let config = B2Config {
                key_id: account_id,
                application_key: account_key,
                bucket: bucket_name,
            };
            
            return Self::new(config);
        }
        
        // Fall back to direct parameters (legacy approach)
        if let (Some(key_id), Some(application_key)) = 
            (&args.b2_key_id, &args.b2_application_key) {
            
            // B2 bucket is now required to be in the input JSON
            // Use a placeholder here that will be replaced during upload
            let bucket_name = "placeholder_bucket_will_be_provided_from_json".to_string();
            
            let config = B2Config {
                key_id: key_id.clone(),
                application_key: application_key.clone(),
                bucket: bucket_name,
            };
            
            return Self::new(config);
        }
        
        // If neither approach worked, return an error
        Err(Box::<dyn std::error::Error>::from(
            "Either B2 account ID and key files or direct credentials are required"
        ))
    }
    
    /// Check if a file exists in B2 storage - delegated to S3 client
    pub fn file_exists(&self, file_key: &str) -> Result<bool> {
        self.client.file_exists(file_key)
    }
    
    /// Get file metadata from B2 - delegated to S3 client
    pub fn get_file_metadata(&self, file_key: &str) -> Result<Option<std::collections::HashMap<String, String>>> {
        self.client.get_file_metadata(file_key)
    }
    
    /// Upload a file to B2 storage - delegated to S3 client
    pub fn upload_file(&self, local_path: &str, object_key: &str, metadata: Option<std::collections::HashMap<String, String>>) -> Result<B2UploadResult> {
        // Use the underlying S3 client to upload
        let s3_result = self.client.upload_file(local_path, object_key, metadata)?;
        
        // Convert to B2UploadResult for backwards compatibility
        Ok(B2UploadResult {
            hash: s3_result.hash,
            id: s3_result.id,
            bucket_id: s3_result.bucket_id,
            name: s3_result.name,
        })
    }
    
    /// Upload a file with details from FileDetails struct - delegated to S3 client
    pub fn upload_file_with_details(&self, file_details: &FileDetails) -> Result<B2UploadResult> {
        // Use the underlying S3 client to upload
        let s3_result = self.client.upload_file_with_details(file_details)?;
        
        // Convert to B2UploadResult for backwards compatibility
        Ok(B2UploadResult {
            hash: s3_result.hash,
            id: s3_result.id,
            bucket_id: s3_result.bucket_id,
            name: s3_result.name,
        })
    }
    
    /// Download a file from B2 storage - delegated to S3 client
    pub fn download_file(&self, object_key: &str) -> Result<Vec<u8>> {
        self.client.download_file(object_key)
    }
    
    /// Save downloaded content to a file - delegated to S3 client
    pub fn save_to_file(&self, object_key: &str, local_path: &str) -> Result<()> {
        self.client.save_to_file(object_key, local_path)
    }
}

/// Represents the result of a B2 upload operation (for backwards compatibility)
pub struct B2UploadResult {
    pub hash: String,
    pub id: String,
    pub bucket_id: String,
    pub name: String,
}