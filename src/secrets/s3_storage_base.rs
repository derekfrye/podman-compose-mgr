// This file is kept for backward compatibility
// New code should use the modules in the s3/ directory

// Re-export all types and functions from the new s3 module
pub use crate::secrets::s3::models::{S3Config, S3Provider, S3StorageClient, S3UploadResult};
pub use crate::secrets::s3::bucket::{get_bucket_name, read_value_from_file};