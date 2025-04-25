use crate::interfaces::ReadInteractiveInputHelper;
use crate::read_interactive_input::{GrammarFragment, GrammarType};
use crate::secrets::error::Result;
use crate::secrets::file_details::{display_file_details, format_file_size, get_file_details};
use crate::utils::json_utils;
use serde_json::Value;

/// Setup the interactive prompt for validation
pub fn setup_validation_prompt(grammars: &mut Vec<GrammarFragment>, entry: &Value) -> Result<()> {
    // Add "Check" text
    let static_prompt_grammar = GrammarFragment {
        original_val_for_prompt: Some("Check".to_string()),
        shortened_val_for_prompt: None,
        pos: 0,
        prefix: None,
        suffix: Some(" ".to_string()),
        grammar_type: GrammarType::Verbiage,
        can_shorten: false,
        display_at_all: true,
    };
    grammars.push(static_prompt_grammar);

    // Extract and add file name
    let file_name = json_utils::extract_string_field(entry, "file_nm")?;
    let file_nm_grammar = GrammarFragment {
        original_val_for_prompt: Some(file_name.to_string()),
        shortened_val_for_prompt: None,
        pos: 1,
        prefix: None,
        suffix: Some("? ".to_string()),
        grammar_type: GrammarType::FileName,
        can_shorten: true,
        display_at_all: true,
    };
    grammars.push(file_nm_grammar);

    // Add choice options
    add_choice_options(grammars);

    Ok(())
}

/// Add user choice options to the prompt for validation
pub fn add_choice_options(grammars: &mut Vec<GrammarFragment>) {
    let choices = ["d", "N", "v", "a", "?"];
    for i in 0..choices.len() {
        let mut choice_separator = Some("/".to_string());
        if i == choices.len() - 1 {
            choice_separator = Some(": ".to_string());
        }
        let choice_grammar = GrammarFragment {
            original_val_for_prompt: Some(choices[i].to_string()),
            shortened_val_for_prompt: None,
            pos: (i + 2) as u8,
            prefix: None,
            suffix: choice_separator,
            grammar_type: GrammarType::UserChoice,
            can_shorten: false,
            display_at_all: true,
        };
        grammars.push(choice_grammar);
    }
}

/// Add user choice options to the prompt for upload
pub fn add_upload_choice_options(grammars: &mut Vec<GrammarFragment>) {
    let choices = ["d", "y", "N", "?"];
    for i in 0..choices.len() {
        let mut choice_separator = Some("/".to_string());
        if i == choices.len() - 1 {
            choice_separator = Some(": ".to_string());
        }
        let choice_grammar = GrammarFragment {
            original_val_for_prompt: Some(choices[i].to_string()),
            shortened_val_for_prompt: None,
            pos: (i + 4) as u8, // Start after the filename, size, and "for" text
            prefix: None,
            suffix: choice_separator,
            grammar_type: GrammarType::UserChoice,
            can_shorten: false,
            display_at_all: true,
        };
        grammars.push(choice_grammar);
    }
}

/// Setup the prompt for uploading a secret
pub fn setup_upload_prompt(grammars: &mut Vec<GrammarFragment>, file_path: &str) -> Result<()> {
    // Calculate file size
    let metadata = std::fs::metadata(file_path).map_err(|e| {
        Box::<dyn std::error::Error>::from(format!(
            "Failed to get metadata for {}: {}",
            file_path, e
        ))
    })?;
    let size_bytes = metadata.len();

    // Format file size with appropriate units
    let formatted_size = format_file_size(size_bytes);

    // Split the formatted size into value and unit (e.g., "123.45 KiB" -> "123.45", "KiB")
    let parts: Vec<&str> = formatted_size.split_whitespace().collect();
    let size_value = parts[0];
    let size_unit = parts[1].to_string(); // Just the unit without added spaces

    // Add "Upload" text
    let upload_grammar = GrammarFragment {
        original_val_for_prompt: Some("Upload".to_string()),
        shortened_val_for_prompt: None,
        pos: 0,
        prefix: None,
        suffix: Some(" ".to_string()),
        grammar_type: GrammarType::Verbiage,
        can_shorten: false,
        display_at_all: true,
    };
    grammars.push(upload_grammar);

    // Add file size with appropriate unit
    let size_grammar = GrammarFragment {
        original_val_for_prompt: Some(size_value.to_string()),
        shortened_val_for_prompt: None,
        pos: 1,
        prefix: None,
        suffix: Some(format!(" {} ", size_unit)), // Add space followed by unit and an additional space
        grammar_type: GrammarType::Verbiage,
        can_shorten: false,
        display_at_all: true,
    };
    grammars.push(size_grammar);

    // Add "for" text
    let for_grammar = GrammarFragment {
        original_val_for_prompt: Some("for".to_string()),
        shortened_val_for_prompt: None,
        pos: 2,
        prefix: None,
        suffix: Some(" ".to_string()),
        grammar_type: GrammarType::Verbiage,
        can_shorten: false,
        display_at_all: true,
    };
    grammars.push(for_grammar);

    // Add file path (instead of encoded name/hash)
    let name_grammar = GrammarFragment {
        original_val_for_prompt: Some(file_path.to_string()),
        shortened_val_for_prompt: None,
        pos: 3,
        prefix: None,
        suffix: Some("? ".to_string()),
        grammar_type: GrammarType::FileName,
        can_shorten: true,
        display_at_all: true,
    };
    grammars.push(name_grammar);

    // Add choice options
    add_upload_choice_options(grammars);

    Ok(())
}

/// Display help for validation options
pub fn display_validation_help() {
    println!("N = Do nothing, skip this secret.");
    println!(
        "d = Display info (file name, cloud storage name, upstream secret create date, and file name modify date)."
    );
    println!("v = Validate on-disk item matches the cloud storage secret.");
    println!("a = Validate all items.");
    println!("? = Display this help.");
}

/// Display help for upload options
pub fn display_upload_help() {
    println!("y = Upload this secret to cloud storage.");
    println!("N = Skip this secret, don't upload it (default).");
    println!("d = Display details about the file.");
    println!("? = Display this help.");
}

/// Setup the prompt for retrieving and comparing secrets
pub fn setup_retrieve_prompt(grammars: &mut Vec<GrammarFragment>, entry: &Value) -> Result<()> {
    // Check if the local file exists by checking the path from entry
    let file_name = json_utils::extract_string_field(entry, "file_nm")?;
    let file_exists = std::path::Path::new(&file_name).exists();
    
    // Add first text based on file existence
    let status_text = if file_exists { "Files differ" } else { "File missing" };
    let static_prompt_grammar = GrammarFragment {
        original_val_for_prompt: Some(status_text.to_string()),
        shortened_val_for_prompt: None,
        pos: 0,
        prefix: None,
        suffix: Some(" ".to_string()),
        grammar_type: GrammarType::Verbiage,
        can_shorten: false,
        display_at_all: true,
    };
    grammars.push(static_prompt_grammar);

    // Add file name
    let file_nm_grammar = GrammarFragment {
        original_val_for_prompt: Some(file_name.to_string()),
        shortened_val_for_prompt: None,
        pos: 1,
        prefix: None,
        suffix: Some(". ".to_string()),
        grammar_type: GrammarType::FileName,
        can_shorten: true,
        display_at_all: true,
    };
    grammars.push(file_nm_grammar);

    // Add appropriate action text based on file existence
    let action_text = if file_exists { "View diff?" } else { "Save locally?" };
    let action_prompt_grammar = GrammarFragment {
        original_val_for_prompt: Some(action_text.to_string()),
        shortened_val_for_prompt: None,
        pos: 2,
        prefix: None,
        suffix: Some(" ".to_string()),
        grammar_type: GrammarType::Verbiage,
        can_shorten: false,
        display_at_all: true,
    };
    grammars.push(action_prompt_grammar);

    // Add choices - if the file exists, default is "N"; if not, default is "Y"
    let choices = if file_exists {
        ["N", "y", "d", "?"]
    } else {
        ["Y", "n", "d", "?"]
    };
    
    for i in 0..choices.len() {
        let mut choice_separator = Some("/".to_string());
        if i == choices.len() - 1 {
            choice_separator = Some(": ".to_string());
        }
        let choice_grammar = GrammarFragment {
            original_val_for_prompt: Some(choices[i].to_string()),
            shortened_val_for_prompt: None,
            pos: (i + 3) as u8, // Start after file name and prompt text
            prefix: None,
            suffix: choice_separator,
            grammar_type: GrammarType::UserChoice,
            can_shorten: false,
            display_at_all: true,
        };
        grammars.push(choice_grammar);
    }

    Ok(())
}

/// Upload prompt configuration
pub struct UploadPromptConfig<'a> {
    /// Path to the file being uploaded
    pub file_path: &'a str,
    /// Whether the file already exists in the cloud
    pub secret_exists: bool,
    /// Cloud file creation timestamp
    pub cloud_created: Option<String>,
    /// Cloud file update timestamp
    pub cloud_updated: Option<String>,
    /// Type of cloud storage (azure_kv, r2, b2)
    pub cloud_type: Option<&'a str>,
    /// Size of the file in cloud storage
    pub cloud_file_size: Option<u64>,
    /// Size of the local file
    pub local_file_size: u64,
}

/// Prompt the user for confirmation before uploading a file
///
/// This function uses the default implementation of ReadInteractiveInputHelper
pub fn prompt_for_upload(config: UploadPromptConfig) -> Result<bool> {
    use crate::interfaces::DefaultReadInteractiveInputHelper;
    let read_val_helper = DefaultReadInteractiveInputHelper;
    prompt_for_upload_with_helper(&config, &read_val_helper)
}

/// Version of prompt_for_upload that accepts dependency injection for testing
pub fn prompt_for_upload_with_helper<R: ReadInteractiveInputHelper>(
    config: &UploadPromptConfig<'_>,
    read_val_helper: &R,
) -> Result<bool> {
    // If the secret exists and we're prompting, show an overwrite warning
    if config.secret_exists {
        // We'll show the detailed warning only if the user selects 'd'
        // The main warning was already printed in the upload.rs file
    }

    let mut grammars: Vec<GrammarFragment> = Vec::new();

    // Setup the prompt
    setup_upload_prompt(&mut grammars, config.file_path)?;

    loop {
        // Display prompt and get user input using the provided helper
        let result = read_val_helper.read_val_from_cmd_line_and_proceed(&mut grammars, None);

        match result.user_entered_val {
            None => return Ok(false), // Empty input means no (same as "N")
            Some(choice) => {
                match choice.as_str() {
                    // Yes - upload the file
                    "y" | "Y" => {
                        return Ok(true);
                    }
                    // No, skip this file
                    "n" | "N" | "" => {
                        return Ok(false);
                    }
                    // Display details about the file
                    "d" => {
                        // Get file details
                        let mut details = get_file_details(config.file_path)?;

                        // Add cloud metadata if available
                        details.cloud_created = config.cloud_created.clone();
                        details.cloud_updated = config.cloud_updated.clone();
                        if let Some(ct) = config.cloud_type {
                            details.cloud_type = Some(ct.to_string());
                        }

                        // Display the details
                        display_file_details(&details);

                        // If file exists in cloud and cloud_file_size is available, show the size comparison
                        if config.secret_exists
                            && config.cloud_type == Some("r2")
                            && config.cloud_file_size.is_some()
                        {
                            let cloud_size = config.cloud_file_size.unwrap();

                            // Compare file sizes and show difference
                            if cloud_size != config.local_file_size {
                                let size_diff = if cloud_size > config.local_file_size {
                                    cloud_size - config.local_file_size
                                } else {
                                    config.local_file_size - cloud_size
                                };

                                let diff_percentage =
                                    (size_diff as f64 / config.local_file_size as f64) * 100.0;

                                if cloud_size > config.local_file_size {
                                    eprintln!(
                                        "warn: Cloud file size ({}) is LARGER than local file size ({}) by {} ({:.2}%)",
                                        crate::secrets::file_details::format_file_size(cloud_size),
                                        crate::secrets::file_details::format_file_size(
                                            config.local_file_size
                                        ),
                                        crate::secrets::file_details::format_file_size(size_diff),
                                        diff_percentage
                                    );
                                } else {
                                    eprintln!(
                                        "warn: Cloud file size ({}) is SMALLER than local file size ({}) by {} ({:.2}%)",
                                        crate::secrets::file_details::format_file_size(cloud_size),
                                        crate::secrets::file_details::format_file_size(
                                            config.local_file_size
                                        ),
                                        crate::secrets::file_details::format_file_size(size_diff),
                                        diff_percentage
                                    );
                                }
                            } else {
                                println!(
                                    "File sizes match: Cloud file size equals local file size ({})",
                                    crate::secrets::file_details::format_file_size(
                                        config.local_file_size
                                    )
                                );
                            }
                        }

                        // Add an extra newline for better readability
                        println!();
                    }
                    // Display help
                    "?" => {
                        display_upload_help();
                    }
                    // Invalid choice
                    _ => {
                        eprintln!("Invalid choice: {}", choice);
                    }
                }
            }
        }
    }
}
