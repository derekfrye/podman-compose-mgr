use super::super::types::BuildFile;
use super::grammar::{make_build_prompt_grammar, make_choice_grammar};
use crate::read_interactive_input::GrammarFragment;

/// Build the interactive prompt grammars for buildfile selection
pub fn buildfile_prompt_grammars(files: &[BuildFile]) -> (Vec<GrammarFragment>, Vec<&'static str>) {
    let mut prompt_grammars: Vec<GrammarFragment> = vec![];
    let mut user_choices: Vec<&str> = vec![];

    let buildfile = files[0].clone();
    let multiple = files.iter().filter(|x| x.filepath.is_some()).count() > 1;

    if multiple {
        prompt_grammars.push(GrammarFragment {
            original_val_for_prompt: Some("Prefer Dockerfile or Makefile?".to_string()),
            ..Default::default()
        });
        user_choices = vec!["D", "M", "d", "?"];
        prompt_grammars.extend(make_choice_grammar(&user_choices, 1));
    } else if buildfile.link_target_dir.is_some() {
        prompt_grammars = make_build_prompt_grammar(&buildfile);
        user_choices = vec!["1", "2", "d", "?"];
        prompt_grammars.extend(make_choice_grammar(
            &user_choices,
            u8::try_from(prompt_grammars.len()).unwrap_or(255),
        ));
    }

    (prompt_grammars, user_choices)
}
