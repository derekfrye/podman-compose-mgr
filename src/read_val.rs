use crate::helpers::cmd_helper_fns as cmd;

use std::cmp::max;
use std::collections::HashSet;
use std::io::{self, Write};

pub struct Result {
    pub user_entered_val: Option<String>,
    pub grammar: Vec<Grammar>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum GrammerType {
    Verbiage,
    UserChoice,
    Image,
    DockerComposePath,
    ContainerName,
    FileName,
    None,
}

#[derive(Debug, PartialEq)]
pub struct Grammar {
    pub original_val_for_prompt: Option<String>,
    pub shortend_val_for_prompt: Option<String>,
    pub pos: u8,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub grammer_type: GrammerType,
    pub part_of_static_prompt: bool,
    pub display_at_all: bool,
}

impl Default for Grammar {
    fn default() -> Self {
        Grammar {
            original_val_for_prompt: None,
            shortend_val_for_prompt: None,
            pos: 0,
            prefix: None,
            suffix: None,
            grammer_type: GrammerType::Verbiage,
            part_of_static_prompt: false,
            display_at_all: false,
        }
    }
}

fn unroll_grammer_into_string(
    grammars: &Vec<Grammar>,
    excl_if_not_in_base_prompt: bool,
    use_shortened_val: bool,
) -> String {
    let mut return_result = String::new();
    // lets loop through based on the position
    for grammar in grammars.iter() {
        if excl_if_not_in_base_prompt && !grammar.part_of_static_prompt {
            return_result.push_str(" ");
            continue;
        }
        if let Some(prefix) = &grammar.prefix {
            return_result.push_str(prefix);
        }

        if use_shortened_val && grammar.shortend_val_for_prompt.is_some() {
            return_result.push_str(grammar.shortend_val_for_prompt.as_ref().unwrap().as_str());
        } else if grammar.display_at_all {
            return_result.push_str(grammar.original_val_for_prompt.as_ref().unwrap().as_str());
        }

        if let Some(suffix) = &grammar.suffix {
            return_result.push_str(suffix);
        }
    }
    return_result
}

// moved from main, i've got to believe i'll use it for secrets and restartsvcs too
pub fn read_val_from_cmd_line_and_proceed(
    grammars: &mut Vec<Grammar>,
    grammar_type_1_to_shorten: GrammerType,
    grammar_type_2_to_shorten: GrammerType,
) -> Result {
    let type_1_to_shorten = grammars
        .iter()
        .find(|x| x.grammer_type == grammar_type_1_to_shorten)
        .map(|f| f.original_val_for_prompt.clone())
        .unwrap()
        .unwrap();
    let type_2_to_shorten = grammars
        .iter()
        .find(|x| x.grammer_type == grammar_type_2_to_shorten)
        .map(|f| f.original_val_for_prompt.clone())
        .unwrap_or_else(|| Some(String::new()))
        .unwrap_or_else(String::new);

    let mut return_result = Result {
        user_entered_val: None,
        grammar: Vec::new(),
    };

    let refresh_static = unroll_grammer_into_string(grammars, true, false);
    let refresh_prompt = unroll_grammer_into_string(grammars, false, false);

    // if the prompt is too long, we need to shorten some stuff.
    // At a minimum, we'll display our 23 chars of "refresh ... from ?" stuff.
    // Then we divide remaining space equally between image name and path name.
    // We're not going to go less than 12 chars for path and image name, anything less feels like we're cutting too much off maybe.
    // This means total display chars is 23 + 12 + 12 = 47 at a min
    // if user has less than 47 wide, well then we'll have to let the terminal word-wrap.
    let term_width = cmd::get_terminal_display_width();
    // println!("term_width: {}", term_width);
    // println!("refresh_prompt len: {}", refresh_prompt.len());
    let mut type_1_shortened = type_1_to_shorten.clone();
    // let docker_compose_path_orig = docker_compose_pth_shortened.to_string();
    let mut type_2_shortened = type_2_to_shorten.clone();
    // let image_orig = image.to_string();
    // 1 char for a little buffer so it doesnt wrap after user input
    if refresh_prompt.len() > term_width - 1 {
        let truncated_symbols = "...";
        let mut max_avail_chars_for_image_and_path =
            max(term_width, 47) - refresh_static.len() - 2 * truncated_symbols.len() - 1;
        if max_avail_chars_for_image_and_path % 2 != 0 {
            max_avail_chars_for_image_and_path -= 1;
        }

        if type_1_shortened.len() > max_avail_chars_for_image_and_path / 2 {
            type_1_shortened = format!(
                "...{}",
                type_1_shortened[type_1_shortened.len() - max_avail_chars_for_image_and_path / 2..]
                    .to_string()
            );
        }

        if type_2_shortened.len() > max_avail_chars_for_image_and_path / 2 {
            type_2_shortened = format!(
                "...{}",
                type_2_shortened[type_2_shortened.len() - max_avail_chars_for_image_and_path / 2..]
                    .to_string()
            );
        }
    }

    let type_1_grammar = Grammar {
        original_val_for_prompt: Some(type_1_to_shorten.clone()),
        shortend_val_for_prompt: Some(type_1_shortened.clone()),
        pos: 0,
        prefix: None,
        suffix: None,
        grammer_type: grammar_type_1_to_shorten.clone(),
        part_of_static_prompt: true,
        display_at_all: true,
    };

    let type_2_grammar = Grammar {
        original_val_for_prompt: Some(type_2_to_shorten.clone()),
        shortend_val_for_prompt: Some(type_2_shortened.clone()),
        pos: 0,
        prefix: None,
        suffix: None,
        grammer_type: grammar_type_2_to_shorten.clone(),
        part_of_static_prompt: true,
        display_at_all: true,
    };
    return_result.grammar.push(type_1_grammar);
    return_result.grammar.push(type_2_grammar);

    // put the shortened values into the input grammar, so when we prompt the user from the unrolled grammar, we use the shortend values
    let x = return_result
        .grammar
        .iter()
        .find(|x| x.grammer_type == grammar_type_1_to_shorten)
        .and_then(|x| x.shortend_val_for_prompt.clone());
    let z = return_result
        .grammar
        .iter()
        .find(|x| x.grammer_type == grammar_type_2_to_shorten)
        .and_then(|x| x.shortend_val_for_prompt.clone());
    grammars.iter_mut().for_each(|y| {
        if y.grammer_type == grammar_type_1_to_shorten {
            y.shortend_val_for_prompt = x.clone();
        }
        if y.grammer_type == grammar_type_2_to_shorten {
            y.shortend_val_for_prompt = z.clone();
        }
    });

    print!("{}", unroll_grammer_into_string(grammars, false, true));

    let user_choices: HashSet<String> = grammars
        .iter()
        .filter(|x| x.grammer_type == GrammerType::UserChoice)
        .collect::<Vec<&Grammar>>()
        .iter()
        .map(|x| x.original_val_for_prompt.clone().unwrap())
        .collect();

    loop {
        let mut input = String::new();
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if user_choices.contains(input) {
            return_result.user_entered_val = Some(input.to_string());
            break;
        } else {
            break;
        }
    }

    return_result
}
