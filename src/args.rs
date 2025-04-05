use clap::{Parser, ValueEnum};
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
// use clap::builder::ValueParser;
use home::home_dir;

/// Parse command line arguments and perform validation with processing
///
/// This function:
/// 1. Parses command line arguments
/// 2. For SecretInitialize mode, processes the init filepath if needed
/// 3. Returns the validated Args structure
///
/// # Returns
///
/// * `Args` - The validated arguments
///
/// # Panics
///
/// Panics if validation fails
pub fn args_checks() -> Args {
    let mut args = Args::parse();
    
    // Process and validate the arguments
    if let Err(e) = args.validate_and_process() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
    
    args
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
    pub input_json: Option<PathBuf>,
    /// Path to the JSON file containing secret files to initialize
    #[arg(long, value_parser = check_readable_file)]
    pub secrets_init_filepath: Option<PathBuf>,
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
                    return Err("secrets_init_filepath contains invalid UTF-8 characters".to_string());
                }
            } else {
                return Err("secrets_init_filepath is required for SecretInitialize mode".to_string());
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
                return Err("secrets_init_filepath is required for SecretInitialize mode".to_string());
            }
            
            if let Some(output_path) = &self.output_json {
                check_file_writable_path(output_path)?;
            } else {
                return Err("output_json is required for SecretInitialize mode".to_string());
            }
            
            // The actual processing of secrets_init_filepath happens in validate_and_process()
        } else if let Mode::SecretUpload = self.mode {
            // Check for required fields for SecretUpload mode
            if self.input_json.is_none() {
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


/// Checks if a file is readable
///
/// # Arguments
/// 
/// * `file` - Path to check
/// 
/// # Returns
/// 
/// * `Result<PathBuf, String>` - The validated PathBuf or an error message
pub fn check_readable_file(file: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(file);
    
    let xpath = if path.starts_with("~") {
        if let Some(home) = home_dir() {
            home.join(path.strip_prefix("~").unwrap_or(path.as_path()))
        } else {
            return Err("Home directory could not be determined.".to_string());
        }
    } else {
        path
    };
    
    if xpath.is_file() && fs::metadata(&xpath).is_ok() {
        Ok(xpath)
    } else {
        Err(format!("The file '{}' is not readable.", file))
    }
}

/// Checks if a file is readable (PathBuf version)
///
/// # Arguments
/// 
/// * `file` - PathBuf to check
/// 
/// # Returns
/// 
/// * `Result<PathBuf, String>` - The validated PathBuf or an error message
pub fn check_readable_path(file: &std::path::Path) -> Result<PathBuf, String> {
    if let Some(file_str) = file.to_str() {
        check_readable_file(file_str)
    } else {
        Err("Invalid path: contains non-UTF-8 characters".to_string())
    }
}

/// Checks if a file is a valid JSON file
///
/// # Arguments
/// 
/// * `file` - Path to check
/// 
/// # Returns
/// 
/// * `Result<PathBuf, String>` - The validated PathBuf or an error message
pub fn check_valid_json_file(file: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(file);
    
    let mut file_handle = File::open(&path).map_err(|e| format!("Unable to open '{}': {}", file, e))?;
    let mut file_content = String::new();
    file_handle.read_to_string(&mut file_content)
        .map_err(|e| format!("Unable to read '{}': {}", file, e))?;
    
    let mut entries = Vec::new();
    let deserializer = serde_json::Deserializer::from_str(&file_content).into_iter::<Value>();

    for entry in deserializer {
        let entry = entry.map_err(|e| format!("Invalid JSON in '{}': {}", file, e))?;
        entries.push(entry);
    }
    Ok(path)
}

/// Checks if a PathBuf is a valid JSON file
///
/// # Arguments
/// 
/// * `file` - PathBuf to check
/// 
/// # Returns
/// 
/// * `Result<PathBuf, String>` - The validated PathBuf or an error message
pub fn check_valid_json_path(file: &std::path::Path) -> Result<PathBuf, String> {
    if let Some(file_str) = file.to_str() {
        check_valid_json_file(file_str)
    } else {
        Err("Invalid path: contains non-UTF-8 characters".to_string())
    }
}

/// Checks if a directory is readable
///
/// # Arguments
/// 
/// * `dir` - Path to check
/// 
/// # Returns
/// 
/// * `Result<PathBuf, String>` - The validated PathBuf or an error message
pub fn check_readable_dir(dir: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(dir);
    
    if path.is_dir() && fs::metadata(&path).is_ok() && fs::read_dir(&path).is_ok() {
        Ok(path)
    } else {
        Err(format!("The directory '{}' is not readable.", dir))
    }
}

/// Checks if a directory PathBuf is readable
///
/// # Arguments
/// 
/// * `dir` - PathBuf to check
/// 
/// # Returns
/// 
/// * `Result<PathBuf, String>` - The validated PathBuf or an error message
pub fn check_readable_dir_path(dir: &std::path::Path) -> Result<PathBuf, String> {
    if let Some(dir_str) = dir.to_str() {
        check_readable_dir(dir_str)
    } else {
        Err("Invalid path: contains non-UTF-8 characters".to_string())
    }
}

/// Checks if a file is valid for initialization
/// It must be readable and either valid JSON or contain a list of filenames
///
/// When processing a file containing a list of filenames, this function:
/// 1. Reads the input file
/// 2. Validates each filename
/// 3. Creates a JSON array with valid filenames
/// 4. Writes the JSON to a new file with timestamp extension
/// 5. Returns the path to the new JSON file
///
/// # Arguments
/// 
/// * `file_path` - Path to the file to check
/// 
/// # Returns
/// 
/// * `Result<PathBuf, String>` - The validated or generated new PathBuf
pub fn check_init_filepath(file_path: &str) -> Result<PathBuf, String> {
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
                // if let Some(expanded_filenm) =check_readable_file(filename).is_ok() {
                //     file_array.push(serde_json::json!({"filenm": filename}));
                // }
                match check_readable_file(filename) {
                    Ok(path) => {
                        if let Some(path_str) = path.to_str() {
                            file_array.push(serde_json::json!({"filenm": path_str}));
                        } else {
                            eprintln!("Warning: Path '{}' contains invalid UTF-8 characters, skipping", filename);
                        }
                    },
                    Err(_) => continue, // Skip invalid files
                }
            }
            
            // If no valid files found, return an error
            if file_array.is_empty() {
                return Err(format!("No readable files found in '{}'.", file_path));
            }
            
            
            let json_content = serde_json::to_string_pretty(&file_array)
                .map_err(|e| format!("Failed to convert filename list to JSON: {}", e))?;
            
            // Write back to the file with unix timestamp and .json extension
            let new_extension = format!("{}.json", chrono::Utc::now().timestamp());
            let new_file_path = path.with_extension(new_extension);
            let mut output_file = File::create(&new_file_path)
                .map_err(|e| format!("Failed to open file for writing: {}", e))?;
            
            output_file.write_all(json_content.as_bytes())
                .map_err(|e| format!("Failed to write JSON content: {}", e))?;
            
            Ok(new_file_path)
        }
    }
}

/// Checks if a file is writable (or can be created and written to)
///
/// # Arguments
/// 
/// * `file_path` - Path to check
/// 
/// # Returns
/// 
/// * `Result<PathBuf, String>` - The validated PathBuf or an error message
pub fn check_file_writable(file_path: &str) -> Result<PathBuf, String> {
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

/// Checks if a PathBuf is writable (or can be created and written to)
///
/// # Arguments
/// 
/// * `file_path` - PathBuf to check
/// 
/// # Returns
/// 
/// * `Result<PathBuf, String>` - The validated PathBuf or an error message
pub fn check_file_writable_path(file_path: &std::path::Path) -> Result<PathBuf, String> {
    if let Some(path_str) = file_path.to_str() {
        check_file_writable(path_str)
    } else {
        Err("Invalid path: contains non-UTF-8 characters".to_string())
    }
}
