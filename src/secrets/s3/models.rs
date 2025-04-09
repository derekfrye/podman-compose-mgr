use aws_sdk_s3::Client;

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
    pub(crate) bucket_name: String,
    pub(crate) client: Client,
    pub(crate) runtime: tokio::runtime::Runtime,
    pub(crate) provider_type: S3Provider,
    pub(crate) is_real_client: bool,
    pub verbose: u32, // Using u32 for multiple verbosity levels
}