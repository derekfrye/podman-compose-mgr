use crate::read_interactive_input::{GrammarFragment, GrammarType};

use std::path::Path;
use walkdir::DirEntry;

/// Build the interactive prompt grammars for rebuild
#[must_use]
pub fn build_rebuild_grammars(
    entry: &DirEntry,
    custom_img_nm: &str,
    container_name: &str,
) -> Vec<GrammarFragment> {
    let docker_compose_pth = entry
        .path()
        .parent()
        .unwrap_or_else(|| Path::new("/"))
        .display()
        .to_string();
    let mut grammars = vec![
        make_fragment("Refresh", 0, GrammarType::Verbiage, Some(" "), false, true, false),
        make_fragment(
            custom_img_nm,
            1,
            GrammarType::Image,
            Some(" "),
            true,
            true,
            false,
        ),
        make_fragment("from", 2, GrammarType::Verbiage, Some(" "), false, true, false),
        make_fragment(
            &docker_compose_pth,
            3,
            GrammarType::DockerComposePath,
            Some("? "),
            true,
            true,
            false,
        ),
        make_fragment(
            container_name,
            4,
            GrammarType::ContainerName,
            None,
            true,
            false,
            false,
        ),
    ];

    grammars.extend(make_choice_fragments());
    grammars
}

fn make_fragment(
    value: &str,
    pos: u8,
    grammar_type: GrammarType,
    suffix: Option<&str>,
    can_shorten: bool,
    display_at_all: bool,
    is_default_choice: bool,
) -> GrammarFragment {
    GrammarFragment {
        original_val_for_prompt: Some(value.to_string()),
        shortened_val_for_prompt: None,
        pos,
        prefix: None,
        suffix: suffix.map(std::string::ToString::to_string),
        grammar_type,
        can_shorten,
        display_at_all,
        is_default_choice,
    }
}

fn make_choice_fragments() -> Vec<GrammarFragment> {
    let choices = ["p", "N", "d", "b", "s", "?"];
    choices
        .iter()
        .enumerate()
        .map(|(idx, &choice)| {
            let suffix = if idx == choices.len() - 1 {
                Some(": ")
            } else {
                Some("/")
            };

            make_fragment(
                choice,
                u8::try_from(idx + 5).unwrap_or(255),
                GrammarType::UserChoice,
                suffix,
                false,
                true,
                choice == "N",
            )
        })
        .collect()
}
