use crate::read_interactive_input::{GrammarFragment, GrammarType};
use crate::secrets::error::Result;
use crate::utils::json_utils;
use serde_json::Value;
use std::path::Path;

/// Setup the prompt for retrieving and comparing secrets
pub fn setup_retrieve_prompt(grammars: &mut Vec<GrammarFragment>, entry: &Value) -> Result<()> {
    // Determine if local file exists
    let file_name = json_utils::extract_string_field(entry, "file_nm")?;
    let file_exists = Path::new(&file_name).exists();

    // Status text
    let status_text = if file_exists {
        "Files differ"
    } else {
        "File missing"
    };
    grammars.push(GrammarFragment {
        original_val_for_prompt: Some(status_text.to_string()),
        shortened_val_for_prompt: None,
        pos: 0,
        prefix: None,
        suffix: Some(" ".to_string()),
        grammar_type: GrammarType::Verbiage,
        can_shorten: false,
        display_at_all: true,
    });

    // File name
    grammars.push(GrammarFragment {
        original_val_for_prompt: Some(file_name.clone()),
        shortened_val_for_prompt: None,
        pos: 1,
        prefix: None,
        suffix: Some(". ".to_string()),
        grammar_type: GrammarType::FileName,
        can_shorten: true,
        display_at_all: true,
    });

    // Action text
    let action_text = if file_exists {
        "View diff?"
    } else {
        "Save locally?"
    };
    grammars.push(GrammarFragment {
        original_val_for_prompt: Some(action_text.to_string()),
        shortened_val_for_prompt: None,
        pos: 2,
        prefix: None,
        suffix: Some(" ".to_string()),
        grammar_type: GrammarType::Verbiage,
        can_shorten: false,
        display_at_all: true,
    });

    // Choices based on existence
    let choices: Vec<&str> = if file_exists {
        vec!["N", "y", "s", "d", "?"]
    } else {
        vec!["Y", "n", "d", "?"]
    };
    for (i, &choice) in choices.iter().enumerate() {
        let mut sep = Some("/".to_string());
        if i == choices.len() - 1 {
            sep = Some(": ".to_string());
        }
        grammars.push(GrammarFragment {
            original_val_for_prompt: Some(choice.to_string()),
            shortened_val_for_prompt: None,
            pos: (i + 3) as u8,
            prefix: None,
            suffix: sep,
            grammar_type: GrammarType::UserChoice,
            can_shorten: false,
            display_at_all: true,
        });
    }

    Ok(())
}
