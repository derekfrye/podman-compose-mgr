use crate::read_interactive_input::types::{GrammarFragment, GrammarType};

/// Grammar for a prompt element
#[derive(Clone, Debug)]
pub struct PromptGrammar {
    pub text: String,
    pub can_shorten: bool,
    pub display_at_all: bool,
    pub suffix: String,
}

/// Complete prompt with grammar
#[derive(Clone, Debug)]
pub struct Prompt {
    pub full_prompt: String,
    pub grammar: Vec<PromptGrammar>,
}

/// Build a string to display to the user. Generally use `read_val_from_cmd_line_and_proceed` instead.
/// Made public to allow usage in tests.
///
/// # Panics
/// Panics if grammar fragments contain invalid shortened values when `use_shortened_val` is true
#[must_use]
pub fn unroll_grammar_into_string(
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

/// Calculate the fixed length of non-shortenable grammar fragments
fn calculate_fixed_length(grammars: &[GrammarFragment]) -> usize {
    grammars
        .iter()
        .filter(|g| {
            g.grammar_type == GrammarType::Verbiage || g.grammar_type == GrammarType::UserChoice
        })
        .map(|g| {
            let suffix = g.suffix.clone().unwrap_or_default();
            let prefix = g.prefix.clone().unwrap_or_default();
            prefix.len() + g.original_val_for_prompt.as_ref().unwrap().len() + suffix.len()
        })
        .sum()
}

/// Collect fragments that can be shortened
fn collect_shortenable_fragments(grammars: &mut [GrammarFragment]) -> Vec<&mut GrammarFragment> {
    grammars
        .iter_mut()
        .filter(|g| {
            g.grammar_type != GrammarType::Verbiage
                && g.grammar_type != GrammarType::UserChoice
                && g.display_at_all
        })
        .collect()
}

/// Shorten a grammar fragment based on its type
fn shorten_fragment(grammar: &mut GrammarFragment, allowed_len: usize) {
    let orig = grammar.original_val_for_prompt.as_ref().unwrap();

    // If the original is longer than the allowed length, shorten it
    if orig.len() > allowed_len {
        if grammar.grammar_type == GrammarType::Image {
            // For images, keep the beginning and add ellipsis at the end
            let substring = &orig[..allowed_len - 1];
            grammar.shortened_val_for_prompt = Some(format!("{substring}..."));
        } else {
            // For other types, keep the end and add ellipsis at the beginning
            let substring = &orig[orig.len() - allowed_len..];
            grammar.shortened_val_for_prompt = Some(format!("...{substring}"));
        }
    } else {
        // If it already fits, use the original
        grammar.shortened_val_for_prompt = Some(orig.clone());
    }
}

// Format the prompt to fit within the terminal width
pub fn do_prompt_formatting(grammars: &mut [GrammarFragment], term_width: usize) -> String {
    let initial_prompt = unroll_grammar_into_string(grammars, false, false);

    // If prompt is too long, we need to shorten some fragments
    if initial_prompt.len() > term_width - 1 {
        // Calculate space taken by fixed-length grammar fragments
        let fixed_len_grammars = calculate_fixed_length(grammars);

        // Collect fragments that can be shortened
        let mut shortenable_grammars = collect_shortenable_fragments(grammars);
        let n = shortenable_grammars.len();

        // Calculate remaining space for shortenable fragments
        let total_remaining_space = if term_width > fixed_len_grammars {
            term_width - fixed_len_grammars - (n + 5) // Reserve space for UI elements
        } else {
            0
        };

        // Only proceed if we have space and fragments to shorten
        if total_remaining_space > 0 && n > 0 && total_remaining_space > 3 {
            // Calculate allowed length per fragment
            let allowed_len = (total_remaining_space - 3) / n;

            // Shorten each fragment
            for grammar in &mut shortenable_grammars {
                shorten_fragment(grammar, allowed_len);
            }
        }

        // Ensure all display fragments have a shortened value
        for grammar in grammars.iter_mut() {
            if grammar.display_at_all && grammar.shortened_val_for_prompt.is_none() {
                grammar.shortened_val_for_prompt = grammar.original_val_for_prompt.clone();
            }
        }
    }

    // Return the formatted prompt
    unroll_grammar_into_string(grammars, false, true)
}
