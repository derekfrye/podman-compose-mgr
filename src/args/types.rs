use clap::{Parser, ValueEnum};
use hostname;
use serde_json;
use std::io::Read;
use std::path::PathBuf;

use super::initialization::check_init_filepath;
use super::validators::{
    check_file_writable, check_file_writable_path, check_readable_dir, check_readable_file,
    check_readable_path, check_valid_json_path, check_writable_dir,
};

#[derive(Parser, Debug, serde::Serialize)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Search path for docker-compose files
    #[arg(
        short = 'p',
        long,
        value_name = "PATH",
        default_value = ".",
        value_parser = check_readable_dir
    )]
    pub path: PathBuf,
    /// rebuild = pull latest docker.io images and rebuild custom images, secrets = refresh secrets files (not impl yet)
    #[arg(short = 'm', long, default_value = "Rebuild", value_parser = clap::value_parser!(Mode))]
    pub mode: Mode,

    /// Print extra stuff (use -v -v or --verbose --verbose for even more detail)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
    /// Regex pattern(s) to exclude paths, e.g., docker/archive or [^\.]+/archive
    #[arg(short, long)]
    pub exclude_path_patterns: Vec<String>,
    /// Regex pattern(s) to include paths. If both incl. and excl. are specified, excl. is applied first.
    #[arg(short, long)]
    pub include_path_patterns: Vec<String>,
    #[arg(short, long)]
    pub build_args: Vec<String>,
    /// Pass as guid or filepath
    #[arg(long)]
    pub secrets_client_id: Option<String>,
    /// Pass as filepath
    #[arg(long)]
    pub secrets_client_secret_path: Option<PathBuf>,
    /// Pass as guid or filepath
    #[arg(long)]
    pub secrets_tenant_id: Option<String>,
    /// Pass as guid or filepath
    #[arg(long)]
    pub secrets_vault_name: Option<String>,
    #[arg(long, value_parser = check_file_writable)]
    pub output_json: Option<PathBuf>,
    #[arg(long, value_parser = check_readable_file)]
    pub input_json: Option<PathBuf>,
    /// Path to the JSON file containing secret files to initialize
    #[arg(long, value_parser = check_readable_file)]
    pub secrets_init_filepath: Option<PathBuf>,

    // S3-compatible storage parameters (B2/R2)
    /// Path to file containing S3-compatible account ID/key ID (for B2/R2)
    #[arg(long)]
    pub s3_account_id_filepath: Option<PathBuf>,

    /// Path to file containing S3-compatible access key/secret (for B2/R2)
    #[arg(long)]
    pub s3_secret_key_filepath: Option<PathBuf>,

    /// Path to file containing S3-compatible endpoint (for R2, the Cloudflare account ID)
    #[arg(long)]
    pub s3_endpoint_filepath: Option<PathBuf>,

    /// Directory to use for temporary files
    #[arg(long, default_value = "/tmp", value_parser = check_writable_dir)]
    pub temp_file_path: PathBuf,
}

impl Default for Args {
    fn default() -> Self {
        // Use check_writable_dir to ensure the default path is valid or created
        // We need to handle the potential error here, perhaps by panicking
        // if the default /tmp isn't usable, as it's a fundamental requirement.
        let default_temp_path = check_writable_dir("/tmp")
            .expect("Default temporary directory '/tmp' must be writable or creatable.");

        Self {
            path: PathBuf::from("."),
            mode: Mode::Rebuild,
            verbose: 0,
            exclude_path_patterns: Vec::new(),
            include_path_patterns: Vec::new(),
            build_args: Vec::new(),
            secrets_client_id: None,
            secrets_client_secret_path: None,
            secrets_tenant_id: None,
            secrets_vault_name: None,
            output_json: None,
            input_json: None,
            secrets_init_filepath: None,
            s3_account_id_filepath: None,
            s3_secret_key_filepath: None,
            s3_endpoint_filepath: None,
            temp_file_path: default_temp_path,
        }
    }
}

impl Args {
    /// Validates and processes the secrets init filepath, updating it if necessary.
    ///
    /// When in SecretInitialize mode, if secrets_init_filepath is a list of filenames,
    /// this function will create a new JSON file and update the filepath.
    ///
    /// # Returns
    ///
    /// * `Result<(), String>` - Success or error message
    pub fn validate_and_process(&mut self) -> Result<(), String> {
        if let Mode::SecretInitialize = self.mode {
            if let Some(filepath) = &self.secrets_init_filepath {
                if let Some(path_str) = filepath.to_str() {
                    // Process the file and get updated path if needed
                    let new_path = check_init_filepath(path_str)?;
                    // Update the filepath with the processed one
                    self.secrets_init_filepath = Some(new_path);
                } else {
                    return Err(
                        "secrets_init_filepath contains invalid UTF-8 characters".to_string()
                    );
                }
            } else {
                return Err(
                    "secrets_init_filepath is required for SecretInitialize mode".to_string(),
                );
            }

            if self.output_json.is_none() {
                return Err("output_json is required for SecretInitialize mode".to_string());
            }
        }

        // Call the regular validation for other checks
        self.validate()
    }

    /// Check if cloud storage credentials are needed based on entries in the input JSON
    ///
    /// Returns a tuple of booleans (need_b2, need_r2) indicating if credentials
    /// are needed for each cloud storage type.
    fn needs_cloud_credentials_for_upload(&self) -> Result<(bool, bool), String> {
        // Get the input JSON path
        let input_json_path = match &self.input_json {
            Some(path) => path,
            None => return Ok((false, false)), // No input JSON, no cloud credentials needed
        };

        // Get current hostname for comparison
        let current_hostname = match hostname::get() {
            Ok(hostname) => hostname.to_string_lossy().to_string(),
            Err(e) => return Err(format!("Failed to get system hostname: {}", e)),
        };

        // Try to read the input JSON file
        let mut file = match std::fs::File::open(input_json_path) {
            Ok(file) => file,
            Err(e) => return Err(format!("Failed to open input JSON file: {}", e)),
        };

        let mut content = String::new();
        if let Err(e) = file.read_to_string(&mut content) {
            return Err(format!("Failed to read input JSON file: {}", e));
        }

        // Parse JSON as array
        let entries: Vec<serde_json::Value> = match serde_json::from_str(&content) {
            Ok(entries) => entries,
            Err(e) => return Err(format!("Failed to parse input JSON: {}", e)),
        };

        // Variables to track if we need credentials for each cloud provider
        let mut need_b2 = false;
        let mut need_r2 = false;

        // Check if any entry is for cloud storage and matches the current hostname
        for entry in entries {
            // Get hostname
            let hostname = match entry.get("hostname").and_then(|v| v.as_str()) {
                Some(h) => h,
                None => continue, // Skip entries without hostname
            };

            // Get destination cloud
            let destination_cloud = match entry.get("destination_cloud").and_then(|v| v.as_str()) {
                Some(d) => d,
                None => continue, // Skip entries without destination_cloud
            };

            // Skip if the hostname doesn't match current host
            if hostname != current_hostname {
                continue;
            }

            // Check the destination cloud type
            match destination_cloud {
                "b2" => need_b2 = true,
                "r2" => need_r2 = true,
                _ => {} // Other cloud types (including azure_kv) don't need special handling here
            }

            // If we already need both types of credentials, we can exit early
            if need_b2 && need_r2 {
                break;
            }
        }

        // Return which cloud credentials are needed
        Ok((need_b2, need_r2))
    }

    /// Validate the secrets based on the mode, without modifying the Args
    pub fn validate(&self) -> Result<(), String> {
        if let Mode::SecretRetrieve = self.mode {
            if let Some(client_id) = &self.secrets_client_id {
                check_readable_file(client_id)?;
            }

            if let Some(client_secret) = &self.secrets_client_secret_path {
                check_readable_path(client_secret)?;
            }

            if let Some(output_json) = &self.output_json {
                check_file_writable_path(output_json)?;
            }

            if let Some(input_json) = &self.input_json {
                check_valid_json_path(input_json)?;
            }
        } else if let Mode::SecretInitialize = self.mode {
            // Basic validation - make sure fields exist
            if self.secrets_init_filepath.is_none() {
                return Err(
                    "secrets_init_filepath is required for SecretInitialize mode".to_string(),
                );
            }

            if let Some(output_path) = &self.output_json {
                check_file_writable_path(output_path)?;
            } else {
                return Err("output_json is required for SecretInitialize mode".to_string());
            }

            // Bucket for upload is now specified in the JSON file, no longer a command-line argument

            // The actual processing of secrets_init_filepath happens in validate_and_process()
        } else if let Mode::SecretUpload = self.mode {
            // Check for required fields for SecretUpload mode
            if self.input_json.is_none() {
                return Err("input_json is required for SecretUpload mode".to_string());
            }
            if self.output_json.is_none() {
                return Err("output_json is required for SecretUpload mode".to_string());
            }

            // Check for Azure KeyVault credentials
            if self.secrets_vault_name.is_none() {
                return Err("secrets_vault_name is required for SecretUpload mode".to_string());
            }
            if self.secrets_tenant_id.is_none() {
                return Err("secrets_tenant_id is required for SecretUpload mode".to_string());
            }
            if self.secrets_client_secret_path.is_none() {
                return Err(
                    "secrets_client_secret_path is required for SecretUpload mode".to_string(),
                );
            }
            if self.secrets_client_id.is_none() {
                return Err("secrets_client_id is required for SecretUpload mode".to_string());
            }

            // Validate the file paths
            if let Some(input_json) = &self.input_json {
                check_valid_json_path(input_json)?;

                // Check if cloud credentials are needed for any entries in the input JSON
                let (need_b2_credentials, need_r2_credentials) =
                    self.needs_cloud_credentials_for_upload()?;

                // Handle both B2 and R2 credentials with shared parameters
                if need_b2_credentials || need_r2_credentials {
                    // Check if s3_account_id_filepath is provided
                    if self.s3_account_id_filepath.is_none() {
                        return Err("s3_account_id_filepath is required for upload mode when input json contains B2 or R2 entries".to_string());
                    }

                    // Check if s3_secret_key_filepath is provided
                    if self.s3_secret_key_filepath.is_none() {
                        return Err("s3_secret_key_filepath is required for upload mode when input json contains B2 or R2 entries".to_string());
                    }

                    // Validate s3_account_id_filepath
                    if let Some(filepath) = &self.s3_account_id_filepath {
                        check_readable_path(filepath)?;
                    }

                    // Validate s3_secret_key_filepath
                    if let Some(filepath) = &self.s3_secret_key_filepath {
                        check_readable_path(filepath)?;
                    }

                    // For R2, we need to check if both access key ID and endpoint (account ID) are provided
                    if need_r2_credentials {
                        if self.s3_account_id_filepath.is_none() {
                            return Err("s3_account_id_filepath is required for upload mode with R2 entries".to_string());
                        }

                        if self.s3_endpoint_filepath.is_none() {
                            return Err(
                                "s3_endpoint_filepath is required for upload mode with R2 entries"
                                    .to_string(),
                            );
                        }

                        // Validate s3_endpoint_filepath if provided
                        if let Some(filepath) = &self.s3_endpoint_filepath {
                            check_readable_path(filepath)?;
                        }
                    }
                }
            }

            if let Some(client_secret) = &self.secrets_client_secret_path {
                check_readable_path(client_secret)?;
            }

            if let Some(output_json) = &self.output_json {
                check_file_writable_path(output_json)?;
            }
        }
        Ok(())
    }
}

/// Enumeration of possible modes
#[derive(Clone, ValueEnum, Debug, Copy, serde::Serialize)]
pub enum Mode {
    Rebuild,
    SecretRetrieve,
    SecretInitialize,
    SecretUpload,
}
