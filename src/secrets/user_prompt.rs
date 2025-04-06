use crate::read_interactive_input::{GrammarFragment, GrammarType};
use crate::utils::json_utils;
use crate::secrets::error::Result;
use crate::secrets::file_details::{format_file_size, display_file_details, get_file_details};
use crate::interfaces::ReadInteractiveInputHelper;
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
            pos: (i + 4) as u8,  // Start after the filename, size, and "for" text
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
    encoded_name: &str
) -> Result<()> {
    // Calculate file size
    let metadata = std::fs::metadata(file_path)
        .map_err(|e| Box::<dyn std::error::Error>::from(format!("Failed to get metadata for {}: {}", file_path, e)))?;
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
    
    // Add encoded name
    let name_grammar = GrammarFragment {
        original_val_for_prompt: Some(encoded_name.to_string()),
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
    encoded_name: &str, 
    secret_exists: bool,
    az_created: Option<String>,
    az_updated: Option<String>,
) -> Result<bool> {
    use crate::interfaces::DefaultReadInteractiveInputHelper;
    let read_val_helper = DefaultReadInteractiveInputHelper;
    prompt_for_upload_with_helper(file_path, encoded_name, &read_val_helper, secret_exists, az_created, az_updated)
}

/// Version of prompt_for_upload that accepts dependency injection for testing
pub fn prompt_for_upload_with_helper<R: ReadInteractiveInputHelper>(
    file_path: &str, 
    encoded_name: &str, 
    read_val_helper: &R,
    secret_exists: bool,
    az_created: Option<String>,
    az_updated: Option<String>,
) -> Result<bool> {
    // If the secret exists and we're prompting, show an overwrite warning
    if secret_exists {
        println!("Warning: This will overwrite the existing secret in Azure Key Vault.");
    }

    let mut grammars: Vec<GrammarFragment> = Vec::new();
    
    // Setup the prompt
    setup_upload_prompt(&mut grammars, file_path, encoded_name)?;
    
    loop {
        // Display prompt and get user input using the provided helper
        let result = read_val_helper.read_val_from_cmd_line_and_proceed(&mut grammars, None);
        
        match result.user_entered_val {
            None => return Ok(false), // Empty input means no
            Some(choice) => {
                match choice.as_str() {
                    // Yes, upload the file
                    "Y" => {
                        return Ok(true);
                    },
                    // No, skip this file
                    "n" => {
                        return Ok(false);
                    },
                    // Display details about the file
                    "d" => {
                        // Get file details
                        let mut details = get_file_details(file_path, encoded_name)?;
                        
                        // Add Azure metadata if available
                        details.az_created = az_created.clone();
                        details.az_updated = az_updated.clone();
                        
                        // Display the details
                        display_file_details(&details);
                    },
                    // Display help
                    "?" => {
                        display_upload_help();
                    },
                    // Invalid choice
                    _ => {
                        eprintln!("Invalid choice: {}", choice);
                    }
                }
            }
        }
    }
}