use crate::interfaces::CommandHelper;
use crate::read_interactive_input::{
    GrammarFragment, ReadValResult, StdinHelperWrapper, do_prompt_formatting,
    unroll_grammar_into_string,
};

/// Helper function for testing that runs the full prompt formatting pipeline
/// and returns the formatted prompt string.
///
/// This allows tests to see exactly what the prompt would look like to users.
pub fn test_format_prompt<C: CommandHelper>(
    grammars: &mut [GrammarFragment],
    cmd_helper: &C,
    size: Option<usize>,
) -> String {
    // Get the terminal width
    let term_width = cmd_helper.get_terminal_display_width(size);

    // Do the actual formatting
    do_prompt_formatting(grammars, term_width);

    // Get the formatted string that would be displayed to the user
    unroll_grammar_into_string(grammars, false, true)
}

/// A test-specific wrapper for `read_val_from_cmd_line_and_proceed_with_deps` that
/// displays the formatted prompt to stdout for debugging and test verification.
pub fn test_read_val_with_debug_output<C: CommandHelper>(
    grammars: &mut [GrammarFragment],
    cmd_helper: &C,
    size: Option<usize>,
    stdin_input: &str,
) -> ReadValResult {
    // Create a test stdin helper that will return the provided input
    let stdin_helper = StdinHelperWrapper::Test(crate::testing::stdin_helpers::TestStdinHelper {
        response: stdin_input.to_string(),
    });

    // Format the prompt string
    let term_width = cmd_helper.get_terminal_display_width(size);
    do_prompt_formatting(grammars, term_width);
    let prompt_string = unroll_grammar_into_string(grammars, false, true);

    // Print the formatted prompt for test verification
    println!("TEST DEBUG - Formatted prompt: {prompt_string}");

    // Call the real function
    crate::read_interactive_input::read_val_from_cmd_line_and_proceed_with_deps(
        grammars,
        cmd_helper,
        Box::new(|s| print!("{s}")), // Print function that prints to stdout
        size,
        Some(&stdin_helper),
    )
}
