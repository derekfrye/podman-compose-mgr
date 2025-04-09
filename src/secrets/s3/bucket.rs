use crate::secrets::error::Result;
use crate::secrets::s3::models::S3StorageClient;
use std::path::Path;
use std::fs::File;
use std::io::Read;

impl S3StorageClient {
    /// Ensure bucket exists, create it if it doesn't
    pub fn ensure_bucket_exists(&self, bucket_name: &str) -> Result<()> {
        // Use the client's runtime instead of creating a new one
        self.runtime.block_on(async {
            // Check if bucket exists
            let bucket_exists = self
                .client
                .head_bucket()
                .bucket(bucket_name)
                .send()
                .await
                .is_ok();

            // Create bucket if it doesn't exist
            if !bucket_exists {
                println!("Bucket '{}' doesn't exist, creating it...", bucket_name);

                // Create bucket without location constraint (works for both B2 and R2)
                let result = self.client.create_bucket().bucket(bucket_name).send().await;

                match result {
                    Ok(_) => {
                        if self.verbose >= 2 {
                            crate::utils::log_utils::debug(
                                &format!("Successfully created bucket '{}'", bucket_name),
                                self.verbose as u8,
                            );
                        }
                    }
                    Err(e) => {
                        // If error is because bucket already exists, that's fine
                        let err_str = format!("{}", e);
                        println!("Bucket creation error details: {}", err_str);

                        if err_str.contains("BucketAlreadyExists")
                            || err_str.contains("BucketAlreadyOwnedByYou")
                            || err_str.contains("already exists")
                        {
                            if self.verbose >= 2 {
                                crate::utils::log_utils::debug(
                                    &format!(
                                        "Bucket '{}' already exists, proceeding...",
                                        bucket_name
                                    ),
                                    self.verbose as u8,
                                );
                            }
                            return Ok(());
                        }

                        // For S3 providers, we'll assume the bucket already exists in production and continue
                        // This is because some accounts may not have bucket creation permissions
                        // but still have upload permissions to existing buckets
                        eprintln!(
                            "warn: Failed to create bucket '{}', but will attempt to use it anyway",
                            bucket_name
                        );

                        // Instead of failing, we'll try to continue
                        // If the bucket truly doesn't exist, operations will fail later naturally
                        return Ok(());
                    }
                }
            }

            Ok(())
        })
    }
}

/// List objects with a given prefix
impl S3StorageClient {
    pub fn list_objects_with_prefix(&self, prefix: &str) -> Result<Vec<String>> {
        // For non-real clients, return a mock response
        if !self.is_real_client {
            crate::utils::log_utils::debug(
                &format!(
                    "Using mock S3-compatible storage client - would have listed objects with prefix {}",
                    prefix
                ),
                self.verbose as u8,
            );
            // Return some mock object keys
            return Ok(vec![
                format!("{}{}", prefix, "mock_file1.txt"),
                format!("{}{}", prefix, "mock_file2.txt"),
            ]);
        }

        // Use the client's runtime instead of creating a new one
        let objects = self.runtime.block_on(async {
            let mut result = Vec::new();

            // Log the request details if verbose
            crate::utils::log_utils::debug(
                &format!(
                    "Listing objects with prefix '{}' in bucket '{}'",
                    prefix, self.bucket_name
                ),
                self.verbose as u8,
            );

            // Create the list objects request
            let list_request = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket_name)
                .prefix(prefix)
                .max_keys(100); // Limit to 100 objects

            // Send the request
            match list_request.send().await {
                Ok(response) => {
                    // Extract the object keys from the response
                    if let Some(contents) = response.contents {
                        for object in contents {
                            if let Some(key) = object.key {
                                result.push(key.to_string());
                            }
                        }
                    }

                    // Log how many objects we found
                    crate::utils::log_utils::debug(
                        &format!("Found {} objects with prefix '{}'", result.len(), prefix),
                        self.verbose as u8,
                    );

                    Ok::<Vec<String>, Box<dyn std::error::Error>>(result)
                }
                Err(e) => {
                    // Enhanced error information
                    crate::utils::log_utils::debug(
                        &format!("Error listing objects with prefix '{}': {}", prefix, e),
                        self.verbose as u8,
                    );

                    Err(Box::<dyn std::error::Error>::from(format!(
                        "Failed to list objects with prefix '{}': {}",
                        prefix, e
                    )))
                }
            }
        })?;

        Ok(objects)
    }
}

/// Helper function to read a value from a file
pub fn read_value_from_file(file_path: &Path) -> Result<String> {
    let mut value = String::new();
    File::open(file_path)
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to open file: {}", e)))?
        .read_to_string(&mut value)
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to read file: {}", e)))?;

    // Trim whitespace and newlines
    Ok(value.trim().to_string())
}

/// Helper function to get a bucket name, either from a direct value or from a file
pub fn get_bucket_name(bucket_path: &str) -> Result<String> {
    // Check if this is a file path
    if Path::new(bucket_path).exists() {
        // Read bucket name from file
        read_value_from_file(Path::new(bucket_path))
    } else {
        // Use value directly
        Ok(bucket_path.to_string())
    }
}