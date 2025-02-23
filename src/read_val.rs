use crate::helpers::cmd_helper_fns as cmd;

use std::cmp::max;
use std::collections::HashSet;
use std::io::{self, Write};

pub struct ReadValResult {
    pub user_entered_val: Option<String>,
    pub grammar: Vec<GrammarFragment>,
    
}

#[derive(Debug, PartialEq, Clone)]
pub enum GrammarType {
    Verbiage,
    UserChoice,
    Image,
    DockerComposePath,
    ContainerName,
    FileName,
    None,
}

#[derive(Debug, PartialEq, Clone)]
pub struct GrammarFragment {
    pub original_val_for_prompt: Option<String>,
    pub shortened_val_for_prompt: Option<String>,
    pub pos: u8,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub grammar_type: GrammarType,
    pub part_of_static_prompt: bool,
    pub display_at_all: bool,
}

impl Default for GrammarFragment {
    fn default() -> Self {
        GrammarFragment {
            original_val_for_prompt: None,
            shortened_val_for_prompt: None,
            pos: 0,
            prefix: None,
            suffix: None,
            grammar_type: GrammarType::Verbiage,
            part_of_static_prompt: false,
            display_at_all: false,
        }
    }
}

/// Build a string to display to the user. Don't use this publicly, try to use read_val_from_cmd_line_and_proceed instead.
 fn unroll_grammar_into_string(
    grammars: &Vec<GrammarFragment>,
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

        if use_shortened_val && grammar.shortened_val_for_prompt.is_some() {
            return_result.push_str(grammar.shortened_val_for_prompt.as_ref().unwrap().as_str());
        } else if grammar.display_at_all {
            return_result.push_str(grammar.original_val_for_prompt.as_ref().unwrap().as_str());
        }

        if let Some(suffix) = &grammar.suffix {
            return_result.push_str(suffix);
        }
    }
    return_result
}

// moved from main, i've got to believe i'll use it for secrets and restart svcs too
pub fn read_val_from_cmd_line_and_proceed(
    grammars: &mut Vec<GrammarFragment>,
    grammars_to_shorten: Option<Vec<GrammarType>>,
    
) -> ReadValResult {
    let type_1_to_shorten = grammars_to_shorten
        .as_ref()
        .and_then(|z| z.get(0))
        .and_then(|&grammar_type| {
            grammars
                .iter()
                .find(|x| x.grammar_type == grammar_type)
                .and_then(|f| f.original_val_for_prompt.clone())
        })
        .unwrap_or_else(String::new);
    let type_2_to_shorten = grammars_to_shorten
    .as_ref()
    .and_then(|z| z.get(1))
    .and_then(|&grammar_type| {
        grammars
            .iter()
            .find(|x| x.grammar_type == grammar_type)
            .and_then(|f| f.original_val_for_prompt.clone())
    })
    .unwrap_or_else(String::new);

    let mut return_result = ReadValResult {
        user_entered_val: None,
        grammar: Vec::new(),
        
    };

    let refresh_static = unroll_grammar_into_string(grammars, true, false);
    let refresh_prompt = unroll_grammar_into_string(grammars, false, false);

    // if the prompt is too long, we need to shorten some stuff.
    // At a minimum, we'll display Verbiage and UserChoices un-shortened. 
    // We're not going to go less than 12 chars for path and image name, anything less feels like we're cutting too much off maybe.
    let fixed_len_grammars = grammars.iter().fold(0, |acc, x| {
        if x.grammar_type == GrammarType::Verbiage || x.grammar_type == GrammarType::UserChoice {
            acc + x.original_val_for_prompt.as_ref().unwrap().len()
        } else {
            acc
        }
    });

    // Then we divide remaining space equally between items that can be shortened
    let term_width = cmd::get_terminal_display_width();
    // println!("term_width: {}", term_width);
    // println!("refresh_prompt len: {}", refresh_prompt.len());
    let mut type_1_shortened = type_1_to_shorten.clone();
    // let docker_compose_path_orig = docker_compose_pth_shortened.to_string();
    let mut type_2_shortened = type_2_to_shorten.clone();
    // let image_orig = image.to_string();
    // 1 char for a little buffer so it doesn't wrap after user input
    if refresh_prompt.len() > term_width - 1 {
        let truncated_symbols = "...";
        let mut max_avail_chars_for_image_and_path =
            max(term_width, fixed_len_grammars) - refresh_static.len() - 2 * truncated_symbols.len() - 1;
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

    let type_1_grammar = GrammarFragment {
        original_val_for_prompt: Some(type_1_to_shorten.clone()),
        shortened_val_for_prompt: Some(type_1_shortened.clone()),
        pos: 0,
        prefix: None,
        suffix: None,
        grammar_type: grammars_to_shorten.clone(),
        part_of_static_prompt: true,
        display_at_all: true,
    };

    let type_2_grammar = GrammarFragment {
        original_val_for_prompt: Some(type_2_to_shorten.clone()),
        shortened_val_for_prompt: Some(type_2_shortened.clone()),
        pos: 0,
        prefix: None,
        suffix: None,
        grammar_type: grammar_type_2_to_shorten.clone(),
        part_of_static_prompt: true,
        display_at_all: true,
    };
    return_result.grammar.push(type_1_grammar);
    return_result.grammar.push(type_2_grammar);

    // put the shortened values into the input grammar, so when we prompt the user from the unrolled grammar, we use the shortened values
    let x = return_result
        .grammar
        .iter()
        .find(|x| x.grammar_type == grammars_to_shorten)
        .and_then(|x| x.shortened_val_for_prompt.clone());
    let z = return_result
        .grammar
        .iter()
        .find(|x| x.grammar_type == grammar_type_2_to_shorten)
        .and_then(|x| x.shortened_val_for_prompt.clone());
    grammars.iter_mut().for_each(|y| {
        if y.grammar_type == grammars_to_shorten {
            y.shortened_val_for_prompt = x.clone();
        }
        if y.grammar_type == grammar_type_2_to_shorten {
            y.shortened_val_for_prompt = z.clone();
        }
    });

    // prepare the prompt, this might go to stdout, or we have to flush first
    print!("{}", unroll_grammar_into_string(grammars, false, true));

    // what were the available choices someone could've made
    let user_choices: HashSet<String> = grammars
        .iter()
        .filter(|x| x.grammar_type == GrammarType::UserChoice)
        .collect::<Vec<&GrammarFragment>>()
        .iter()
        .map(|x| x.original_val_for_prompt.clone().unwrap())
        .collect();

    loop {
        let mut input = String::new();
        // flush stdout so prompt for sure displays
        io::stdout().flush().unwrap();
        // read a line of input from stdin
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        // if user specified something that was an available choice, return that result
        if user_choices.contains(input) {
            return_result.user_entered_val = Some(input.to_string());
            break;
        } else {
            break;
        }
    }

    return_result
}
