// Re-exporting from modular files for backward compatibility
mod types;
mod helpers;
mod format;

pub use self::types::{
    ReadValResult, PrintFunction, GrammarType, GrammarFragment, 
    StdinHelper, DefaultStdinHelper, StdinHelperWrapper
};

pub use self::helpers::{
    default_print, default_println, 
    read_val_from_cmd_line_and_proceed,
    read_val_from_cmd_line_and_proceed_default,
    read_val_from_cmd_line_and_proceed_with_deps
};

pub use self::format::{
    unroll_grammar_into_string,
    do_prompt_formatting
};

// Re-export TestStdinHelper for backward compatibility
pub use crate::testing::stdin_helpers::TestStdinHelper;