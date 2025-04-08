use crate::read_interactive_input::{GrammarFragment, GrammarType};
use std::path::Path;
use walkdir::DirEntry;

/// Creates a grammar fragment for a specific prompt part
pub fn create_grammar_fragment(
    text: &str, 
    pos: u8, 
    suffix: Option<String>, 
    grammar_type: GrammarType, 
    can_shorten: bool, 
    display_at_all: bool
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
    }
}

/// Creates grammar fragments for the rebuild prompt
pub fn create_rebuild_grammars(
    custom_img_nm: &str,
    entry: &DirEntry,
    container_name: &str
) -> Vec<GrammarFragment> {
    let mut grammars: Vec<GrammarFragment> = vec![];
    
    // Add "Refresh" text
    grammars.push(create_grammar_fragment(
        "Refresh", 
        0, 
        Some(" ".to_string()), 
        GrammarType::Verbiage, 
        false, 
        true
    ));
    
    // Add image name
    grammars.push(create_grammar_fragment(
        custom_img_nm, 
        1, 
        Some(" ".to_string()), 
        GrammarType::Image, 
        true, 
        true
    ));
    
    // Add "from" text
    grammars.push(create_grammar_fragment(
        "from", 
        2, 
        Some(" ".to_string()), 
        GrammarType::Verbiage, 
        false, 
        true
    ));
    
    // Get Docker compose path
    let docker_compose_pth = entry
        .path()
        .parent()
        .unwrap_or_else(|| Path::new("/"))
        .display()
        .to_string();
        
    // Add Docker compose path
    grammars.push(create_grammar_fragment(
        &docker_compose_pth, 
        3, 
        Some("? ".to_string()), 
        GrammarType::DockerComposePath, 
        true, 
        true
    ));
    
    // Add container name (hidden)
    grammars.push(create_grammar_fragment(
        container_name, 
        4, 
        None, 
        GrammarType::ContainerName, 
        true, 
        false
    ));
    
    grammars
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
            pos: (i + 5) as u8,
            prefix: None,
            suffix: choice_separator,
            grammar_type: GrammarType::UserChoice,
            can_shorten: false,
            display_at_all: true,
        };
        grammars.push(choice_grammar);
    }
}