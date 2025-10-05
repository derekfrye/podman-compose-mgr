pub mod app;
pub mod args;
pub mod domain;
pub mod errors;
pub mod ports;
pub mod infra {
    pub mod discovery_adapter;
    pub mod interrupt_adapter;
    pub mod podman_adapter;
}
pub mod image_build;
pub mod cli_mvu;
pub mod interfaces;
pub mod read_interactive_input;

pub mod testing;
pub mod tui;
pub mod utils;
pub mod walk_dirs;

pub use args::Args;
pub use interfaces::{CommandHelper, ReadInteractiveInputHelper};
pub use read_interactive_input::unroll_grammar_into_string;
pub use utils::cmd_utils;
pub use utils::error_utils;
pub use utils::json_utils;
pub use utils::log_utils;

// no-op
use std::fmt::Write as FmtWrite;
use std::io;

/// Main application logic separated from `main()` for testing
///
/// This function contains all the core application logic that would normally
/// be in `main()`, making it testable.
///
/// # Arguments
///
/// * `args` - The command-line arguments
///
/// # Returns
///
/// * `io::Result<()>` - Success or error
///
/// # Errors
///
/// Returns an error if the application fails to initialize or run.
pub fn run_app(args: &args::Args) -> io::Result<()> {
    use crate::utils::log_utils::Logger;
    // no extra imports needed here
    // Create logger instance
    let logger = Logger::new(args.verbose);

    // If double verbose, print the command line in a copy-paste friendly format
    if args.verbose >= 2 {
        // Get the program name
        let exe_path = std::env::current_exe()
            .unwrap_or_else(|_| std::path::PathBuf::from("podman-compose-mgr"));
        let exe_name = exe_path
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("podman-compose-mgr"));

        // Start building the command line
        let mut cmd_line = format!("{}", exe_name.to_string_lossy());

        // Use serde to convert Args to a JSON value for inspection
        let args_json = serde_json::to_value(args).unwrap_or(serde_json::Value::Null);

        if let serde_json::Value::Object(map) = args_json {
            // Sort the keys for consistent output
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();

            for key in keys {
                // Format key from snake_case to kebab-case for command line args
                let arg_key = key.replace('_', "-");

                // Skip certain fields that don't need to be included
                if key == "verbose" {
                    // Add the verbose flag based on the count
                    let count = map
                        .get(key)
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                    for _ in 0..count {
                        cmd_line.push_str(" --verbose");
                    }
                    continue;
                }

                match map.get(key) {
                    // Skip values that should be ignored
                    Some(serde_json::Value::Null | serde_json::Value::Bool(false)) | None => {
                        // Skip null values, false booleans, and missing keys
                    }
                    Some(serde_json::Value::Array(arr)) if arr.is_empty() => {
                        // Skip empty arrays
                    }
                    Some(serde_json::Value::String(s)) if s.is_empty() => {
                        // Skip empty strings
                    }
                    Some(serde_json::Value::Array(arr)) => {
                        // Format arrays (e.g., vectors)
                        for item in arr {
                            let escaped_value = match item {
                                serde_json::Value::String(s) => format!("\"{s}\""),
                                _ => item.to_string(),
                            };
                            write!(cmd_line, " --{arg_key} {escaped_value}").unwrap();
                        }
                    }
                    Some(serde_json::Value::Bool(true)) => {
                        write!(cmd_line, " --{arg_key}").unwrap();
                    }
                    Some(value) => {
                        // Format everything else
                        let escaped_value = match value {
                            serde_json::Value::String(s) => format!("\"{s}\""),
                            _ => value.to_string(),
                        };
                        write!(cmd_line, " --{arg_key} {escaped_value}").unwrap();
                    }
                }
            }

            logger.debug(&format!("Command: {cmd_line}"));
            println!();
        } else {
            // Fallback if the conversion fails
            logger.debug(&format!(
                "Command: {} {:?}",
                exe_name.to_string_lossy(),
                args
            ));
            println!();
        }
    }

    // If TUI is requested, launch it immediately and skip CLI prompt loop
    if args.tui {
        crate::tui::run(args, &logger)?;
        logger.info("Done.");
        return Ok(());
    }

    // CLI mode: MVU-driven loop
    crate::cli_mvu::run_cli_loop(args, &logger);

    logger.info("Done.");

    Ok(())
}
