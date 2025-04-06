use clap::{Parser, ValueEnum};
use std::path::PathBuf;

use super::initialization::check_init_filepath;
use super::validators::{
    check_file_writable, check_file_writable_path, check_readable_dir, check_readable_file,
    check_readable_path, check_valid_json_path,
};

#[derive(Parser)]
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

    /// Print extra stuff
    #[arg(short, long)]
    pub verbose: bool,
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

    // B2 credentials and configuration
    /// Backblaze B2 key ID
    #[arg(long)]
    pub b2_key_id: Option<String>,

    /// Backblaze B2 application key
    #[arg(long)]
    pub b2_application_key: Option<String>,

    /// Backblaze B2 bucket name
    #[arg(long)]
    pub b2_bucket_name: Option<String>,

    /// Backblaze B2 bucket name to use for uploads
    #[arg(long)]
    pub b2_bucket_for_upload: Option<String>,
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

    /// Validate the secrets based on the mode, without modifying the Args
    pub fn validate(&self) -> Result<(), String> {
        if let Mode::SecretRefresh = self.mode {
            if let Some(client_id) = &self.secrets_client_id {
                if client_id.len() != 8 {
                    return Err(
                        "secrets_client_id must be exactly 8 characters long when Mode is Secrets."
                            .to_string(),
                    );
                }
            }

            if let Some(client_secret) = &self.secrets_client_secret_path {
                check_readable_path(client_secret)?;
            }
        } else if let Mode::SecretRetrieve = self.mode {
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
            
            // B2 bucket for upload is required for SecretInitialize mode
            if self.b2_bucket_for_upload.is_none() {
                return Err("b2_bucket_for_upload is required for SecretInitialize mode".to_string());
            }

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

            // Also check for B2 credentials as they might be needed for large files
            if self.b2_key_id.is_none() {
                return Err(
                    "b2_key_id is required for SecretUpload mode with large files".to_string(),
                );
            }
            if self.b2_application_key.is_none() {
                return Err(
                    "b2_application_key is required for SecretUpload mode with large files"
                        .to_string(),
                );
            }
            if self.b2_bucket_name.is_none() {
                return Err(
                    "b2_bucket_name is required for SecretUpload mode with large files".to_string(),
                );
            }

            // Validate the file paths
            if let Some(input_json) = &self.input_json {
                check_valid_json_path(input_json)?;
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
#[derive(Clone, ValueEnum, Debug, Copy)]
pub enum Mode {
    Rebuild,
    SecretRefresh,
    SecretRetrieve,
    SecretInitialize,
    SecretUpload,
    RestartSvcs,
}
