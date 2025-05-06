use podman_compose_mgr::{
    args::{self, Mode},
    secrets,
    utils::log_utils::Logger,
    walk_dirs::walk_dirs,
};

// use futures::executor;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn main() -> io::Result<()> {
    // Set up a global Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Use ctrlc crate to handle Ctrl+C globally
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
        println!("\nOperation cancelled by user");
        std::process::exit(0);
    })
    .expect("Error setting Ctrl+C handler");

    // Parse command-line arguments
    let args = args::args_checks();
    if let Err(e) = args.validate() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    // Create logger instance
    let logger = Logger::new(args.verbose);

    // If double verbose, print the command line in a copy-paste friendly format
    // Get the program name
    let exe_path =
        std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("podman-compose-mgr"));
    let exe_name = exe_path
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("podman-compose-mgr"));

    // Start building the command line
    let mut cmd_line = format!("{}", exe_name.to_string_lossy());

    // Use serde to convert Args to a JSON value for inspection
    let args_json = serde_json::to_value(&args).unwrap_or_else(|_| serde_json::Value::Null);

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
                let count = map.get(key).and_then(|v| v.as_u64()).unwrap_or(0);
                for _ in 0..count {
                    cmd_line.push_str(" --verbose");
                }
                continue;
            }

            match map.get(key) {
                Some(serde_json::Value::Null) => {
                    // Skip null values (None options)
                }
                Some(serde_json::Value::Array(arr)) if arr.is_empty() => {
                    // Skip empty arrays
                }
                Some(serde_json::Value::Array(arr)) => {
                    // Format arrays (e.g., vectors)
                    for item in arr {
                        let escaped_value = match item {
                            serde_json::Value::String(s) => format!("\"{}\"", s),
                            _ => item.to_string(),
                        };
                        cmd_line.push_str(&format!(" --{} {}", arg_key, escaped_value));
                    }
                }
                Some(serde_json::Value::String(s)) if s.is_empty() => {
                    // Skip empty strings
                }
                Some(serde_json::Value::Bool(true)) => {
                    cmd_line.push_str(&format!(" --{}", arg_key));
                }
                Some(serde_json::Value::Bool(false)) => {
                    // Skip false boolean values
                }
                Some(value) => {
                    // Format everything else
                    let escaped_value = match value {
                        serde_json::Value::String(s) => format!("\"{}\"", s),
                        _ => value.to_string(),
                    };
                    cmd_line.push_str(&format!(" --{} {}", arg_key, escaped_value));
                }
                None => {
                    // Skip if the key doesn't exist (shouldn't happen)
                }
            }
        }

        logger.debug(&format!("Command: {}", cmd_line));
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

    match args.mode {
        Mode::SecretRetrieve | Mode::SecretInitialize | Mode::SecretUpload | Mode::SecretMigrate => {
            if let Err(e) = secrets::process_secrets_mode(&args, &logger) {
                eprintln!("Error processing secrets: {}", e);
                std::process::exit(1);
            }
        }
        _ => {
            walk_dirs(&args, &logger, args.tui);
        }
    }

    logger.info("Done.");

    Ok(())
}
