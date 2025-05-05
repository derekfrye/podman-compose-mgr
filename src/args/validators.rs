use home::home_dir;
use hostname;
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};

use super::types::{Args, Mode};

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
pub fn check_readable_path(file: &Path) -> Result<PathBuf, String> {
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

    let mut file_handle =
        File::open(&path).map_err(|e| format!("Unable to open '{}': {}", file, e))?;
    let mut file_content = String::new();
    file_handle
        .read_to_string(&mut file_content)
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
pub fn check_valid_json_path(file: &Path) -> Result<PathBuf, String> {
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
pub fn check_readable_dir_path(dir: &Path) -> Result<PathBuf, String> {
    if let Some(dir_str) = dir.to_str() {
        check_readable_dir(dir_str)
    } else {
        Err("Invalid path: contains non-UTF-8 characters".to_string())
    }
}

/// Checks if a directory is writable, creating it if it doesn't exist.
///
/// # Arguments
///
/// * `dir` - Path to check
///
/// # Returns
///
/// * `Result<PathBuf, String>` - The validated PathBuf or an error message
pub fn check_writable_dir(dir: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(dir);

    // Resolve ~ if present
    let expanded_path = if path.starts_with("~") {
        if let Some(home) = home_dir() {
            home.join(path.strip_prefix("~").unwrap_or(path.as_path()))
        } else {
            return Err("Home directory could not be determined.".to_string());
        }
    } else {
        path
    };

    // Create the directory if it doesn't exist
    if !expanded_path.exists() {
        fs::create_dir_all(&expanded_path).map_err(|e| {
            format!(
                "Failed to create directory '{}': {}",
                expanded_path.display(),
                e
            )
        })?;
    }

    // Check if it's a directory
    if !expanded_path.is_dir() {
        return Err(format!("'{}' is not a directory.", expanded_path.display()));
    }

    // Check if it's writable by trying to create a temporary file inside it
    match tempfile::tempfile_in(&expanded_path) {
        Ok(_) => Ok(expanded_path), // Successfully created and implicitly deleted a temp file
        Err(e) => Err(format!(
            "Directory '{}' is not writable: {}",
            expanded_path.display(),
            e
        )),
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

    // Resolve ~ if present
    let expanded_path = if path.starts_with("~") {
        if let Some(home) = home_dir() {
            home.join(path.strip_prefix("~").unwrap_or(path.as_path()))
        } else {
            return Err("Home directory could not be determined.".to_string());
        }
    } else {
        path
    };

    // First check if the parent directory exists and is writable
    if let Some(parent) = expanded_path.parent() {
        if !parent.exists() {
            return Err(format!(
                "The parent directory of '{}' does not exist.",
                expanded_path.display()
            ));
        }

        if !parent.is_dir() {
            return Err(format!(
                "The parent path '{}' is not a directory.",
                parent.display()
            ));
        }
    }

    // Try to open the file in write mode
    match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false) // Don't truncate an existing file
        .open(&expanded_path)
    {
        Ok(_) => Ok(expanded_path),
        Err(e) => Err(format!(
            "The file '{}' is not writable: {}",
            expanded_path.display(),
            e
        )),
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
pub fn check_file_writable_path(file_path: &Path) -> Result<PathBuf, String> {
    if let Some(path_str) = file_path.to_str() {
        check_file_writable(path_str)
    } else {
        Err("Invalid path: contains non-UTF-8 characters".to_string())
    }
}

/// Check if cloud storage credentials are needed based on entries in the input JSON
///
/// Returns a tuple of booleans (need_b2, need_r2) indicating if credentials
/// are needed for each cloud storage type.
pub fn needs_cloud_credentials_for_upload(args: &Args) -> Result<(bool, bool), String> {
    // Get the input JSON path
    let input_json_path = match &args.input_json {
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
pub fn validate(args: &Args) -> Result<(), String> {
    if let Mode::SecretRetrieve = args.mode {
        // Client ID is already validated by check_readable_file in the argument definition

        // Client secret path is already validated by the value_parser

        if let Some(output_json) = &args.output_json {
            check_file_writable_path(output_json)?;
        }

        if let Some(input_json) = &args.input_json {
            check_valid_json_path(input_json)?;
        }
    } else if let Mode::SecretInitialize = args.mode {
        // Basic validation - make sure fields exist
        if args.secrets_init_filepath.is_none() {
            return Err(
                "secrets_init_filepath is required for SecretInitialize mode".to_string(),
            );
        }

        if let Some(output_path) = &args.output_json {
            check_file_writable_path(output_path)?;
        } else {
            return Err("output_json is required for SecretInitialize mode".to_string());
        }

        // Bucket for upload is now specified in the JSON file, no longer a command-line argument

        // The actual processing of secrets_init_filepath happens in validate_and_process()
    } else if let Mode::SecretUpload = args.mode {
        // Check for required fields for SecretUpload mode
        if args.input_json.is_none() {
            return Err("input_json is required for SecretUpload mode".to_string());
        }
        if args.output_json.is_none() {
            return Err("output_json is required for SecretUpload mode".to_string());
        }

        // Check for Azure KeyVault credentials
        if args.azure_vault_name_path.is_none() {
            return Err("secrets_vault_name is required for SecretUpload mode".to_string());
        }
        if args.azure_tenant_id_path.is_none() {
            return Err("secrets_tenant_id is required for SecretUpload mode".to_string());
        }
        if args.azure_client_secret_path.is_none() {
            return Err(
                "secrets_client_secret_path is required for SecretUpload mode".to_string(),
            );
        }
        if args.azure_client_id_path.is_none() {
            return Err("secrets_client_id is required for SecretUpload mode".to_string());
        }

        // Validate the file paths
        if let Some(input_json) = &args.input_json {
            check_valid_json_path(input_json)?;

            // Check if cloud credentials are needed for any entries in the input JSON
            let (need_b2_credentials, need_r2_credentials) =
                needs_cloud_credentials_for_upload(args)?;

            // Handle both B2 and R2 credentials with shared parameters
            if need_b2_credentials || need_r2_credentials {
                // Check if s3_account_id_filepath is provided
                if args.s3_account_id_filepath.is_none() {
                    return Err("s3_account_id_filepath is required for upload mode when input json contains B2 or R2 entries".to_string());
                }

                // Check if s3_secret_key_filepath is provided
                if args.s3_secret_key_filepath.is_none() {
                    return Err("s3_secret_key_filepath is required for upload mode when input json contains B2 or R2 entries".to_string());
                }

                // For R2, we need to check if both access key ID and endpoint (account ID) are provided
                if need_r2_credentials {
                    if args.s3_account_id_filepath.is_none() {
                        return Err("s3_account_id_filepath is required for upload mode with R2 entries".to_string());
                    }

                    if args.s3_endpoint_filepath.is_none() {
                        return Err(
                            "s3_endpoint_filepath is required for upload mode with R2 entries"
                                .to_string(),
                        );
                    }

                    // All filepaths are already validated by the value_parser
                }
            }
        }

        // Client secret path is already validated by the value_parser

        if let Some(output_json) = &args.output_json {
            check_file_writable_path(output_json)?;
        }
    } else if let Mode::SecretMigrate = args.mode {
        // Check for required fields for SecretMigrate mode
        if args.input_json.is_none() {
            return Err("input_json is required for SecretMigrate mode".to_string());
        }
        if args.output_json.is_none() {
            return Err("output_json is required for SecretMigrate mode".to_string());
        }

        // Check for Azure KeyVault credentials
        if args.azure_vault_name_path.is_none() {
            return Err("azure_vault_name_path is required for SecretMigrate mode".to_string());
        }
        if args.azure_tenant_id_path.is_none() {
            return Err("azure_tenant_id_path is required for SecretMigrate mode".to_string());
        }
        if args.azure_client_secret_path.is_none() {
            return Err("azure_client_secret_path is required for SecretMigrate mode".to_string());
        }
        if args.azure_client_id_path.is_none() {
            return Err("azure_client_id_path is required for SecretMigrate mode".to_string());
        }

        // Check for S3 credentials
        if args.s3_account_id_filepath.is_none() {
            return Err("s3_account_id_filepath is required for SecretMigrate mode".to_string());
        }
        if args.s3_secret_key_filepath.is_none() {
            return Err("s3_secret_key_filepath is required for SecretMigrate mode".to_string());
        }
        if args.s3_endpoint_filepath.is_none() {
            return Err("s3_endpoint_filepath is required for SecretMigrate mode".to_string());
        }

        // Validate the file paths
        if let Some(input_json) = &args.input_json {
            check_valid_json_path(input_json)?;
        }

        if let Some(output_json) = &args.output_json {
            check_file_writable_path(output_json)?;
        }
    }
    Ok(())
}
