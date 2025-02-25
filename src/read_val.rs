use crate::helpers::cmd_helper_fns as cmd;

// use std::cmp::max;
use std::collections::HashSet;
use std::io::{self, Write};

pub struct ReadValResult {
    pub user_entered_val: Option<String>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum GrammarType {
    Verbiage,
    UserChoice,
    Image,
    DockerComposePath,
    ContainerName,
    FileName,
}

#[derive(Debug, PartialEq, Clone)]
pub struct GrammarFragment {
    pub original_val_for_prompt: Option<String>,
    pub shortened_val_for_prompt: Option<String>,
    pub pos: u8,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub grammar_type: GrammarType,
    pub display_at_all: bool,
    pub can_shorten: bool,
}

impl Default for GrammarFragment {
    fn default() -> Self {
        GrammarFragment {
            original_val_for_prompt: None,
            shortened_val_for_prompt: None,
            pos: 0,
            prefix: None,
            suffix: Some(" ".to_string()),
            grammar_type: GrammarType::Verbiage,
            can_shorten: false,
            display_at_all: true,
        }
    }
}

/// Build a string to display to the user. Don't use this publicly, try to use read_val_from_cmd_line_and_proceed instead.
fn unroll_grammar_into_string(
    grammars: &[GrammarFragment],
    excl_if_not_in_base_prompt: bool,
    use_shortened_val: bool,
) -> String {
    let mut return_result = String::new();
    // lets loop through based on the position
    for grammar in grammars.iter().filter(|g| g.display_at_all) {
        if excl_if_not_in_base_prompt && grammar.can_shorten {
            return_result.push(' ');
            continue;
        }
        if let Some(prefix) = &grammar.prefix {
            return_result.push_str(prefix);
        }

        if use_shortened_val && grammar.shortened_val_for_prompt.is_some() {
            return_result.push_str(grammar.shortened_val_for_prompt.as_ref().unwrap().as_str());
        } else {
            return_result.push_str(grammar.original_val_for_prompt.as_ref().unwrap().as_str());
        }

        if let Some(suffix) = &grammar.suffix {
            return_result.push_str(suffix);
        }
    }
    return_result
}

pub fn read_val_from_cmd_line_and_proceed(grammars: &mut Vec<GrammarFragment>) -> ReadValResult {
    let mut return_result = ReadValResult {
        user_entered_val: None,
    };

    let term_width = cmd::get_terminal_display_width();
    let initial_prompt = unroll_grammar_into_string(grammars, false, false);

    if initial_prompt.len() > term_width - 1 {
        // let refresh_static = unroll_grammar_into_string(grammars, true, false);
        // let refresh_prompt = unroll_grammar_into_string(grammars, false, false);

        // if the prompt is too long, we need to shorten some stuff.
        // At a minimum, we'll display Verbiage and UserChoices un-shortened.
        let fixed_len_grammars: usize = grammars
            .iter()
            .filter(|g| {
                g.grammar_type == GrammarType::Verbiage || g.grammar_type == GrammarType::UserChoice
            })
            .map(|g| {
                let suffix = g.suffix.clone().unwrap_or_default();
                let prefix = g.prefix.clone().unwrap_or_default();
                prefix.len() + g.original_val_for_prompt.as_ref().unwrap().len() + suffix.len()
            })
            .sum();

        // Then we divide remaining space equally between items that can be shortened

        // 3. Collect the fragments that we want to shorten (those that are not Verbiage or UserChoice).
        let mut shortenable_grammars: Vec<&mut GrammarFragment> = grammars
            .iter_mut()
            .filter(|g| {
                g.grammar_type != GrammarType::Verbiage
                    && g.grammar_type != GrammarType::UserChoice
                    && g.display_at_all
            })
            .collect();

        let n = shortenable_grammars.len();

        // 2. Calculate the total remaining space available for the other fragments.
        //    We subtract one extra character to account for user input.
        let total_remaining_space = if term_width > fixed_len_grammars {
            term_width - fixed_len_grammars - (n + 5)
        } else {
            0
        };

        if total_remaining_space > 0 {
            // Only proceed if we have shortenable fragments and enough space (reserve 3 for "...")
            if n > 0 && total_remaining_space > 3 {
                // Determine how many characters each shortenable fragment is allowed
                let allowed_len = ((total_remaining_space - 3) as f64 / n as f64).floor() as usize;
                if cfg!(debug_assertions) {
                    println!("term width: {}", term_width);
                    println!("fixed len: {}", fixed_len_grammars);
                    println!("remain space: {}", total_remaining_space);
                    println!("allow len: {}", allowed_len);
                    println!("total calc: {}", allowed_len * n + 3);
                }

                // 4. For each shortenable fragment, set its shortened value.
                for grammar in shortenable_grammars.iter_mut() {
                    let orig = grammar.original_val_for_prompt.as_ref().unwrap();
                    // If the original is longer than the allowed length, shorten it.
                    if orig.len() > allowed_len {
                        // Grab the last `allowed_len` characters.
                        if grammar.grammar_type == GrammarType::Image {
                            let substring = &orig[..allowed_len - 1];
                            grammar.shortened_val_for_prompt = Some(format!("{}...", substring));
                        } else {
                            let substring = &orig[orig.len() - allowed_len..];
                            grammar.shortened_val_for_prompt = Some(format!("...{}", substring));
                        };
                    } else {
                        // If it already fits, use the original.
                        grammar.shortened_val_for_prompt = Some(orig.clone());
                    }
                }
            }
        }

        for i in shortenable_grammars.iter_mut() {
            if i.display_at_all && i.shortened_val_for_prompt.is_none() {
                i.shortened_val_for_prompt = i.original_val_for_prompt.clone();
            }
        }
    }

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
            println!("Invalid input '{}'. Please try again.", input);
            print!("{}", unroll_grammar_into_string(grammars, false, true));
        }
    }

    return_result
}
