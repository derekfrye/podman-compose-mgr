use crate::secrets::error::Result;
use crate::secrets::s3::models::S3StorageClient;
use std::collections::HashMap;

impl S3StorageClient {
    /// Check if a file exists in storage
    pub fn file_exists(&self, file_key: &str) -> Result<bool> {
        // For non-real clients, return false to indicate file doesn't exist
        if !self.is_real_client {
            crate::utils::log_utils::debug(
                &format!(
                    "Using mock S3-compatible storage client - would have checked if {} exists",
                    file_key
                ),
                self.verbose as u8,
            );
            return Ok(false);
        }

        // Use the client's runtime instead of creating a new one
        let result = self.runtime.block_on(async {
            let resp = self
                .client
                .head_object()
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
        // For non-real clients, return None to indicate file doesn't exist
        if !self.is_real_client {
            crate::utils::log_utils::debug(
                &format!(
                    "Using mock S3-compatible storage client - would have retrieved metadata for {}",
                    file_key
                ),
                self.verbose as u8,
            );
            return Ok(None);
        }

        // Check if file exists first
        let exists = self.file_exists(file_key)?;
        if !exists {
            return Ok(None);
        }

        // Use the client's runtime instead of creating a new one
        let metadata = self.runtime.block_on(async {
            let resp = self
                .client
                .head_object()
                .bucket(&self.bucket_name)
                .key(file_key)
                .send()
                .await
                .map_err(|e| {
                    Box::<dyn std::error::Error>::from(format!(
                        "Failed to get file metadata: {}",
                        e
                    ))
                })?;

            let mut result = HashMap::new();

            // Get standard metadata
            if let Some(content_type) = resp.content_type() {
                result.insert("content_type".to_string(), content_type.to_string());
            }

            // For debugging, log the whole response if verbose >= 2
            crate::utils::log_utils::debug(
                &format!("R2/S3 Head Object response: {:?}", resp),
                self.verbose as u8,
            );

            // Try multiple approaches to get content length
            // 1. Standard content_length method
            let mut got_content_length = false;
            if let Some(content_length) = resp.content_length() {
                result.insert("content_length".to_string(), content_length.to_string());
                got_content_length = true;

                // Log content length when verbose is enabled
                crate::utils::log_utils::debug(
                    &format!("Received content_length from R2/S3: {}", content_length),
                    self.verbose as u8,
                );
            }

            // 2. Try to get size from user metadata - try several possible metadata field names
            if !got_content_length {
                if let Some(metadata) = resp.metadata() {
                    // Try different metadata keys that might contain the size
                    for size_key in ["size", "content-length", "file-size", "filesize"] {
                        if let Some(size) = metadata.get(size_key) {
                            result.insert("content_length".to_string(), size.to_string());
                            got_content_length = true;

                            crate::utils::log_utils::debug(
                                &format!(
                                    "Received size from R2/S3 metadata field '{}': {}",
                                    size_key, size
                                ),
                                self.verbose as u8,
                            );
                            break;
                        }
                    }

                    // If verbose, log all metadata for debugging
                    if !got_content_length {
                        crate::utils::log_utils::debug(
                            &format!(
                                "Available metadata keys: {:?}",
                                metadata.keys().collect::<Vec<_>>()
                            ),
                            self.verbose as u8,
                        );
                    }
                }
            }

            // Log if we couldn't get content length
            if !got_content_length {
                crate::utils::log_utils::debug(
                    "No content_length or size received from R2/S3 in metadata response",
                    self.verbose as u8,
                );
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

    /// Check if a file exists and get its metadata for specific path
    pub fn check_file_exists_with_details(
        &self,
        hash: &str,
        bucket_name: Option<&str>,
        prefix: Option<&str>,
    ) -> Result<Option<(bool, String, String)>> {
        // Construct the object key based on whether a prefix is provided
        let object_key = if let Some(prefix_path) = prefix {
            format!("{}/{}", prefix_path, hash)
        } else {
            // No prefix, use hash directly as the key
            hash.to_string()
        };

        // For non-real clients, return mock data
        if !self.is_real_client {
            crate::utils::log_utils::debug(
                &format!(
                    "Using mock S3-compatible storage client - would have checked if file with hash {} exists",
                    hash
                ),
                self.verbose as u8,
            );
            // Return mock data for testing: (exists, created_time, updated_time)
            return Ok(Some((
                true,
                "2025-01-01T00:00:00Z".to_string(),
                "2025-01-01T00:00:00Z".to_string(),
            )));
        }

        // Determine which bucket to use
        let bucket_to_use = if let Some(b) = bucket_name {
            b
        } else {
            // Check if we need to use a placeholder bucket
            if self.bucket_name == "placeholder_bucket_will_be_provided_from_json" {
                // We can't check for existence if the bucket name is a placeholder
                crate::utils::log_utils::debug(
                    "Can't check if file exists because bucket name is a placeholder. Please provide bucket name in JSON.",
                    self.verbose as u8,
                );
                return Ok(None);
            }
            &self.bucket_name
        };

        crate::utils::log_utils::debug(
            &format!(
                "Checking if object '{}' exists in bucket '{}'",
                object_key, bucket_to_use
            ),
            self.verbose as u8,
        );

        // Use the client's runtime to check if the object exists
        let result =
            self.runtime.block_on(async {
                let resp = self
                    .client
                    .head_object()
                    .bucket(bucket_to_use)
                    .key(&object_key)
                    .send()
                    .await;

                if resp.is_err() {
                    crate::utils::log_utils::debug(
                        &format!("Object does not exist or error occurred: {:?}", resp.err()),
                        self.verbose as u8,
                    );
                    return Ok::<Option<(bool, String, String)>, Box<dyn std::error::Error>>(Some(
                        (false, "".to_string(), "".to_string()),
                    ));
                }

                let response = resp.unwrap();

                // Get last_modified as a string
                let last_modified = if let Some(lm) = response.last_modified() {
                    // Use standard DateTime formatting for AWS DateTime
                    format!("{}", lm)
                } else {
                    "Unknown".to_string()
                };

                crate::utils::log_utils::debug(
                    &format!("Object exists with last modified time: {}", last_modified),
                    self.verbose as u8,
                );

                // For S3 compatible services, we don't have created time, so use last_modified
                Ok(Some((true, last_modified.clone(), last_modified)))
            })?;

        Ok(result)
    }
}
