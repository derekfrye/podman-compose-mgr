use crate::secrets::error::Result;
use crate::secrets::file_details::FileDetails;
use crate::secrets::s3::models::{S3StorageClient, S3UploadResult};
use aws_sdk_s3::primitives::ByteStream;
use std::collections::HashMap;
use std::path::Path;

impl S3StorageClient {
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

            crate::utils::log_utils::debug(
                &format!(
                    "Using mock S3-compatible storage client - would have uploaded {} to {}",
                    local_path, object_key
                ),
                self.verbose as u8,
            );

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
        if (matches!(
            self.provider_type,
            crate::secrets::s3::models::S3Provider::BackblazeB2
        ) || matches!(
            self.provider_type,
            crate::secrets::s3::models::S3Provider::CloudflareR2
        )) && file_details.cloud_upload_bucket.is_none()
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

            crate::utils::log_utils::debug(
                &format!(
                    "Using mock S3-compatible storage client - would have uploaded {} using hash {}",
                    file_details.file_path, file_details.hash
                ),
                self.verbose as u8,
            );

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

        // For R2/B2, always use the original file directly (no base64 encoding)
        // This is because:
        // 1. R2/B2 can natively store binary files without encoding
        // 2. Base64 encoding increases the file size by ~33%
        // 3. It's more efficient to upload the raw file
        let file_to_use = file_details.file_path.clone();

        // Create metadata for the file
        let mut metadata = HashMap::new();
        metadata.insert("original-path".to_string(), file_details.file_path.clone());
        metadata.insert("hash".to_string(), file_details.hash.clone());
        metadata.insert("hash-algo".to_string(), file_details.hash_algo.clone());
        // Store original encoding in metadata for reference, but treat file as binary (no encoding needed for S3)
        metadata.insert("encoding".to_string(), "binary".to_string()); 
        metadata.insert("original-encoding".to_string(), file_details.encoding.clone());
        metadata.insert("size".to_string(), file_details.file_size.to_string());
        // Add both for compatibility with different storage providers
        metadata.insert(
            "content-length".to_string(),
            file_details.file_size.to_string(),
        );

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
            crate::utils::log_utils::debug(
                &format!(
                    "Using real bucket name '{}' from JSON for upload",
                    upload_bucket
                ),
                self.verbose as u8,
            );
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
}
