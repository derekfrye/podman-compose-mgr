use chrono::{DateTime, Local};
use std::fs::{self, metadata};
use crate::secrets::error::Result;

/// Represents the detailed information about a file for secret upload
#[derive(Debug, Clone)]
pub struct FileDetails {
    pub file_path: String,
    pub size_bytes: u64,
    pub last_modified: String,
    pub secret_name: String,
    pub is_utf8: bool,
    pub az_created: Option<String>,
    pub az_updated: Option<String>,
}

/// Get detailed information about the file
pub fn get_file_details(file_path: &str, encoded_name: &str) -> Result<FileDetails> {
    // Get file metadata for size and last modified time
    let metadata = metadata(file_path)
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to get metadata: {}", e)))?;
    
    // Get file size in bytes
    let size_bytes = metadata.len();
    
    // Format the last modified time
    let modified = metadata.modified()
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to get modified time: {}", e)))?;
    
    let datetime: DateTime<Local> = modified.into();
    let formatted_time = datetime.format("%m/%d/%y %H:%M:%S").to_string();
    
    // Check if the file contains valid UTF-8
    let is_utf8 = match fs::read(file_path) {
        Ok(bytes) => std::str::from_utf8(&bytes).is_ok(),
        Err(_) => false, // If we can't read the file, assume it's not UTF-8
    };
    
    // Return the details
    Ok(FileDetails {
        file_path: file_path.to_string(),
        size_bytes,
        last_modified: formatted_time,
        secret_name: encoded_name.to_string(),
        is_utf8,
        az_created: None,
        az_updated: None,
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
    println!("Size: {}", format_file_size(details.size_bytes));
    println!("Last modified: {}", details.last_modified);
    println!("Secret name: {}", details.secret_name);
    
    // Display encoding information
    if !details.is_utf8 {
        println!("Encoding: Non-UTF-8 content (will be base64 encoded)");
    } else {
        println!("Encoding: UTF-8");
    }
    
    // Display Azure Key Vault information if available
    if let Some(created) = &details.az_created {
        println!("Azure Key Vault created: {}", created);
    }
    
    if let Some(updated) = &details.az_updated {
        println!("Azure Key Vault last updated: {}", updated);
    }
}