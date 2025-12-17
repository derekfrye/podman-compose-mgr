use crate::interfaces::CommandHelper;
use crate::read_interactive_input::format::Prompt;
use crate::read_interactive_input::prompt_data::{
    PromptData, build_prompt_data, create_reedline_editor, resolve_stdin_wrapper,
};
use crate::read_interactive_input::types::{
    GrammarFragment, GrammarType, InputProcessResult, PrintFunction, ReadValResult,
    StdinHelperWrapper,
};
use std::collections::HashSet;
// Use reedline for line editing when no custom stdin helper is provided
use reedline::{DefaultPrompt, Reedline, Signal};

/// Default print function that writes to stdout
pub fn default_print(s: &str) {
    print!("{s}");
}

/// Default println function that writes to stdout with newline
pub fn default_println(s: &str) {
    println!("{s}");
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

/// Compatibility wrapper that uses `DefaultCommandHelper`
pub fn read_val_from_cmd_line_and_proceed_default(
    grammars: &mut [GrammarFragment],
) -> ReadValResult {
    // Use DefaultCommandHelper for the terminal width
    let cmd_helper = crate::interfaces::DefaultCommandHelper;
    read_val_from_cmd_line_and_proceed(grammars, &cmd_helper)
}

/// Process user input and determine action
fn process_user_input(
    input: &str,
    user_choices: &HashSet<String>,
    print_fn: &PrintFunction<'_>,
    prompt_string: &str,
) -> InputProcessResult {
    if user_choices.contains(input) {
        // Valid choice
        InputProcessResult::Valid(input.to_string())
    } else if input.is_empty() || input.trim().is_empty() {
        // Empty input
        InputProcessResult::Empty
    } else {
        // Invalid input
        eprintln!("Invalid input '{input}'. Please try again.");
        print_fn(prompt_string);
        InputProcessResult::Invalid
    }
}

/// Implementation with dependency injection for the `CommandHelper` trait. Keep in sync with testing code.
#[allow(clippy::needless_pass_by_value)] // PrintFunction needs to be owned for trait object
pub fn read_val_from_cmd_line_and_proceed_with_deps<C: CommandHelper>(
    grammars: &mut [GrammarFragment],
    cmd_helper: &C,
    print_fn: PrintFunction<'_>,
    size: Option<usize>,
    stdin_helper: Option<&StdinHelperWrapper>,
) -> ReadValResult {
    let prompt_data = build_prompt_data(grammars, cmd_helper, size);
    print_fn(&prompt_data.prompt_string);

    let default_stdin_wrapper = StdinHelperWrapper::default();
    let stdin_wrapper = resolve_stdin_wrapper(stdin_helper, &default_stdin_wrapper);
    let mut editor = create_reedline_editor(stdin_helper);

    read_input_loop(&prompt_data, &print_fn, stdin_wrapper, editor.as_mut())
}

fn read_input_loop(
    prompt_data: &PromptData,
    print_fn: &PrintFunction<'_>,
    stdin_wrapper: &StdinHelperWrapper,
    mut editor: Option<&mut Reedline>,
) -> ReadValResult {
    let mut result = ReadValResult {
        user_entered_val: None,
        was_interrupted: false,
    };

    while let Some(input) = read_input(editor.as_deref_mut(), stdin_wrapper, &mut result) {
        match process_user_input(
            &input,
            &prompt_data.user_choices,
            print_fn,
            &prompt_data.prompt_string,
        ) {
            InputProcessResult::Valid(value) => {
                result.user_entered_val = Some(value);
                break;
            }
            InputProcessResult::Empty => {
                result
                    .user_entered_val
                    .clone_from(&prompt_data.default_choice);
                break;
            }
            InputProcessResult::Invalid => {}
        }
    }

    result
}

fn read_input(
    editor: Option<&mut Reedline>,
    stdin_wrapper: &StdinHelperWrapper,
    result: &mut ReadValResult,
) -> Option<String> {
    match editor {
        Some(editor) => match editor.read_line(&DefaultPrompt::default()) {
            Ok(Signal::Success(buffer)) => Some(buffer),
            Ok(Signal::CtrlC | Signal::CtrlD) => {
                result.was_interrupted = true;
                Some(String::new())
            }
            Err(err) => {
                eprintln!("Error reading line: {err}");
                None
            }
        },
        None => Some(stdin_wrapper.read_line()),
    }
}

/// New function for handling structured prompts
#[must_use]
pub fn read_val_from_prompt_and_proceed_default(prompt: &Prompt, verbose: bool) -> ReadValResult {
    // Convert PromptGrammar to GrammarFragment
    let mut grammar_fragments: Vec<GrammarFragment> = prompt
        .grammar
        .iter()
        .map(|g| {
            GrammarFragment {
                grammar_type: if g.can_shorten {
                    GrammarType::FileName
                } else {
                    GrammarType::Verbiage
                },
                can_shorten: g.can_shorten,
                display_at_all: g.display_at_all,
                original_val_for_prompt: Some(g.text.clone()),
                shortened_val_for_prompt: None,
                prefix: None,
                suffix: Some(g.suffix.clone()),
                pos: 0, // Default position
                is_default_choice: false,
            }
        })
        .collect();

    // Print full prompt if verbose
    if verbose {
        println!("{}", prompt.full_prompt);
    }

    // Use the existing function with our converted grammar
    read_val_from_cmd_line_and_proceed_default(&mut grammar_fragments)
}
