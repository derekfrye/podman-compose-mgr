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
    #[arg(long, value_parser = check_file_writable)]
    pub output_json: Option<PathBuf>,
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

            if let Some(output_json) = &self.output_json {
                check_file_writable(output_json.to_str().unwrap())?;
            }

            if let Some(input_json) = &self.secret_mode_input_json {
                check_valid_json_file(input_json.to_str().unwrap())?;
            }
        } else if let Mode::SecretInitialize = self.mode {
            if let Some(filepath) = &self.secrets_init_filepath {
                check_init_filepath(filepath.to_str().unwrap())?;
            } else {
                return Err("secrets_init_filepath is required for SecretInitialize mode".to_string());
            }
            
            if let Some(output_dir) = &self.secret_mode_input_json {
                check_file_writable(output_dir.to_str().unwrap())?;
            } else {
                return Err("secret_mode_input_json is required for SecretInitialize mode".to_string());
            }
        } else if let Mode::SecretUpload = self.mode {
            // Check for required fields for SecretUpload mode
            if self.secret_mode_input_json.is_none() {
                return Err("secret_mode_input_json is required for SecretUpload mode".to_string());
            }
            if self.secrets_vault_name.is_none() {
                return Err("secrets_vault_name is required for SecretUpload mode".to_string());
            }
            if self.secrets_tenant_id.is_none() {
                return Err("secrets_tenant_id is required for SecretUpload mode".to_string());
            }
            if self.secrets_client_secret_path.is_none() {
                return Err("secrets_client_secret_path is required for SecretUpload mode".to_string());
            }
            if self.secrets_client_id.is_none() {
                return Err("secrets_client_id is required for SecretUpload mode".to_string());
            }
            if self.output_json.is_none() {
                return Err("secret_mode_output_json is required for SecretUpload mode".to_string());
            }
            
            // Validate the file paths
            if let Some(input_json) = &self.secret_mode_input_json {
                check_valid_json_file(input_json.to_str().unwrap())?;
            }
            if let Some(client_secret) = &self.secrets_client_secret_path {
                check_readable_file(client_secret.to_str().unwrap())?;
            }
            if let Some(output_json) = &self.output_json {
                check_file_writable(output_json.to_str().unwrap())?;
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

// Checks if a file is valid for initialization
// It must be readable and either valid JSON or contain a list of filenames
fn check_init_filepath(file_path: &str) -> Result<PathBuf, String> {
    // First check if the file is readable
    let path = check_readable_file(file_path)?;
    
    // Try to parse as JSON
    let mut file = File::open(&path).map_err(|e| e.to_string())?;
    let mut file_content = String::new();
    file.read_to_string(&mut file_content).map_err(|e| e.to_string())?;
    
    // If empty file, return error
    if file_content.trim().is_empty() {
        return Err(format!("The file '{}' is empty.", file_path));
    }
    
    // Try to parse as JSON first
    let json_result = serde_json::from_str::<serde_json::Value>(&file_content);
    
    match json_result {
        Ok(_) => {
            // Valid JSON, no further processing needed
            Ok(path)
        },
        Err(_) => {
            // Not valid JSON, check if it's a list of filenames (one per line)
            let lines: Vec<&str> = file_content.lines().collect();
            
            // Filter out empty lines
            let non_empty_lines: Vec<&str> = lines.iter()
                .filter(|line| !line.trim().is_empty())
                .copied()
                .collect();
            
            if non_empty_lines.is_empty() {
                return Err(format!("The file '{}' does not contain valid JSON or filenames.", file_path));
            }
            
            // Check if each filename is a readable file
            let mut all_valid = true;
            for line in &non_empty_lines {
                let filename = line.trim();
                if check_readable_file(filename).is_err() {
                    eprintln!("Warning: File '{}' does not exist or is not readable.", filename);
                    all_valid = false;
                }
            }
            
            if !all_valid {
                eprintln!("Warning: Some files are not readable. Continuing with valid files only.");
            }
            
            // Create JSON array with filenames (only for files that exist and are readable)
            let mut file_array = Vec::new();
            for line in non_empty_lines {
                let filename = line.trim();
                if check_readable_file(filename).is_ok() {
                    file_array.push(serde_json::json!({"filenm": filename}));
                }
            }
            
            // If no valid files found, return an error
            if file_array.is_empty() {
                return Err(format!("No readable files found in '{}'.", file_path));
            }
            
            // Write back to the file
            let json_content = serde_json::to_string_pretty(&file_array)
                .map_err(|e| format!("Failed to convert filename list to JSON: {}", e))?;
            
            let mut output_file = File::create(&path)
                .map_err(|e| format!("Failed to open file for writing: {}", e))?;
            
            output_file.write_all(json_content.as_bytes())
                .map_err(|e| format!("Failed to write JSON content: {}", e))?;
            
            Ok(path)
        }
    }
}

// Checks if a file is writable (or can be created and written to)
fn check_file_writable(file_path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(file_path);
    
    // First check if the parent directory exists and is writable
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            return Err(format!("The parent directory of '{}' does not exist.", file_path));
        }
        
        if !parent.is_dir() {
            return Err(format!("The parent path '{}' is not a directory.", parent.display()));
        }
    }
    
    // Try to open the file in write mode
    match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false) // Don't truncate an existing file
        .open(&path)
    {
        Ok(_) => Ok(path),
        Err(e) => Err(format!("The file '{}' is not writable: {}", file_path, e)),
    }
}
