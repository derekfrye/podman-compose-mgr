use opendal::services::B2;
use opendal::{Operator, Scheme};
use std::fs::File;
use std::io::{BufReader, Read};
use async_std::task;
use crate::secrets::error;

pub struct B2Config {
    pub key_id: String,
    pub application_key: String,
    pub bucket: String,
}

pub struct B2Client {
    operator: Operator,
}

impl B2Client {
    pub fn new(config: B2Config) -> error::Result<Self> {
        // Create Backblaze B2 service builder
        let mut builder = B2::default();
        builder.key_id(&config.key_id);
        builder.application_key(&config.application_key);
        builder.bucket(&config.bucket);
        
        // Create operator
        let operator = Operator::new(builder)
            .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to create B2 client: {}", e)))?
            .finish();
        
        Ok(Self { operator })
    }
    
    pub fn file_exists(&self, key: &str) -> error::Result<bool> {
        // Run the async check in a blocking context
        task::block_on(async {
            match self.operator.stat(key).await {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            }
        })
    }
    
    pub fn get_file_metadata(&self, key: &str) -> error::Result<Option<(String, String)>> {
        task::block_on(async {
            match self.operator.stat(key).await {
                Ok(metadata) => {
                    let created = metadata.created_at()
                        .map(|t| t.to_string())
                        .unwrap_or_default();
                    let updated = metadata.last_modified()
                        .map(|t| t.to_string())
                        .unwrap_or_default();
                    
                    Ok(Some((created, updated)))
                },
                Err(_) => Ok(None),
            }
        })
    }
    
    pub fn upload_file(&self, key: &str, filepath: &str, content_type: Option<&str>) -> error::Result<String> {
        task::block_on(async {
            // For small files, read directly
            let content = std::fs::read(filepath)
                .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to read file {}: {}", filepath, e)))?;
            
            // Set content type if provided
            let mut writer = self.operator.writer(key);
            if let Some(ct) = content_type {
                writer = writer.with_content_type(ct);
            }
            
            // Write content to B2
            writer.write(content).await
                .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to upload file to B2: {}", e)))?;
            
            // Get the URL or ID
            let metadata = self.operator.stat(key).await
                .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to get metadata after upload: {}", e)))?;
            
            let id = metadata.content_md5()
                .unwrap_or_else(|| key.to_string());
            
            Ok(id)
        })
    }
    
    pub fn upload_large_file(&self, key: &str, filepath: &str, content_type: Option<&str>) -> error::Result<String> {
        task::block_on(async {
            // Open the file
            let file = File::open(filepath)
                .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to open file {}: {}", filepath, e)))?;
            
            let file_size = file.metadata()
                .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to get file metadata: {}", e)))?
                .len();
            
            // Stream in chunks
            let mut reader = BufReader::new(file);
            
            // Set content type if provided
            let mut writer = self.operator.writer(key);
            if let Some(ct) = content_type {
                writer = writer.with_content_type(ct);
            }
            
            let mut buffer = vec![0; 8192]; // 8KB buffer
            let mut total_bytes = 0;
            
            loop {
                let bytes_read = reader.read(&mut buffer)
                    .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to read from file: {}", e)))?;
                
                if bytes_read == 0 {
                    break;
                }
                
                writer.write(&buffer[..bytes_read]).await
                    .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to write chunk to B2: {}", e)))?;
                
                total_bytes += bytes_read;
            }
            
            // Check if we uploaded all bytes
            if total_bytes != file_size {
                return Err(Box::<dyn std::error::Error>::from(
                    format!("Uploaded {} bytes but file size is {} bytes", total_bytes, file_size)
                ));
            }
            
            // Get the URL or ID
            let metadata = self.operator.stat(key).await
                .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to get metadata after upload: {}", e)))?;
            
            let id = metadata.content_md5()
                .unwrap_or_else(|| key.to_string());
            
            Ok(id)
        })
    }
    
    pub fn download_file(&self, key: &str) -> error::Result<Vec<u8>> {
        task::block_on(async {
            // Get the content
            let content = self.operator.read(key).await
                .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to download file from B2: {}", e)))?;
            
            Ok(content)
        })
    }
}