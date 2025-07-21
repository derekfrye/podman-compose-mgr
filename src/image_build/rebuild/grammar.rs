use crate::read_interactive_input::{GrammarFragment, GrammarType};

use std::path::Path;
use walkdir::DirEntry;

/// Build the interactive prompt grammars for rebuild
pub fn build_rebuild_grammars(
    entry: &DirEntry,
    custom_img_nm: &str,
    container_name: &str,
) -> Vec<GrammarFragment> {
    let mut grammars: Vec<GrammarFragment> = vec![];

    let grm1 = GrammarFragment {
        original_val_for_prompt: Some("Refresh".to_string()),
        shortened_val_for_prompt: None,
        pos: 0,
        prefix: None,
        suffix: Some(" ".to_string()),
        grammar_type: GrammarType::Verbiage,
        can_shorten: false,
        display_at_all: true,
    };
    grammars.push(grm1);

    let grm2 = GrammarFragment {
        original_val_for_prompt: Some(custom_img_nm.to_string()),
        shortened_val_for_prompt: None,
        pos: 1,
        prefix: None,
        suffix: Some(" ".to_string()),
        grammar_type: GrammarType::Image,
        can_shorten: true,
        display_at_all: true,
    };
    grammars.push(grm2);

    let grm3 = GrammarFragment {
        original_val_for_prompt: Some("from".to_string()),
        shortened_val_for_prompt: None,
        pos: 2,
        prefix: None,
        suffix: Some(" ".to_string()),
        grammar_type: GrammarType::Verbiage,
        can_shorten: false,
        display_at_all: true,
    };
    grammars.push(grm3);

    let docker_compose_pth = entry
        .path()
        .parent()
        .unwrap_or_else(|| Path::new("/"))
        .display()
        .to_string();
    let grm4 = GrammarFragment {
        original_val_for_prompt: Some(docker_compose_pth),
        shortened_val_for_prompt: None,
        pos: 3,
        prefix: None,
        suffix: Some("? ".to_string()),
        grammar_type: GrammarType::DockerComposePath,
        can_shorten: true,
        display_at_all: true,
    };
    grammars.push(grm4);

    let grm5 = GrammarFragment {
        original_val_for_prompt: Some(container_name.to_string()),
        shortened_val_for_prompt: None,
        pos: 4,
        prefix: None,
        suffix: None,
        grammar_type: GrammarType::ContainerName,
        can_shorten: true,
        display_at_all: false,
    };
    grammars.push(grm5);

    let choices = ["p", "N", "d", "b", "s", "?"];
    for (i, &c) in choices.iter().enumerate() {
        let mut sep = Some("/".to_string());
        if i == choices.len() - 1 {
            sep = Some(": ".to_string());
        }
        let choice_grammar = GrammarFragment {
            original_val_for_prompt: Some(c.to_string()),
            shortened_val_for_prompt: None,
            pos: u8::try_from(i + 5).unwrap_or(255),
            prefix: None,
            suffix: sep,
            grammar_type: GrammarType::UserChoice,
            can_shorten: false,
            display_at_all: true,
        };
        grammars.push(choice_grammar);
    }

    grammars
}
