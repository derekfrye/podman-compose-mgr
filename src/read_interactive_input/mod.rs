// Re-exporting from modular files for backward compatibility
mod format;
mod helpers;
mod types;

pub use self::types::{
    DefaultStdinHelper, GrammarFragment, GrammarType, PrintFunction, ReadValResult, StdinHelper,
    StdinHelperWrapper,
};

pub use self::helpers::{
    default_print, default_println, read_val_from_cmd_line_and_proceed,
    read_val_from_cmd_line_and_proceed_default, read_val_from_cmd_line_and_proceed_with_deps,
};

pub use self::format::{do_prompt_formatting, unroll_grammar_into_string};

// Re-export TestStdinHelper for backward compatibility
pub use crate::testing::stdin_helpers::TestStdinHelper;
