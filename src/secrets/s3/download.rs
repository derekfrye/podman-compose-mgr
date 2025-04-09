use crate::secrets::error::Result;
use crate::secrets::s3::models::S3StorageClient;
use aws_sdk_s3::error::ProvideErrorMetadata;
use std::path::Path;

impl S3StorageClient {
    /// Download a file from storage
    pub fn download_file(&self, object_key: &str) -> Result<Vec<u8>> {
        // For non-real clients, return a mock response
        if !self.is_real_client {
            crate::utils::log_utils::debug(
                &format!(
                    "Using mock S3-compatible storage client - would have downloaded {}",
                    object_key
                ),
                self.verbose as u8,
            );
            // Return a small mock content
            return Ok(b"mock content for testing".to_vec());
        }

        // Use the client's runtime instead of creating a new one
        let content = self.runtime.block_on(async {
            // If verbose, log details about the request
            crate::utils::log_utils::debug(
                &format!("S3 download request details:\nBucket: {}\nObject key: {}", self.bucket_name, object_key),
                self.verbose as u8
            );

            // Create the request - easier to debug when we split it out
            let request = self
                .client
                .get_object()
                .bucket(&self.bucket_name)
                .key(object_key);

            // If verbose, log the full request
            crate::utils::log_utils::debug(
                &format!("S3 request: {:?}", request),
                self.verbose as u8
            );

            // Send the request
            let resp = request.send().await.map_err(|e| {
                // Enhanced error information
                crate::utils::log_utils::debug(
                    &format!("S3 download error details:\nError: {:?}\nError SDK source: {}\nMessage: {}\nRaw: {:?}", 
                        e, e.code().unwrap_or("unknown"), e, e),
                    self.verbose as u8
                );

                Box::<dyn std::error::Error>::from(format!(
                    "Failed to download from storage: {}",
                    e
                ))
            })?;

            // If verbose, log response details
            crate::utils::log_utils::debug(
                &format!("S3 download response received successfully\nContent length: {:?}\nE-Tag: {:?}", 
                    resp.content_length(), resp.e_tag()),
                self.verbose as u8
            );

            // Get content as bytes
            let bytes = resp.body.collect().await.map_err(|e| {
                Box::<dyn std::error::Error>::from(format!("Failed to read response body: {}", e))
            })?;

            // Convert to Vec<u8> once
            let content_bytes = bytes.to_vec();

            // If verbose, log successful download
            crate::utils::log_utils::debug(
                &format!("Successfully downloaded {} bytes from S3", content_bytes.len()),
                self.verbose as u8
            );

            Ok::<Vec<u8>, Box<dyn std::error::Error>>(content_bytes)
        })?;

        Ok(content)
    }

    /// Save downloaded content to a file
    pub fn save_to_file(&self, object_key: &str, local_path: &str) -> Result<()> {
        // For non-real clients, create a mock file
        if !self.is_real_client {
            crate::utils::log_utils::debug(
                &format!(
                    "Using mock S3-compatible storage client - would have downloaded {} to {}",
                    object_key, local_path
                ),
                self.verbose as u8,
            );
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
}