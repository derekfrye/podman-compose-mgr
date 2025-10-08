use crate::read_interactive_input::{GrammarFragment, GrammarType};

/// Create grammar fragments for choice options
#[must_use]
pub fn make_choice_grammar(
    user_choices: &[&str],
    pos_to_start_from: u8,
    default_choice: Option<&str>,
) -> Vec<GrammarFragment> {
    let mut new_prompt_grammars = vec![];
    for i in 0..user_choices.len() {
        let mut choice_separator = Some("/".to_string());
        if i == user_choices.len() - 1 {
            choice_separator = Some(": ".to_string());
        }
        let choice_grammar = GrammarFragment {
            original_val_for_prompt: Some(user_choices[i].to_string()),
            shortened_val_for_prompt: None,
            pos: u8::try_from(i + (pos_to_start_from as usize)).unwrap_or(255),
            prefix: None,
            suffix: choice_separator,
            grammar_type: GrammarType::UserChoice,
            can_shorten: false,
            display_at_all: true,
            is_default_choice: default_choice
                .map(|default| default.eq_ignore_ascii_case(user_choices[i]))
                .unwrap_or(false),
        };
        new_prompt_grammars.push(choice_grammar);
    }
    new_prompt_grammars
}
