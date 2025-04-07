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
    let choices = ["d", "Y", "n", "?"];
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
pub fn setup_upload_prompt(
    grammars: &mut Vec<GrammarFragment>,
    file_path: &str,
    _encoded_name: &str, // Still keeping parameter for backward compatibility
) -> Result<()> {
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

    // Split the formatted size into value and unit (e.g., "123.45 KiB" -> "123.45", "KiB ")
    let parts: Vec<&str> = formatted_size.split_whitespace().collect();
    let size_value = parts[0];
    let size_unit = format!("{} ", parts[1]); // Add space after unit

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
        suffix: Some(size_unit),
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
        "d = Display info (file name, Azure KV name, upstream secret create date, and file name modify date)."
    );
    println!("v = Validate on-disk item matches the Azure Key Vault secret.");
    println!("a = Validate all items.");
    println!("? = Display this help.");
}

/// Display help for upload options
pub fn display_upload_help() {
    println!("Y = Upload this secret to Azure Key Vault.");
    println!("n = Skip this secret, don't upload it.");
    println!("d = Display details about the file.");
    println!("? = Display this help.");
}

/// Prompt the user for confirmation before uploading a file
///
/// This function uses the default implementation of ReadInteractiveInputHelper
pub fn prompt_for_upload(
    file_path: &str,
    secret_name: &str,
    secret_exists: bool,
    cloud_created: Option<String>,
    cloud_updated: Option<String>,
    cloud_type: Option<&str>,
) -> Result<bool> {
    use crate::interfaces::DefaultReadInteractiveInputHelper;
    let read_val_helper = DefaultReadInteractiveInputHelper;
    prompt_for_upload_with_helper(
        file_path,
        secret_name,
        &read_val_helper,
        secret_exists,
        cloud_created,
        cloud_updated,
        cloud_type,
    )
}

/// Version of prompt_for_upload that accepts dependency injection for testing
pub fn prompt_for_upload_with_helper<R: ReadInteractiveInputHelper>(
    file_path: &str,
    secret_name: &str,
    read_val_helper: &R,
    secret_exists: bool,
    cloud_created: Option<String>,
    cloud_updated: Option<String>,
    cloud_type: Option<&str>,
) -> Result<bool> {
    // If the secret exists and we're prompting, show an overwrite warning
    if secret_exists {
        // We'll show the detailed warning only if the user selects 'd'
        // The main warning was already printed in the upload.rs file
    }

    let mut grammars: Vec<GrammarFragment> = Vec::new();

    // Setup the prompt
    setup_upload_prompt(&mut grammars, file_path, secret_name)?;

    loop {
        // Display prompt and get user input using the provided helper
        let result = read_val_helper.read_val_from_cmd_line_and_proceed(&mut grammars, None);

        match result.user_entered_val {
            None => return Ok(true), // Empty input means yes (same as "Y")
            Some(choice) => {
                match choice.as_str() {
                    // Yes or empty response - upload the file
                    "Y" | "y" | "" => {
                        return Ok(true);
                    }
                    // No, skip this file
                    "n" => {
                        return Ok(false);
                    }
                    // Display details about the file
                    "d" => {
                        // Get file details
                        let mut details = get_file_details(file_path, secret_name)?;

                        // Add cloud metadata if available
                        details.cloud_created = cloud_created.clone();
                        details.cloud_updated = cloud_updated.clone();
                        if let Some(ct) = cloud_type {
                            details.cloud_type = Some(ct.to_string());
                        }

                        // Display the details
                        display_file_details(&details);
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
