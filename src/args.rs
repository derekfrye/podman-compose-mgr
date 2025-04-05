use clap::{Parser, ValueEnum};
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
// use clap::builder::ValueParser;

pub fn args_checks() -> Args {
    Args::parse()
}

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
    #[arg(long, value_parser = check_parent_dir_is_writeable)]
    pub secret_mode_output_json: Option<PathBuf>,
    #[arg(long, value_parser = check_readable_file)]
    pub secret_mode_input_json: Option<PathBuf>,
    /// Path to the JSON file containing secret files to initialize
    #[arg(long, value_parser = check_readable_file)]
    pub secrets_init_filepath: Option<PathBuf>,
}

impl Args {
    /// Validate the secrets based on the mode
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
                check_readable_file(client_secret.to_str().unwrap())?;
            }
        } else if let Mode::SecretRetrieve = self.mode {
            if let Some(client_id) = &self.secrets_client_id {
               check_readable_file(client_id)?;
            }

            if let Some(client_secret) = &self.secrets_client_secret_path {
                check_readable_file(client_secret.to_str().unwrap())?;
            }

            if let Some(output_json) = &self.secret_mode_output_json {
                check_parent_dir_is_writeable(output_json.to_str().unwrap())?;
            }

            if let Some(input_json) = &self.secret_mode_input_json {
                check_valid_json_file(input_json.to_str().unwrap())?;
            }
        } else if let Mode::SecretInitialize = self.mode {
            if let Some(filepath) = &self.secrets_init_filepath {
                check_readable_file(filepath.to_str().unwrap())?;
            } else {
                return Err("secrets_init_filepath is required for SecretInitialize mode".to_string());
            }
            
            if let Some(output_dir) = &self.secret_mode_input_json {
                check_parent_dir_is_writeable(output_dir.to_str().unwrap())?;
            } else {
                return Err("secret_mode_input_json is required for SecretInitialize mode".to_string());
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
    RestartSvcs,
}

// for a passed PathBuf, get the parent dir, check if it exists and is writable
fn check_parent_dir_is_writeable(existing_file: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(existing_file).to_owned();
    let parent_path = path.parent().unwrap();
    if parent_path.is_dir()
        && fs::metadata(parent_path).is_ok()
        && fs::read_dir(parent_path).is_ok()
    {
        let temp_file_path = parent_path.join(".temp_write_check");
        match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_file_path)
        {
            Ok(mut file) => {
                let _ = file
                    .write_all(b"test")
                    .map_err(|e| format!("Failed to write to temp file: {}", e));
                let _ = fs::remove_file(&temp_file_path); // Clean up the temporary file
                Ok(path)
            }
            Err(_) => Err(format!("The dir '{}' is not writable.", existing_file)),
        }
    } else {
        Err(format!("The dir '{}' is not writable.", existing_file))
    }
}

fn check_readable_file(file: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(file);
    if path.is_file() && fs::metadata(&path).is_ok() {
        Ok(path)
    } else {
        Err(format!("The file '{}' is not readable.", file))
    }
}

fn check_valid_json_file(file: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(file);
    let mut file = File::open(&path).map_err(|e| e.to_string())?;
    let mut file_content = String::new();
    file.read_to_string(&mut file_content)
        .map_err(|e| e.to_string())?;
    let mut entries = Vec::new();
    let deserializer = serde_json::Deserializer::from_str(&file_content).into_iter::<Value>();

    for entry in deserializer {
        let entry = entry.map_err(|e| e.to_string())?;
        entries.push(entry);
    }
    Ok(path)
}

fn check_readable_dir(dir: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(dir);
    if path.is_dir() && fs::metadata(&path).is_ok() && fs::read_dir(&path).is_ok() {
        Ok(path)
    } else {
        Err(format!("The dir '{}' is not readable.", dir))
    }
}
