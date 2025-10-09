use crate::read_interactive_input::{GrammarFragment, GrammarType};
use std::path::Path;
use walkdir::DirEntry;

/// Creates a grammar fragment for a specific prompt part
#[must_use]
pub fn create_grammar_fragment(
    text: &str,
    pos: u8,
    suffix: Option<String>,
    grammar_type: GrammarType,
    can_shorten: bool,
    display_at_all: bool,
) -> GrammarFragment {
    GrammarFragment {
        original_val_for_prompt: Some(text.to_string()),
        shortened_val_for_prompt: None,
        pos,
        prefix: None,
        suffix,
        grammar_type,
        can_shorten,
        display_at_all,
        is_default_choice: false,
    }
}

/// Creates grammar fragments for the rebuild prompt
#[must_use]
pub fn create_rebuild_grammars(
    custom_img_nm: &str,
    entry: &DirEntry,
    container_name: &str,
) -> Vec<GrammarFragment> {
    let docker_compose_pth = entry
        .path()
        .parent()
        .unwrap_or_else(|| Path::new("/"))
        .display()
        .to_string();
    vec![
        rebuild_fragment("Refresh", 0, Some(" "), GrammarType::Verbiage, false, true),
        rebuild_fragment(custom_img_nm, 1, Some(" "), GrammarType::Image, true, true),
        rebuild_fragment("from", 2, Some(" "), GrammarType::Verbiage, false, true),
        rebuild_fragment(
            &docker_compose_pth,
            3,
            Some("? "),
            GrammarType::DockerComposePath,
            true,
            true,
        ),
        rebuild_fragment(
            container_name,
            4,
            None,
            GrammarType::ContainerName,
            true,
            false,
        ),
    ]
}

fn rebuild_fragment(
    text: &str,
    pos: u8,
    suffix: Option<&str>,
    grammar_type: GrammarType,
    can_shorten: bool,
    display_at_all: bool,
) -> GrammarFragment {
    create_grammar_fragment(
        text,
        pos,
        suffix.map(std::string::ToString::to_string),
        grammar_type,
        can_shorten,
        display_at_all,
    )
}

/// Add user choice options to the grammar fragments
pub fn add_choice_options(grammars: &mut Vec<GrammarFragment>) {
    let choices = ["p", "N", "d", "b", "s", "?"];
    for i in 0..choices.len() {
        let mut choice_separator = Some("/".to_string());
        if i == choices.len() - 1 {
            choice_separator = Some(": ".to_string());
        }
        let choice_grammar = GrammarFragment {
            original_val_for_prompt: Some(choices[i].to_string()),
            shortened_val_for_prompt: None,
            pos: u8::try_from(i + 5).unwrap_or(255),
            prefix: None,
            suffix: choice_separator,
            grammar_type: GrammarType::UserChoice,
            can_shorten: false,
            display_at_all: true,
            is_default_choice: choices[i] == "N",
        };
        grammars.push(choice_grammar);
    }
}
