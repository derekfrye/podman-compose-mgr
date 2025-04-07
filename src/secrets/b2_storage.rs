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
        
        // Create the base S3 client - default to non-verbose
        let client = S3StorageClient::new(s3_config, false)?;
        
        Ok(Self { client })
    }
    
    /// Create a new B2 client from the Args struct
    pub fn from_args(args: &Args) -> Result<Self> {
        // Use the unified S3 parameters
        if let (Some(account_id_filepath), Some(secret_key_filepath)) = 
            (&args.s3_account_id_filepath, &args.s3_secret_key_filepath) {
            
            // Read account ID and key from files
            let account_id = read_value_from_file(account_id_filepath)?;
            let account_key = read_value_from_file(secret_key_filepath)?;
            
            // B2 bucket is now required to be in the input JSON
            // Use a placeholder here that will be replaced during upload
            let bucket_name = "placeholder_bucket_will_be_provided_from_json".to_string();
            
            let config = B2Config {
                key_id: account_id,
                application_key: account_key,
                bucket: bucket_name,
            };
            
            // Create a client and then update the verbose flag
            let mut client = Self::new(config)?;
            // Set verbose flag based on args
            client.client.verbose = args.verbose > 0;
            return Ok(client);
        }
        
        // If parameters are missing, return an error
        Err(Box::<dyn std::error::Error>::from(
            "S3-compatible credentials are required for B2 storage"
        ))
    }
    
    /// Check if a file exists in B2 storage - delegated to S3 client
    pub fn file_exists(&self, file_key: &str) -> Result<bool> {
        self.client.file_exists(file_key)
    }
    
    /// Check if a file exists in B2 storage with detailed information
    pub fn check_file_exists_with_details(&self, hash: &str, bucket_name: Option<&str>) -> Result<Option<(bool, String, String)>> {
        self.client.check_file_exists_with_details(hash, bucket_name, None)
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
            created: s3_result.created,
            updated: s3_result.updated,
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
            created: s3_result.created,
            updated: s3_result.updated,
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
    pub created: String,
    pub updated: String,
}