use crate::secrets::error::Result;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use chrono::{DateTime, Local};
use std::fs::{File, metadata};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

/// Represents the detailed information about a file for secret upload
#[derive(Debug, Clone)]
pub struct FileDetails {
    pub file_path: String,
    pub file_size: u64,    // Original file size
    pub encoded_size: u64, // Size after encoding (if base64)
    pub last_modified: String,
    pub secret_name: String,
    pub encoding: String, // "utf8" or "base64"
    pub cloud_created: Option<String>,
    pub cloud_updated: Option<String>,
    pub cloud_type: Option<String>, // "azure_kv" or "b2"
    pub hash: String,
    pub hash_algo: String,
    pub cloud_upload_bucket: Option<String>, // Bucket name for B2 storage
}

/// Check if file is UTF-8 encoded and return encoding info
/// For non-UTF-8 files, creates a .base64 encoded version if it doesn't exist
pub fn check_encoding_and_size(filepath: &str) -> Result<(String, u64, u64)> {
    // Get original file size
    let file = File::open(filepath).map_err(|e| {
        Box::<dyn std::error::Error>::from(format!("Failed to open file {}: {}", filepath, e))
    })?;

    let file_size = file
        .metadata()
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to get metadata: {}", e)))?
        .len();

    // Stream through file checking for UTF-8 validity
    let mut reader = BufReader::new(file);
    let mut is_utf8 = true;
    let mut buffer = [0; 8192]; // 8KB buffer

    loop {
        let bytes_read = reader.read(&mut buffer).map_err(|e| {
            Box::<dyn std::error::Error>::from(format!("Failed to read file: {}", e))
        })?;

        if bytes_read == 0 {
            break;
        }

        // Check if the chunk is valid UTF-8
        if std::str::from_utf8(&buffer[..bytes_read]).is_err() {
            is_utf8 = false;
            break;
        }
    }

    if is_utf8 {
        // File is UTF-8, return original size
        Ok(("utf8".to_string(), file_size, file_size))
    } else {
        // File is not UTF-8, create or check base64 version
        let base64_path = format!("{}.base64", filepath);

        // Check if base64 file already exists and is up to date
        if Path::new(&base64_path).exists() {
            let base64_metadata = metadata(&base64_path).ok();
            let source_metadata = metadata(filepath).ok();

            // If both metadata exists and source is older than base64 file, use existing
            if let (Some(base64_meta), Some(source_meta)) = (base64_metadata, source_metadata) {
                if let (Ok(base64_modified), Ok(source_modified)) =
                    (base64_meta.modified(), source_meta.modified())
                {
                    if source_modified <= base64_modified {
                        // Base64 file is up to date, just return sizes
                        return Ok(("base64".to_string(), file_size, base64_meta.len()));
                    }
                }
            }
        }

        // Need to create/update the base64 version
        let source = File::open(filepath).map_err(|e| {
            Box::<dyn std::error::Error>::from(format!("Failed to open source file: {}", e))
        })?;

        let mut reader = BufReader::new(source);
        let output = File::create(&base64_path).map_err(|e| {
            Box::<dyn std::error::Error>::from(format!("Failed to create base64 file: {}", e))
        })?;

        let mut writer = BufWriter::new(output);

        // Stream file through base64 encoder
        let mut buffer = [0; 6144]; // 6KB divisible by 3 for base64

        loop {
            let bytes_read = reader.read(&mut buffer).map_err(|e| {
                Box::<dyn std::error::Error>::from(format!("Failed to read from source: {}", e))
            })?;

            if bytes_read == 0 {
                break;
            }

            let encoded = BASE64_STANDARD.encode(&buffer[..bytes_read]);
            writer.write_all(encoded.as_bytes()).map_err(|e| {
                Box::<dyn std::error::Error>::from(format!("Failed to write to base64 file: {}", e))
            })?;
        }

        writer.flush().map_err(|e| {
            Box::<dyn std::error::Error>::from(format!("Failed to flush base64 file: {}", e))
        })?;

        // Get size of base64 file
        let base64_size = File::open(&base64_path)
            .map_err(|e| {
                Box::<dyn std::error::Error>::from(format!(
                    "Failed to open base64 file for size check: {}",
                    e
                ))
            })?
            .metadata()
            .map_err(|e| {
                Box::<dyn std::error::Error>::from(format!(
                    "Failed to get base64 file metadata: {}",
                    e
                ))
            })?
            .len();

        Ok(("base64".to_string(), file_size, base64_size))
    }
}

/// Get detailed information about the file
pub fn get_file_details(file_path: &str, secret_name: &str) -> Result<FileDetails> {
    // Get file metadata for size and last modified time
    let metadata = metadata(file_path).map_err(|e| {
        Box::<dyn std::error::Error>::from(format!("Failed to get metadata: {}", e))
    })?;

    // Format the last modified time
    let modified = metadata.modified().map_err(|e| {
        Box::<dyn std::error::Error>::from(format!("Failed to get modified time: {}", e))
    })?;

    let datetime: DateTime<Local> = modified.into();
    let formatted_time = datetime.format("%m/%d/%y %H:%M:%S").to_string();

    // Check encoding and sizes (creates base64 version if needed)
    let (encoding, file_size, encoded_size) = check_encoding_and_size(file_path)?;

    // Calculate hash
    let hash = crate::secrets::utils::calculate_hash(file_path)?;

    // Determine destination based on encoded size
    let destination_cloud = if encoded_size > 24000 {
        Some("b2".to_string())
    } else {
        Some("azure_kv".to_string())
    };

    // Return the details
    Ok(FileDetails {
        file_path: file_path.to_string(),
        file_size,
        encoded_size,
        last_modified: formatted_time,
        secret_name: secret_name.to_string(),
        encoding,
        cloud_created: None,
        cloud_updated: None,
        cloud_type: destination_cloud,
        hash,
        hash_algo: "sha1".to_string(),
        cloud_upload_bucket: None, // Will be set during upload
    })
}

/// Helper function to format file size with appropriate units
pub fn format_file_size(size_bytes: u64) -> String {
    if size_bytes < 1024 {
        // Less than 1 KiB, display in bytes
        format!("{} bytes", size_bytes)
    } else if size_bytes < 1024 * 1024 {
        // Display in KiB with 2 decimal places
        let size_kib = size_bytes as f64 / 1024.0;
        format!("{:.2} KiB", size_kib)
    } else if size_bytes < 1024 * 1024 * 1024 {
        // Display in MiB with 2 decimal places
        let size_mib = size_bytes as f64 / (1024.0 * 1024.0);
        format!("{:.2} MiB", size_mib)
    } else if size_bytes < 1024 * 1024 * 1024 * 1024 {
        // Display in GiB with 2 decimal places
        let size_gib = size_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        format!("{:.2} GiB", size_gib)
    } else if size_bytes < 1024 * 1024 * 1024 * 1024 * 1024 {
        // Display in TiB with 2 decimal places
        let size_tib = size_bytes as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0);
        format!("{:.2} TiB", size_tib)
    } else {
        // Display in PiB with 2 decimal places (for extremely large files)
        let size_pib = size_bytes as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0);
        format!("{:.2} PiB", size_pib)
    }
}

/// Display file details information
pub fn display_file_details(details: &FileDetails) {
    // Display the details
    println!("File path: {}", details.file_path);
    println!("Original size: {}", format_file_size(details.file_size));

    // Show encoded size if different from original
    if details.encoding == "base64" {
        println!("Encoded size: {}", format_file_size(details.encoded_size));
    }

    println!("Last modified: {}", details.last_modified);
    println!("Secret name: {}", details.secret_name);
    println!("Hash: {} ({})", details.hash, details.hash_algo);

    // Display encoding information
    println!("Encoding: {}", details.encoding);

    // Display storage destination
    if let Some(cloud_type) = &details.cloud_type {
        println!("Storage destination: {}", cloud_type);
    }

    // Display cloud information if available
    if let Some(created) = &details.cloud_created {
        println!("Cloud created: {}", created);
    }

    if let Some(updated) = &details.cloud_updated {
        println!("Cloud last updated: {}", updated);
    }
}
