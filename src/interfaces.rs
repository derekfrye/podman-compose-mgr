use std::path::PathBuf;
use crate::read_val::{GrammarFragment, ReadValResult};

/// Interface for command-related functions to facilitate testing
pub trait CommandHelper {
    fn exec_cmd(&self, cmd: &str, args: Vec<String>);
    fn pull_base_image(&self, dockerfile: &PathBuf) -> Result<(), Box<dyn std::error::Error>>;
    fn get_terminal_display_width(&self, specify_size: Option<usize>) -> usize;
    fn file_exists_and_readable(&self, file: &std::path::Path) -> bool;
}

/// Interface for read_val-related functions to facilitate testing
pub trait ReadValHelper {
    fn read_val_from_cmd_line_and_proceed(&self, grammars: &mut [GrammarFragment], size: Option<usize>) -> ReadValResult;
}

/// Default implementation of CommandHelper that uses the actual functions
pub struct DefaultCommandHelper;

impl CommandHelper for DefaultCommandHelper {
    fn exec_cmd(&self, cmd: &str, args: Vec<String>) {
        // Convert Vec<String> to Vec<&str> for compatibility with existing function
        let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        crate::helpers::cmd_helper_fns::exec_cmd(cmd, &args_ref);
    }
    
    fn pull_base_image(&self, dockerfile: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        crate::helpers::cmd_helper_fns::pull_base_image(dockerfile)
    }
    
    fn get_terminal_display_width(&self, specify_size: Option<usize>) -> usize {
        crate::helpers::cmd_helper_fns::get_terminal_display_width(specify_size)
    }
    
    fn file_exists_and_readable(&self, file: &std::path::Path) -> bool {
        crate::helpers::cmd_helper_fns::file_exists_and_readable(file)
    }
}

/// Default implementation of ReadValHelper that uses the actual function
pub struct DefaultReadValHelper;

impl ReadValHelper for DefaultReadValHelper {
    fn read_val_from_cmd_line_and_proceed(&self, grammars: &mut [GrammarFragment], _size: Option<usize>) -> ReadValResult {
        // Use the default command helper for terminal width
        let cmd_helper = DefaultCommandHelper;
        crate::read_val::read_val_from_cmd_line_and_proceed_with_deps(
            grammars, 
            &cmd_helper, 
            crate::read_val::default_print, 
            None,
        )
    }
}