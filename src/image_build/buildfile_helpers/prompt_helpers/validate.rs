use super::format::{handle_display_info, make_build_prompt_grammar, setup_prompts};
use crate::image_build::buildfile_types::{BuildChoice, BuildFile, WhatWereBuilding};
use crate::read_interactive_input::{GrammarFragment, GrammarType};

/// Handle file type choice
///
/// # Panics
/// Panics if no file matches the chosen type or if file operations fail
#[must_use]
pub fn handle_file_type_choice<'a>(
    files: &[BuildFile],
    choice: &str,
    buildfile: &BuildFile,
) -> Option<(BuildFile, Vec<GrammarFragment>, Vec<&'a str>)> {
    if files.len() > 1 {
        let chosen_file = files
            .iter()
            .find(|x| {
                x.filetype
                    == (match choice {
                        "M" => BuildChoice::Makefile,
                        _ => BuildChoice::Dockerfile,
                    })
            })
            .unwrap()
            .clone();

        let prompt_grammars = make_build_prompt_grammar(buildfile);
        let user_choices = vec!["1", "2", "d", "?"];

        Some((chosen_file, prompt_grammars, user_choices))
    } else {
        eprintln!(
            "No {} found at '{}'",
            match choice {
                "M" => "Makefile",
                _ => "Dockerfile",
            },
            buildfile.parent_dir.display()
        );
        None
    }
}

#[must_use]
pub fn read_val_loop(files: &[BuildFile]) -> WhatWereBuilding {
    let (mut prompt_grammars, user_choices, are_there_multiple_files) = setup_prompts(files);

    let mut choice_of_where_to_build = WhatWereBuilding {
        file: files[0].clone(),
        follow_link: false,
    };

    while should_keep_prompting(
        &mut prompt_grammars,
        files,
        &mut choice_of_where_to_build,
        &user_choices,
        are_there_multiple_files,
    ) {}

    choice_of_where_to_build
}

fn should_keep_prompting(
    prompt_grammars: &mut Vec<GrammarFragment>,
    files: &[BuildFile],
    choice_of_where_to_build: &mut WhatWereBuilding,
    user_choices: &[&str],
    are_there_multiple_files: bool,
) -> bool {
    if prompt_grammars.is_empty() {
        return false;
    }

    let result =
        crate::read_interactive_input::read_val_from_cmd_line_and_proceed_default(prompt_grammars);

    if let Some(choice) = result.user_entered_val {
        return !process_choice(
            &choice,
            files,
            choice_of_where_to_build,
            prompt_grammars,
            user_choices,
            are_there_multiple_files,
        );
    }

    true
}

fn process_choice(
    choice: &str,
    files: &[BuildFile],
    choice_of_where_to_build: &mut WhatWereBuilding,
    prompt_grammars: &mut Vec<GrammarFragment>,
    user_choices: &[&str],
    are_there_multiple_files: bool,
) -> bool {
    match choice {
        "D" | "M" => {
            if let Some((chosen_file, new_prompt_grammars, _new_user_choices)) =
                handle_file_type_choice(files, choice, &choice_of_where_to_build.file)
            {
                choice_of_where_to_build.file = chosen_file;
                *prompt_grammars = new_prompt_grammars;
            }
            false
        }
        "d" | "?" => {
            handle_display_info(
                files,
                &choice_of_where_to_build.file,
                user_choices,
                are_there_multiple_files,
            );
            false
        }
        "1" => {
            choice_of_where_to_build.follow_link = true;
            true
        }
        "2" => {
            choice_of_where_to_build.follow_link = false;
            true
        }
        _ => {
            eprintln!("Invalid choice '{choice}'");
            false
        }
    }
}

pub(super) fn build_prompt_fragment(
    text: String,
    pos: u8,
    grammar_type: Option<GrammarType>,
    suffix: Option<&str>,
) -> GrammarFragment {
    let mut fragment = GrammarFragment {
        original_val_for_prompt: Some(text),
        pos,
        ..Default::default()
    };

    if let Some(grammar_type) = grammar_type {
        fragment.grammar_type = grammar_type;
    }

    if let Some(suffix) = suffix {
        fragment.suffix = Some(suffix.to_string());
    }

    fragment
}
