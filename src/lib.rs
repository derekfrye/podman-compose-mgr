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
pub mod cli_mvu;
pub mod image_build;
pub mod interfaces;
pub mod mvu;
pub mod read_interactive_input;

pub mod testing;
pub mod tui;
pub mod utils;
#[cfg(debug_assertions)]
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

    match args.podman_bin.as_ref() {
        Some(path) => {
            crate::utils::podman_utils::set_podman_binary_override(path.clone().into_os_string());
        }
        None => crate::utils::podman_utils::clear_podman_binary_override(),
    }

    let logger = Logger::new(args.verbose);
    log_command_line(args, &logger);

    if let Some(sim_mode) = args.tui_simulate {
        crate::tui::simulate_view(args, sim_mode, &logger)?;
        logger.info("Done.");
        return Ok(());
    }

    if args.one_shot.enabled() {
        crate::cli_mvu::run_one_shot(args, &logger);
        logger.info("Done.");
        return Ok(());
    }

    if args.tui.enabled() {
        crate::tui::run(args, &logger)?;
        logger.info("Done.");
        return Ok(());
    }

    crate::cli_mvu::run_cli_loop(args, &logger);
    logger.info("Done.");

    Ok(())
}

fn log_command_line(args: &args::Args, logger: &crate::utils::log_utils::Logger) {
    if args.verbose < 2 {
        return;
    }

    let exe_name = current_exe_name();
    if let Some(cmd_line) = build_command_line(args, &exe_name) {
        logger.debug(&format!("Command: {cmd_line}"));
    } else {
        logger.debug(&format!(
            "Command: {} {:?}",
            exe_name.to_string_lossy(),
            args
        ));
    }

    println!();
}

fn current_exe_name() -> std::ffi::OsString {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.file_name().map(std::ffi::OsStr::to_os_string))
        .unwrap_or_else(|| std::ffi::OsString::from("podman-compose-mgr"))
}

fn build_command_line(args: &args::Args, exe_name: &std::ffi::OsStr) -> Option<String> {
    let mut cmd_line = exe_name.to_string_lossy().to_string();
    let args_json = serde_json::to_value(args).ok()?;
    let map = args_json.as_object()?;
    let mut keys: Vec<_> = map.keys().collect();
    keys.sort();

    for key in keys {
        let arg_key = key.replace('_', "-");

        if key == "verbose" {
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
            Some(serde_json::Value::Null | serde_json::Value::Bool(false)) | None => {}
            Some(serde_json::Value::Array(arr)) if arr.is_empty() => {}
            Some(serde_json::Value::String(s)) if s.is_empty() => {}
            Some(serde_json::Value::Array(arr)) => {
                for item in arr {
                    let escaped_value = format_cli_value(item);
                    write!(cmd_line, " --{arg_key} {escaped_value}").unwrap();
                }
            }
            Some(serde_json::Value::Bool(true)) => {
                write!(cmd_line, " --{arg_key}").unwrap();
            }
            Some(value) => {
                let escaped_value = format_cli_value(value);
                write!(cmd_line, " --{arg_key} {escaped_value}").unwrap();
            }
        }
    }

    Some(cmd_line)
}

fn format_cli_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => format!("\"{s}\""),
        _ => value.to_string(),
    }
}
