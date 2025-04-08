use crate::args::Args;
use crate::read_interactive_input::{self as read_val, GrammarFragment};
use crate::secrets::error::Result;
use crate::secrets::file_details::{format_file_size, get_file_details};
use crate::secrets::user_prompt::setup_retrieve_prompt;
use crate::utils::cmd_utils;
use serde_json::Value;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

/// Prompt the user to see differences or details
///
/// Returns Some(true) if the user wants to continue, Some(false) if they want to skip,
/// or None if they've chosen an option that doesn't affect the validation process
pub fn prompt_for_diff(
    downloaded_path: &str,
    local_path: &str,
    entry: &Value,
    _args: &Args, // Not used but kept for clarity
) -> Result<Option<bool>> {
    // Create grammarFragments for the prompt
    let mut grammars: Vec<GrammarFragment> = vec![];

    // Set up the prompt with file name
    setup_retrieve_prompt(&mut grammars, entry)?;

    loop {
        // Display prompt and get user input
        let result = read_val::read_val_from_cmd_line_and_proceed_default(&mut grammars);

        match result.user_entered_val {
            None => {
                // Empty input (default to N)
                return Ok(Some(false));
            }
            Some(user_choice) => {
                match user_choice.as_str() {
                    "N" | "n" => {
                        // Skip this file
                        return Ok(Some(false));
                    }
                    "y" => {
                        // Show diff
                        show_diff(downloaded_path, local_path)?;
                        // Continue with validation after showing diff
                        return Ok(Some(true));
                    }
                    "d" => {
                        // Display details
                        show_file_details(entry, local_path)?;
                        // Don't affect validation, continue the loop
                        continue;
                    }
                    "?" => {
                        // Display help
                        display_retrieve_help();
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

/// Show differences between downloaded and local file using diff and pager
pub fn show_diff(downloaded_path: &str, local_path: &str) -> Result<()> {
    // Check if the diff command is available
    let diff_cmd = "diff";
    let pager_cmd = get_default_pager();

    // Print a heading to explain what the diff shows
    println!("\nDifferences between cloud version and local file:");
    println!("< {} (cloud version)", downloaded_path);
    println!("> {} (local version)\n", local_path);

    // Decide on options for diff - use unified diff format for better readability
    let diff_args = ["-u", downloaded_path, local_path];

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

/// Display help for the retrieve options
pub fn display_retrieve_help() {
    println!("N = Do nothing, skip this file. (default)");
    println!("y = Show diff between the cloud version and local file.");
    println!("d = Display detailed information about the file (creation dates, sizes, etc.)");
    println!("? = Display this help.");
}
