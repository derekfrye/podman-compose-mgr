use crate::read_interactive_input::{GrammarFragment, GrammarType};
use crate::secrets::error::Result;
use crate::utils::json_utils;
use crate::interfaces::ReadInteractiveInputHelper;
use crate::secrets::file_details::{display_file_details, format_file_size, get_file_details};
use serde_json::Value;

/// Add user choice options to the prompt for upload
pub fn add_upload_choice_options(grammars: &mut Vec<GrammarFragment>) {
    let choices = ["d", "y", "N", "?"];
    for i in 0..choices.len() {
        let mut sep = Some("/".to_string());
        if i == choices.len() - 1 {
            sep = Some(": ".to_string());
        }
        grammars.push(GrammarFragment {
            original_val_for_prompt: Some(choices[i].to_string()),
            shortened_val_for_prompt: None,
            pos: (i + 4) as u8,
            prefix: None,
            suffix: sep,
            grammar_type: GrammarType::UserChoice,
            can_shorten: false,
            display_at_all: true,
        });
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
    let parts: Vec<&str> = formatted_size.split_whitespace().collect();
    let size_value = parts[0];
    let size_unit = parts[1].to_string();

    // Add "Upload" text
    grammars.push(GrammarFragment {
        original_val_for_prompt: Some("Upload".to_string()),
        shortened_val_for_prompt: None,
        pos: 0,
        prefix: None,
        suffix: Some(" ".to_string()),
        grammar_type: GrammarType::Verbiage,
        can_shorten: false,
        display_at_all: true,
    });

    // Add file size with unit
    grammars.push(GrammarFragment {
        original_val_for_prompt: Some(size_value.to_string()),
        shortened_val_for_prompt: None,
        pos: 1,
        prefix: None,
        suffix: Some(format!(" {} ", size_unit)),
        grammar_type: GrammarType::Verbiage,
        can_shorten: false,
        display_at_all: true,
    });

    // Add "for" text
    grammars.push(GrammarFragment {
        original_val_for_prompt: Some("for".to_string()),
        shortened_val_for_prompt: None,
        pos: 2,
        prefix: None,
        suffix: Some(" ".to_string()),
        grammar_type: GrammarType::Verbiage,
        can_shorten: false,
        display_at_all: true,
    });

    // Add file path
    grammars.push(GrammarFragment {
        original_val_for_prompt: Some(file_path.to_string()),
        shortened_val_for_prompt: None,
        pos: 3,
        prefix: None,
        suffix: Some("? ".to_string()),
        grammar_type: GrammarType::FileName,
        can_shorten: true,
        display_at_all: true,
    });

    // Add upload choices
    add_upload_choice_options(grammars);

    Ok(())
}

/// Display help for upload options
pub fn display_upload_help() {
    println!("y = Upload this secret to cloud storage.");
    println!("N = Skip this secret, don't upload it (default).");
    println!("d = Display details about the file.");
    println!("? = Display this help.");
}

/// Configuration for upload prompt
pub struct UploadPromptConfig<'a> {
    pub file_path: &'a str,
    pub secret_exists: bool,
    pub cloud_created: Option<String>,
    pub cloud_updated: Option<String>,
    pub cloud_type: Option<&'a str>,
    pub cloud_file_size: Option<u64>,
    pub local_file_size: u64,
}

/// Prompt the user for confirmation before uploading a file
pub fn prompt_for_upload(config: UploadPromptConfig) -> Result<bool> {
    use crate::interfaces::DefaultReadInteractiveInputHelper;
    let read_val_helper = DefaultReadInteractiveInputHelper;
    prompt_for_upload_with_helper(&config, &read_val_helper)
}

/// Prompt for upload with dependency injection
pub fn prompt_for_upload_with_helper<R: ReadInteractiveInputHelper>(
    config: &UploadPromptConfig<'_>,
    read_val_helper: &R,
) -> Result<bool> {
    // If secret exists, warning already printed
    let mut grammars = Vec::new();
    setup_upload_prompt(&mut grammars, config.file_path)?;

    loop {
        let result = read_val_helper.read_val_from_cmd_line_and_proceed(&mut grammars, None);
        match result.user_entered_val {
            None => return Ok(false),
            Some(choice) => match choice.as_str() {
                "y" | "Y" => return Ok(true),
                "n" | "N" | "" => return Ok(false),
                "d" => {
                    let mut details = get_file_details(config.file_path)?;
                    details.cloud_created = config.cloud_created.clone();
                    details.cloud_updated = config.cloud_updated.clone();
                    if let Some(ct) = config.cloud_type {
                        details.cloud_type = Some(ct.to_string());
                    }
                    display_file_details(&details);
                    println!();
                }
                "?" => display_upload_help(),
                _ => eprintln!("Invalid choice: {}", choice),
            },
        }
    }
}