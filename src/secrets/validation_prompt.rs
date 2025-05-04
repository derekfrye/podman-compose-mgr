use crate::read_interactive_input::{GrammarFragment, GrammarType};
use crate::secrets::error::Result;
use crate::utils::json_utils;
use serde_json::Value;

/// Setup the interactive prompt for validation
pub fn setup_validation_prompt(grammars: &mut Vec<GrammarFragment>, entry: &Value) -> Result<()> {
    // Add "Check" text
    grammars.push(GrammarFragment {
        original_val_for_prompt: Some("Check".to_string()),
        shortened_val_for_prompt: None,
        pos: 0,
        prefix: None,
        suffix: Some(" ".to_string()),
        grammar_type: GrammarType::Verbiage,
        can_shorten: false,
        display_at_all: true,
    });

    // Extract and add file name
    let file_name = json_utils::extract_string_field(entry, "file_nm")?;
    grammars.push(GrammarFragment {
        original_val_for_prompt: Some(file_name.to_string()),
        shortened_val_for_prompt: None,
        pos: 1,
        prefix: None,
        suffix: Some("? ".to_string()),
        grammar_type: GrammarType::FileName,
        can_shorten: true,
        display_at_all: true,
    });

    // Add choice options
    add_choice_options(grammars);

    Ok(())
}

/// Add user choice options to the prompt for validation
pub fn add_choice_options(grammars: &mut Vec<GrammarFragment>) {
    let choices = ["d", "N", "v", "a", "?"];
    for i in 0..choices.len() {
        let mut sep = Some("/".to_string());
        if i == choices.len() - 1 {
            sep = Some(": ".to_string());
        }
        grammars.push(GrammarFragment {
            original_val_for_prompt: Some(choices[i].to_string()),
            shortened_val_for_prompt: None,
            pos: (i + 2) as u8,
            prefix: None,
            suffix: sep,
            grammar_type: GrammarType::UserChoice,
            can_shorten: false,
            display_at_all: true,
        });
    }
}

/// Display help for validation options
pub fn display_validation_help() {
    println!("N = Do nothing, skip this secret.");
    println!("d = Display info (file name, cloud storage name, upstream secret create date, and file name modify date).");
    println!("v = Validate on-disk item matches the cloud storage secret.");
    println!("a = Validate all items.");
    println!("? = Display this help.");
}