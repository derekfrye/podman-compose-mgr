use crate::args::Args;
use crate::read_interactive_input::{self as read_val, GrammarFragment};
use crate::secrets::error::Result;
use crate::secrets::file_details::{format_file_size, get_file_details};
use crate::secrets::user_prompt::setup_retrieve_prompt;
use crate::utils::cmd_utils;
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::{Builder as TempFileBuilder, NamedTempFile};
use base64::engine::general_purpose::STANDARD;
use base64::Engine; // Import the trait so we can use decode() method
use std::fs::create_dir_all;

/// Prompt the user to see differences or details, or save file if it doesn't exist locally
///
/// Returns Some(true) if the user wants to continue, Some(false) if they want to skip,
/// or None if they've chosen an option that doesn't affect the validation process
pub fn prompt_for_diff_or_save(
    downloaded_path: &str,
    local_path: &str,
    entry: &Value,
    args: &Args,
) -> Result<Option<bool>> {
    // Create grammarFragments for the prompt
    let mut grammars: Vec<GrammarFragment> = vec![];

    // Set up the prompt with file name
    setup_retrieve_prompt(&mut grammars, entry)?;

    // Check if the local file exists
    let file_exists = std::path::Path::new(local_path).exists();

    loop {
        // Display prompt and get user input
        let result = read_val::read_val_from_cmd_line_and_proceed_default(&mut grammars);

        match result.user_entered_val {
            None => {
                // Empty input (default to N for existing files, Y for missing files)
                if file_exists {
                    return Ok(Some(false)); // Skip
                } else {
                    // Save the file locally
                    println!("Saving file locally to {}", local_path);
                    save_file_locally(downloaded_path, local_path, entry)?;
                    return Ok(Some(true));
                }
            }
            Some(user_choice) => {
                match user_choice.as_str() {
                    "N" | "n" => {
                        // Skip this file
                        return Ok(Some(false));
                    }
                    "Y" | "y" => {
                        if file_exists {
                            // Show diff for existing files
                            println!("Showing differences...");
                            view_diff(downloaded_path, local_path, args)?;
                        } else {
                            // Save the file for missing files
                            println!("Saving file locally to {}", local_path);
                            save_file_locally(downloaded_path, local_path, entry)?;
                        }
                        // Continue with validation after showing diff or saving
                        return Ok(Some(true));
                    }
                    "d" => {
                        // Display details
                        show_file_details(entry, local_path)?;
                        // Don't affect validation, continue the loop
                        continue;
                    }
                    "?" => {
                        // Display help with file_exists context
                        display_retrieve_help(file_exists);
                        // Don't affect validation, continue the loop
                        continue;
                    }
                    _ => {
                        // Invalid choice
                        eprintln!("Invalid choice: {}", user_choice);
                        // Don't affect validation, continue the loop
                        continue;
                    }
                }
            }
        }
    }
}

/// Show diff between two files using the 'diff' command
fn view_diff(
    file1_path: &str,
    file2_path: &str,
    args: &Args,
) -> Result<()> {
    let path1_to_diff;
    let path2_to_diff;
    let _temp1;
    let _temp2;

    // Handle potential base64 encoding for file1 (downloaded)
    if file1_path.ends_with(".base64") {
        let content = fs::read(file1_path)?;
        let mut decoded_file = TempFileBuilder::new()
            .prefix("diff1_")
            .suffix(".tmp")
            .tempfile_in(&args.temp_file_path)?;
        let decoded_data = STANDARD.decode(&content)?;
        decoded_file.write_all(&decoded_data)?;
        path1_to_diff = decoded_file.path().to_string_lossy().to_string();
        _temp1 = Some(decoded_file);
    } else {
        path1_to_diff = file1_path.to_string();
        _temp1 = None;
    }

    // Handle potential base64 encoding for file2 (local)
    if file2_path.ends_with(".base64") {
        let content = fs::read(file2_path)?;
        let mut decoded_file = TempFileBuilder::new()
            .prefix("diff2_")
            .suffix(".tmp")
            .tempfile_in(&args.temp_file_path)?;
        let decoded_data = STANDARD.decode(&content)?;
        decoded_file.write_all(&decoded_data)?;
        path2_to_diff = decoded_file.path().to_string_lossy().to_string();
        _temp2 = Some(decoded_file);
    } else {
        path2_to_diff = file2_path.to_string();
        _temp2 = None;
    }

    // Check if the diff command is available
    let diff_cmd = "diff";
    let pager_cmd = get_default_pager();

    // Print a heading to explain what the diff shows
    println!("\nDifferences between cloud version and local file:");
    println!("< {} (cloud version)", file1_path);
    println!("> {} (local version)\n", file2_path);

    // Decide on options for diff - use unified diff format for better readability
    let diff_args = ["-u", &path1_to_diff, &path2_to_diff];

    // Try to run the diff command, but handle the case where files are identical or diff returns error
    let output = match cmd_utils::run_command_with_output(diff_cmd, &diff_args) {
        Ok(output) => {
            if output.trim().is_empty() {
                // Empty output usually means no differences
                println!("No differences found between files.");
                return Ok(());
            }
            output
        }
        Err(e) => {
            // Handle special case where diff returns non-zero exit code for differences
            // This is normal behavior for diff, but will cause run_command_with_output to return an error
            if e.to_string().contains("Command 'diff' failed") {
                // Try to get output even if the command "failed" (which is actually normal for diff)
                match cmd_utils::run_command(diff_cmd, &diff_args) {
                    Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
                    Err(_) => {
                        // If we couldn't get the output, show the original error
                        eprintln!("Error running diff: {}", e);
                        return Ok(());
                    }
                }
            } else {
                // Other kind of error
                eprintln!("Error running diff: {}", e);
                return Ok(());
            }
        }
    };

    // Execute the diff command using a pager if possible
    if !pager_cmd.is_empty() {
        // Write the output to a temp file for paging
        let mut temp_file = NamedTempFile::new().map_err(|e| {
            Box::<dyn std::error::Error>::from(format!("Failed to create temporary file: {}", e))
        })?;
        temp_file.write_all(output.as_bytes()).map_err(|e| {
            Box::<dyn std::error::Error>::from(format!("Failed to write diff output: {}", e))
        })?;

        // Run pager on the temp file - convert path to string and store it
        let temp_path_str = temp_file.path().to_string_lossy().to_string();
        let pager_args = [&temp_path_str[..]];
        if let Err(e) = cmd_utils::exec_cmd(&pager_cmd, &pager_args) {
            // If pager fails, just print the output directly
            println!("{}", output);
            eprintln!("Warning: Failed to use pager: {}", e);
        }
    } else {
        // Just run diff command directly
        println!("{}", output);
    }

    Ok(())
}

/// Get the default system pager
pub fn get_default_pager() -> String {
    // Check the PAGER environment variable
    if let Ok(pager) = std::env::var("PAGER") {
        if !pager.is_empty() {
            return pager;
        }
    }

    // Try common pagers in order of preference
    for pager in ["less", "more", "most", "cat"] {
        // Check if the pager exists and is executable
        if let Ok(output) = cmd_utils::run_command_with_output("which", &[pager]) {
            if !output.trim().is_empty() {
                return pager.to_string();
            }
        }
    }

    // Default to empty string if no pager found
    String::new()
}

/// Display detailed file information
pub fn show_file_details(entry: &Value, local_path: &str) -> Result<()> {
    // Get file details from the entry
    let file_nm = entry["file_nm"].as_str().unwrap_or("Unknown");
    let cloud_cr_ts = entry["cloud_cr_ts"].as_str().unwrap_or("Unknown");
    let cloud_upd_ts = entry["cloud_upd_ts"].as_str().unwrap_or("Unknown");
    let file_size_str = entry["file_size"].as_str().unwrap_or("0");
    let file_size: u64 = file_size_str.parse().unwrap_or(0);

    // Get storage type and hash
    let storage_type = entry["destination_cloud"]
        .as_str()
        .or_else(|| entry["cloud_type"].as_str())
        .unwrap_or("azure_kv");

    let hash = entry["hash"].as_str().unwrap_or("Unknown");
    let hash_algo = entry["hash_algo"].as_str().unwrap_or("sha1");

    // Print file details from the entry
    println!("\nFile details from storage record:");
    println!("Path: {}", file_nm);
    println!("Storage type: {}", storage_type);

    // For R2/B2 storage, show bucket name
    if storage_type == "r2" || storage_type == "b2" {
        if let Some(bucket) = entry["cloud_upload_bucket"].as_str() {
            println!("Bucket: {}", bucket);
        }

        // Use r2_name if available, otherwise use hash
        if let Some(r2_name) = entry["r2_name"].as_str() {
            println!("Object key: secrets/{}", r2_name);
        } else {
            println!("Object key: secrets/{}", hash);
        }
    }

    println!("Cloud created: {}", cloud_cr_ts);
    println!("Cloud updated: {}", cloud_upd_ts);
    println!("Size in storage: {}", format_file_size(file_size));
    println!("Hash ({}): {}", hash_algo, hash);

    // Get local file details if it exists
    if Path::new(local_path).exists() {
        println!("\nLocal file details:");
        match get_file_details(local_path) {
            Ok(details) => {
                println!("Path: {}", local_path);
                println!("Last modified: {}", details.last_modified);
                println!("Size on disk: {}", format_file_size(details.file_size));

                // Show encoding details
                println!("Encoding: {}", details.encoding);
                if details.encoding == "base64" {
                    println!("Encoded size: {}", format_file_size(details.encoded_size));
                }

                // Show hash (if different from cloud hash)
                if details.hash != hash {
                    println!("Local hash ({}): {}", details.hash_algo, details.hash);
                    println!("Warning: Local hash differs from cloud hash!");
                }
            }
            Err(e) => {
                eprintln!("Error getting local file details: {}", e);
            }
        }
    } else {
        println!("\nLocal file does not exist: {}", local_path);
    }

    println!(); // Extra newline for readability
    Ok(())
}

/// Function to save a file from the downloaded temp path to the local path
/// 
/// This handles creating parent directories if needed and copies the file
fn save_file_locally(source_path: &str, destination_path: &str, entry: &Value) -> Result<()> {
    // Create parent directories if they don't exist
    if let Some(parent) = Path::new(destination_path).parent() {
        if !parent.exists() {
            create_dir_all(parent).map_err(|e| {
                Box::<dyn std::error::Error>::from(format!(
                    "Failed to create directory '{}': {}",
                    parent.display(),
                    e
                ))
            })?;
        }
    }

    // Handle potential base64 encoding
    if entry["encoding"].as_str().unwrap_or("") == "base64" || source_path.ends_with(".base64") || destination_path.ends_with(".base64") {
        // Read base64 content
        let content = fs::read(source_path)?;
        // If destination should be base64
        if destination_path.ends_with(".base64") {
            fs::copy(source_path, destination_path)?;
        } else {
            // Decode base64 content
            let decoded_data = STANDARD.decode(&content)?;
            // Write decoded content to destination
            fs::write(destination_path, decoded_data)?;
        }
    } else {
        // Simple file copy for non-base64 files
        fs::copy(source_path, destination_path)?;
    }

    println!("Successfully saved file from cloud to: {}", destination_path);
    Ok(())
}

/// Display help for the retrieve options based on whether
/// we're looking at an existing file or a missing file
/// 
/// # Arguments
/// 
/// * `file_exists` - Boolean indicating if the file exists locally
pub fn display_retrieve_help(file_exists: bool) {

    if file_exists {
        println!("N = Do nothing, skip this file. (default)");
        println!("y = Show diff between the cloud version and local file.");
        println!("d = Display detailed information about the file (creation dates, sizes, etc.)");
        println!("? = Display this help.");
    } else {
        println!("Y = Save the file from cloud storage to the local path. (default)");
        println!("n = Skip this file, don't save it locally.");
        println!("d = Display detailed information about the file (creation dates, sizes, etc.)");
        println!("? = Display this help.");
    }
}
