use crate::secrets::file_details::FileDetails;
use crate::secrets::r2_storage::R2UploadResult;
use crate::secrets::utils::get_hostname;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::OffsetDateTime;

pub struct SetSecretResponse {
    pub created: OffsetDateTime,
    pub updated: OffsetDateTime,
    pub name: String,
    pub id: String,
    pub value: String,
}

/// Represents an entry in the upload JSON file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadEntry {
    // Required fields
    pub file_nm: String,
    pub hash: String,
    pub ins_ts: String,
    pub hostname: String,

    // Optional fields with defaults
    #[serde(default = "default_hash_algo")]
    pub hash_algo: String,
    #[serde(default = "default_utf8")]
    pub encoding: String,
    #[serde(default)]
    pub file_size: u64,
    #[serde(default)]
    pub encoded_size: u64,
    #[serde(default = "default_azure_kv")]
    pub destination_cloud: String,
    pub cloud_upload_bucket: Option<String>,
    pub cloud_prefix: Option<String>,
}

// Helper functions for default values
fn default_hash_algo() -> String {
    "sha1".to_string()
}

fn default_utf8() -> String {
    "utf8".to_string()
}

fn default_azure_kv() -> String {
    "azure_kv".to_string()
}

impl UploadEntry {
    /// Create a new UploadEntry with minimal required fields
    pub fn new(file_path: &str, hash: &str) -> Self {
        let hostname = get_hostname().unwrap_or_else(|_| "unknown_host".to_string());
        let now = chrono::Utc::now().to_rfc3339();

        UploadEntry {
            file_nm: file_path.to_string(),
            hash: hash.to_string(),
            ins_ts: now,
            hostname,
            hash_algo: default_hash_algo(),
            encoding: default_utf8(),
            file_size: 0,
            encoded_size: 0,
            destination_cloud: default_azure_kv(),
            cloud_upload_bucket: None,
            cloud_prefix: None,
        }
    }

    /// Create an UploadEntry for R2 storage
    pub fn new_for_r2(file_path: &str, hash: &str, bucket: &str) -> Self {
        let mut entry = Self::new(file_path, hash);
        entry.destination_cloud = "r2".to_string();
        entry.cloud_upload_bucket = Some(bucket.to_string());
        entry
    }

    /// Set file size information
    pub fn with_size_info(mut self, file_size: u64, encoded_size: Option<u64>) -> Self {
        self.file_size = file_size;
        self.encoded_size = encoded_size.unwrap_or(file_size);
        self
    }

    /// Convert to FileDetails struct
    pub fn to_file_details(&self) -> FileDetails {
        FileDetails {
            file_path: self.file_nm.clone(),
            file_size: self.file_size,
            encoded_size: self.encoded_size,
            last_modified: String::new(), // Not needed for upload
            encoding: self.encoding.clone(),
            cloud_created: None,
            cloud_updated: None,
            cloud_type: Some(self.destination_cloud.clone()),
            hash: self.hash.clone(),
            hash_algo: self.hash_algo.clone(),
            cloud_upload_bucket: self.cloud_upload_bucket.clone(),
            cloud_prefix: self.cloud_prefix.clone(),
        }
    }

    /// Check if this entry is too large for Azure KeyVault
    pub fn is_too_large_for_keyvault(&self) -> bool {
        self.encoded_size > 24000
    }

    /// Create output JSON entry for R2 storage
    pub fn create_r2_output_entry(&self, r2_result: &R2UploadResult) -> Value {
        json!({
            "file_nm": self.file_nm,
            "hash": self.hash,
            "hash_algo": self.hash_algo,
            "ins_ts": self.ins_ts,
            "cloud_id": r2_result.id,
            "cloud_cr_ts": "", // R2 doesn't provide created time separately
            "cloud_upd_ts": "", // Use current time if needed
            "hostname": self.hostname,
            "encoding": self.encoding,
            "file_size": self.file_size,
            "encoded_size": self.encoded_size,
            "destination_cloud": "r2",
            "cloud_upload_bucket": self.cloud_upload_bucket.clone().unwrap_or_default(),
            "cloud_prefix": self.cloud_prefix.clone().unwrap_or_default(),
            "r2_hash": r2_result.hash,
            "r2_bucket_id": r2_result.bucket_id,
            "r2_name": r2_result.name
        })
    }

    /// Create output JSON entry for Azure KeyVault storage
    pub fn create_azure_output_entry(&self, kv_response: &SetSecretResponse) -> Value {
        json!({
            "file_nm": self.file_nm,
            "hash": self.hash,
            "hash_algo": self.hash_algo,
            "ins_ts": self.ins_ts,
            "cloud_id": kv_response.id,
            "cloud_cr_ts": kv_response.created.to_string(),
            "cloud_upd_ts": kv_response.updated.to_string(),
            "hostname": self.hostname,
            "encoding": self.encoding,
            "file_size": self.file_size,
            "encoded_size": self.encoded_size,
            "destination_cloud": self.destination_cloud,
            "cloud_upload_bucket": self.cloud_upload_bucket.clone().unwrap_or_default()
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct JsonOutput {
    pub file_nm: String,
    #[serde(default)]
    pub md5: String, // This is the file hash for backward compatibility
    pub ins_ts: String,
    #[serde(default)]
    pub az_id: String,
    #[serde(default)]
    pub az_create: String,
    #[serde(default)]
    pub az_updated: String,
    #[serde(default)]
    pub az_name: String,
    pub hostname: String,
    #[serde(default = "default_encoding")]
    pub encoding: String,
    #[serde(rename = "hash", default = "String::new")]
    pub hash_val: String, // Added for compatibility with B2 storage
    #[serde(default = "default_hash_algo")]
    pub hash_algo: String, // Added for compatibility with B2 storage
    #[serde(default = "default_azure_kv_output")]
    pub destination_cloud: String, // The cloud storage destination
    #[serde(default)]
    pub file_size: u64, // The original file size
    #[serde(default)]
    pub encoded_size: u64, // The size after encoding
    #[serde(default)]
    pub cloud_upload_bucket: String, // The bucket name for cloud storage
    #[serde(default)]
    pub cloud_id: String, // The cloud storage ID (shared field)
    #[serde(default)]
    pub cloud_cr_ts: String, // Cloud creation timestamp
    #[serde(default)]
    pub cloud_upd_ts: String, // Cloud update timestamp
    #[serde(default)]
    pub cloud_prefix: String, // Cloud storage prefix

    // R2-specific fields
    #[serde(default)]
    pub r2_hash: String, // R2 object hash/etag
    #[serde(default)]
    pub r2_bucket_id: String, // R2 bucket ID
    #[serde(default)]
    pub r2_name: String, // R2 object name
}

// Add Vec-like operations for JsonOutput
impl JsonOutput {
    pub fn is_empty(&self) -> bool {
        self.file_nm.is_empty()
    }
    
    pub fn to_json_entry(&self) -> JsonEntry {
        JsonEntry {
            file_name: self.file_nm.clone(),
            hostname: self.hostname.clone(),
            destination_cloud: self.destination_cloud.clone(),
            sha256: if !self.hash_val.is_empty() { Some(self.hash_val.clone()) } else { None },
            last_updated: if !self.cloud_upd_ts.is_empty() { Some(self.cloud_upd_ts.clone()) } else { None },
        }
    }
    
    pub fn iter(&self) -> std::vec::IntoIter<JsonEntry> {
        // Convert to JsonEntry format for iteration
        let entries = vec![self.to_json_entry()];
        
        entries.into_iter()
    }
}

/// Collection type to handle arrays of JsonOutput entries
pub type JsonOutputCollection = Vec<JsonOutput>;

/// Simplified entry for migration operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonEntry {
    pub file_name: String,
    pub hostname: String,
    pub destination_cloud: String,
    pub sha256: Option<String>,
    pub last_updated: Option<String>,
}

// Default encoding for backward compatibility with existing JSON files
#[allow(dead_code)]
fn default_encoding() -> String {
    "utf8".to_string()
}

// Default hash algorithm for backward compatibility
#[allow(dead_code)]
fn default_hash_algo_output() -> String {
    "sha1".to_string()
}

// Default destination_cloud for compatibility
#[allow(dead_code)]
fn default_azure_kv_output() -> String {
    "azure_kv".to_string()
}

pub struct JsonOutputControl {
    pub json_output: JsonOutput,
    pub validate_all: bool,
}

impl Default for JsonOutputControl {
    fn default() -> Self {
        Self {
            json_output: JsonOutput {
                file_nm: String::new(),
                md5: String::new(),
                ins_ts: String::new(),
                az_id: String::new(),
                az_create: String::new(),
                az_updated: String::new(),
                az_name: String::new(),
                hostname: String::new(),
                encoding: "utf8".to_string(),
                hash_val: String::new(),
                hash_algo: "sha1".to_string(),
                destination_cloud: "azure_kv".to_string(),
                file_size: 0,
                encoded_size: 0,
                cloud_upload_bucket: String::new(),
                cloud_id: String::new(),
                cloud_cr_ts: String::new(),
                cloud_upd_ts: String::new(),
                cloud_prefix: String::new(),
                r2_hash: String::new(),
                r2_bucket_id: String::new(),
                r2_name: String::new(),
            },
            validate_all: false,
        }
    }
}

impl JsonOutputControl {
    pub fn new() -> JsonOutputControl {
        Self::default()
    }
}
