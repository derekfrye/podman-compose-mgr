use crate::interfaces::CommandHelper;
use crate::read_interactive_input::StdinHelperWrapper;
use crate::read_interactive_input::format::{do_prompt_formatting, unroll_grammar_into_string};
use crate::read_interactive_input::types::{GrammarFragment, GrammarType};
use reedline::Reedline;
use std::collections::HashSet;

pub struct PromptData {
    pub prompt_string: String,
    pub user_choices: HashSet<String>,
    pub default_choice: Option<String>,
}

pub fn build_prompt_data<C: CommandHelper>(
    grammars: &mut [GrammarFragment],
    cmd_helper: &C,
    size: Option<usize>,
) -> PromptData {
    let term_width = cmd_helper.get_terminal_display_width(size);
    do_prompt_formatting(grammars, term_width);

    let prompt_string = unroll_grammar_into_string(grammars, false, true);
    let user_choices = collect_user_choices(grammars);
    let default_choice = grammars
        .iter()
        .find(|g| g.grammar_type == GrammarType::UserChoice && g.is_default_choice)
        .and_then(|g| g.original_val_for_prompt.clone());

    PromptData {
        prompt_string,
        user_choices,
        default_choice,
    }
}

pub fn resolve_stdin_wrapper<'a>(
    stdin_helper: Option<&'a StdinHelperWrapper>,
    default_wrapper: &'a StdinHelperWrapper,
) -> &'a StdinHelperWrapper {
    stdin_helper.unwrap_or(default_wrapper)
}

pub fn create_reedline_editor(stdin_helper: Option<&StdinHelperWrapper>) -> Option<Reedline> {
    if stdin_helper.is_none() {
        Some(Reedline::create())
    } else {
        None
    }
}

fn collect_user_choices(grammars: &[GrammarFragment]) -> HashSet<String> {
    grammars
        .iter()
        .filter(|x| x.grammar_type == GrammarType::UserChoice)
        .map(|x| x.original_val_for_prompt.clone().unwrap())
        .collect()
}
