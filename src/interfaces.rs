use crate::read_interactive_input::{GrammarFragment, ReadValResult};
use mockall::automock;
use std::path::Path;

/// Interface for command-related functions to facilitate testing
#[automock]
pub trait CommandHelper {
    /// Execute a command with arguments
    ///
    /// # Arguments
    ///
    /// * `cmd` - Command to execute
    /// * `args` - Arguments to pass to the command
    ///
    /// # Returns
    ///
    /// * `Result<(), Box<dyn std::error::Error>>` - Success or error
    ///
    /// # Errors
    ///
    /// Returns an error if the command execution fails.
    fn exec_cmd(&self, cmd: &str, args: Vec<String>) -> Result<(), Box<dyn std::error::Error>>;
    /// Pull base image specified in dockerfile
    ///
    /// # Arguments
    ///
    /// * `dockerfile` - Path to the dockerfile
    ///
    /// # Returns
    ///
    /// * `Result<(), Box<dyn std::error::Error>>` - Success or error
    ///
    /// # Errors
    ///
    /// Returns an error if the image pull fails.
    fn pull_base_image(&self, dockerfile: &Path) -> Result<(), Box<dyn std::error::Error>>;
    fn get_terminal_display_width(&self, specify_size: Option<usize>) -> usize;
    fn file_exists_and_readable(&self, file: &std::path::Path) -> bool;
}

/// Interface for read_val-related functions to facilitate testing
#[automock]
pub trait ReadInteractiveInputHelper {
    /// Read a value from the command line
    ///
    /// # Arguments
    /// * `grammars` - The grammar fragments to display in the prompt
    /// * `size` - Optional override for terminal width
    ///
    /// # Returns
    /// `ReadValResult` containing the user's input
    fn read_val_from_cmd_line_and_proceed(
        &self,
        grammars: &mut [GrammarFragment],
        size: Option<usize>,
    ) -> ReadValResult;
}

/// Default implementation of `CommandHelper` that uses the actual functions
pub struct DefaultCommandHelper;

impl CommandHelper for DefaultCommandHelper {
    fn exec_cmd(&self, cmd: &str, args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
        // Convert Vec<String> to Vec<&str> for compatibility with existing function
        let args_ref: Vec<&str> = args.iter().map(std::string::String::as_str).collect();
        crate::utils::cmd_utils::exec_cmd(cmd, &args_ref)?;
        Ok(())
    }

    fn pull_base_image(&self, dockerfile: &Path) -> Result<(), Box<dyn std::error::Error>> {
        crate::utils::podman_utils::pull_base_image(dockerfile)
    }

    fn get_terminal_display_width(&self, specify_size: Option<usize>) -> usize {
        crate::utils::podman_utils::get_terminal_display_width(specify_size)
    }

    fn file_exists_and_readable(&self, file: &std::path::Path) -> bool {
        crate::utils::podman_utils::file_exists_and_readable(file)
    }
}

/// Default implementation of `ReadInteractiveInputHelper` that uses the actual functions
pub struct DefaultReadInteractiveInputHelper;

impl ReadInteractiveInputHelper for DefaultReadInteractiveInputHelper {
    fn read_val_from_cmd_line_and_proceed(
        &self,
        grammars: &mut [GrammarFragment],
        size: Option<usize>,
    ) -> ReadValResult {
        let cmd_helper = DefaultCommandHelper;
        crate::read_interactive_input::read_val_from_cmd_line_and_proceed_with_deps(
            grammars,
            &cmd_helper,
            Box::new(crate::read_interactive_input::default_print),
            size,
            None, // Use default stdin behavior
        )
    }
}
