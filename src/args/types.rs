use clap::{Parser, ValueEnum};
use std::path::PathBuf;

use super::initialization::check_init_filepath;
use super::validators::{
    check_file_writable, check_readable_dir, check_readable_file, check_writable_dir, validate,
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
    #[arg(long, value_parser = check_readable_file)]
    pub azure_client_id_path: Option<PathBuf>,
    /// Pass as filepath
    #[arg(long, value_parser = check_readable_file)]
    pub azure_client_secret_path: Option<PathBuf>,
    /// Pass as guid or filepath
    #[arg(long, value_parser = check_readable_file)]
    pub azure_tenant_id_path: Option<PathBuf>,
    /// Pass as guid or filepath
    #[arg(long, value_parser = check_readable_file)]
    pub azure_vault_name_path: Option<PathBuf>,
    #[arg(long, value_parser = check_file_writable)]
    pub output_json: Option<PathBuf>,
    #[arg(long, value_parser = check_readable_file)]
    pub input_json: Option<PathBuf>,
    /// Path to the flat file that lists on-disk secrets
    #[arg(long, value_parser = check_readable_file)]
    pub secrets_init_filepath: Option<PathBuf>,

    // S3-compatible storage parameters (B2/R2)
    /// Path to file containing S3-compatible account ID/key ID (for B2/R2)
    #[arg(long, value_parser = check_readable_file)]
    pub s3_account_id_filepath: Option<PathBuf>,

    /// Path to file containing S3-compatible access key/secret (for B2/R2)
    #[arg(long, value_parser = check_readable_file)]
    pub s3_secret_key_filepath: Option<PathBuf>,

    /// Path to file containing S3-compatible endpoint (for R2, the Cloudflare account ID)
    #[arg(long, value_parser = check_readable_file)]
    pub s3_endpoint_filepath: Option<PathBuf>,

    /// Directory to use for temporary files
    #[arg(long, default_value = "/tmp", value_parser = check_writable_dir)]
    pub temp_file_path: PathBuf,

    /// Use terminal UI mode
    #[arg(long)]
    pub tui: bool,
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
            azure_client_id_path: None,
            azure_client_secret_path: None,
            azure_tenant_id_path: None,
            azure_vault_name_path: None,
            output_json: None,
            input_json: None,
            secrets_init_filepath: None,
            s3_account_id_filepath: None,
            s3_secret_key_filepath: None,
            s3_endpoint_filepath: None,
            temp_file_path: default_temp_path,
            tui: false,
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

    /// Validate the secrets based on the mode, without modifying the Args
    pub fn validate(&self) -> Result<(), String> {
        // Call the validate function from validators.rs
        validate(self)
    }
}

/// Enumeration of possible modes
#[derive(Clone, ValueEnum, Debug, Copy, serde::Serialize)]
pub enum Mode {
    Rebuild,
    SecretRetrieve,
    SecretInitialize,
    SecretUpload,
    SecretMigrate,
}
