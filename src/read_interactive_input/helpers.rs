use crate::interfaces::CommandHelper;
use crate::read_interactive_input::format::{do_prompt_formatting, unroll_grammar_into_string};
use crate::read_interactive_input::types::{
    GrammarFragment, GrammarType, PrintFunction, ReadValResult, StdinHelperWrapper,
};
use std::collections::HashSet;

/// Default print function that writes to stdout
pub fn default_print(s: &str) {
    print!("{}", s);
}

/// Default println function that writes to stdout with newline
pub fn default_println(s: &str) {
    println!("{}", s);
}

/// Original function for backwards compatibility - forwards to the dependency-injected version
pub fn read_val_from_cmd_line_and_proceed<C: crate::interfaces::CommandHelper>(
    grammars: &mut [GrammarFragment],
    cmd_helper: &C,
) -> ReadValResult {
    // Using None for stdin_helper will use the default stdin reading behavior
    read_val_from_cmd_line_and_proceed_with_deps(
        grammars,
        cmd_helper,
        Box::new(default_print),
        None,
        None,
    )
}

/// Compatibility wrapper that uses DefaultCommandHelper
pub fn read_val_from_cmd_line_and_proceed_default(
    grammars: &mut [GrammarFragment],
) -> ReadValResult {
    // Use DefaultCommandHelper for the terminal width
    let cmd_helper = crate::interfaces::DefaultCommandHelper;
    read_val_from_cmd_line_and_proceed(grammars, &cmd_helper)
}

/// Collect available user choices from grammar fragments
fn collect_user_choices(grammars: &[GrammarFragment]) -> HashSet<String> {
    grammars
        .iter()
        .filter(|x| x.grammar_type == GrammarType::UserChoice)
        .map(|x| x.original_val_for_prompt.clone().unwrap())
        .collect()
}

/// Process user input and determine action
fn process_user_input(
    input: &str,
    user_choices: &HashSet<String>,
    print_fn: &PrintFunction<'_>,
    prompt_string: &str,
) -> Option<Option<String>> {
    if user_choices.contains(input) {
        // Valid choice
        Some(Some(input.to_string()))
    } else if input.is_empty() || input.trim().is_empty() {
        // Empty input
        Some(None)
    } else {
        // Invalid input
        eprintln!("Invalid input '{}'. Please try again.", input);
        print_fn(prompt_string);
        None
    }
}

/// Implementation with dependency injection for the CommandHelper trait. Keep in sync with testing code.
pub fn read_val_from_cmd_line_and_proceed_with_deps<C: CommandHelper>(
    grammars: &mut [GrammarFragment],
    cmd_helper: &C,
    print_fn: PrintFunction<'_>,
    size: Option<usize>,
    stdin_helper: Option<StdinHelperWrapper>,
) -> ReadValResult {
    let mut return_result = ReadValResult {
        user_entered_val: None,
    };

    // Format the prompt
    let term_width = cmd_helper.get_terminal_display_width(size);
    do_prompt_formatting(grammars, term_width);
    let prompt_string = unroll_grammar_into_string(grammars, false, true);

    // Print the prompt
    print_fn(&prompt_string);

    // Get available user choices
    let user_choices = collect_user_choices(grammars);

    // Setup stdin helper
    let default_stdin_wrapper = StdinHelperWrapper::default();
    let stdin_wrapper = stdin_helper.as_ref().unwrap_or(&default_stdin_wrapper);

    // Input loop
    loop {
        // Get input
        let input = stdin_wrapper.read_line();

        // Process input
        if let Some(result) = process_user_input(&input, &user_choices, &print_fn, &prompt_string) {
            return_result.user_entered_val = result;
            break;
        }
    }

    return_result
}
