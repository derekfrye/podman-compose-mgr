use serde::Serialize;
use time::OffsetDateTime;

pub struct SetSecretResponse {
    pub created: OffsetDateTime,
    pub updated: OffsetDateTime,
    pub name: String,
    pub id: String,
    pub value: String,
}

#[derive(Serialize)]
pub struct JsonOutput {
    pub file_nm: String,
    pub md5: String,  // This is the file hash
    pub ins_ts: String,
    pub az_id: String,
    pub az_create: String,
    pub az_updated: String,
    pub az_name: String,
    pub hostname: String,
    #[serde(default = "default_encoding")]
    pub encoding: String,
    #[serde(rename = "hash", default = "String::new")]
    pub hash_val: String,  // Added for compatibility with B2 storage
    #[serde(default = "default_hash_algo")]
    pub hash_algo: String, // Added for compatibility with B2 storage
}

// Default encoding for backward compatibility with existing JSON files
#[allow(dead_code)]
fn default_encoding() -> String {
    "utf8".to_string()
}

// Default hash algorithm for backward compatibility
#[allow(dead_code)]
fn default_hash_algo() -> String {
    "sha1".to_string()
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
