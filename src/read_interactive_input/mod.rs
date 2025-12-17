// Re-exporting from modular files for backward compatibility
pub mod format;
mod helpers;
mod prompt_data;
mod types;

pub use self::types::{
    DefaultStdinHelper, GrammarFragment, GrammarType, InputProcessResult, PrintFunction,
    ReadValResult, StdinHelper, StdinHelperWrapper,
};

pub use self::helpers::{
    default_print, default_println, read_val_from_cmd_line_and_proceed,
    read_val_from_cmd_line_and_proceed_default, read_val_from_cmd_line_and_proceed_with_deps,
    read_val_from_prompt_and_proceed_default,
};

pub use self::format::{do_prompt_formatting, unroll_grammar_into_string};

// Re-export TestStdinHelper for backward compatibility
pub use crate::testing::stdin_helpers::TestStdinHelper;
