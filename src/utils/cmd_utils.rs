use std::error::Error;
use std::ffi::OsString;
use std::process::{Command, Output};

use crate::utils::error_utils;

fn resolve_program(program: &str) -> OsString {
    if program == "podman" {
        crate::utils::podman_utils::resolve_podman_binary()
    } else {
        OsString::from(program)
    }
}

fn build_command(program: &str) -> Command {
    Command::new(resolve_program(program))
}

/// Execute a command and return its output as a Result
///
/// # Arguments
///
/// * `program` - Program to execute
/// * `args` - Arguments to pass to the program
///
/// # Returns
///
/// * `Result<Output, Box<dyn Error>>` - Command output or error
///
/// # Errors
///
/// Returns an error if the command fails to execute.
pub fn run_command(program: &str, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    build_command(program)
        .args(args)
        .output()
        .map_err(|e| error_utils::into_boxed_error(e, &format!("Failed to execute '{program}'")))
}

/// Execute a command and return stdout as a string
///
/// # Arguments
///
/// * `program` - Program to execute
/// * `args` - Arguments to pass to the program
///
/// # Returns
///
/// * `Result<String, Box<dyn Error>>` - Command stdout or error
///
/// # Errors
///
/// Returns an error if the command fails to execute or returns non-zero exit code.
pub fn run_command_with_output(program: &str, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = run_command(program, args)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(error_utils::new_error(&format!(
            "Command '{program}' failed: {stderr}"
        )));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| error_utils::into_boxed_error(e, "Invalid UTF-8 in command output"))
}

/// Execute a command with logging, returning stdout as a string
///
/// # Arguments
///
/// * `program` - Program to execute
/// * `args` - Arguments to pass to the program
///
/// # Returns
///
/// * `Result<String, Box<dyn Error>>` - Command stdout or error
///
/// # Errors
///
/// Returns an error if the command fails to execute or returns non-zero exit code.
pub fn run_command_with_logging(program: &str, args: &[&str]) -> Result<String, Box<dyn Error>> {
    println!("Executing: {} {}", program, args.join(" "));
    let result = run_command_with_output(program, args);

    if let Err(ref e) = result {
        println!("Command failed: {e}");
    }

    result
}

/// Execute a command, only caring about success/failure
///
/// # Arguments
///
/// * `program` - Program to execute
/// * `args` - Arguments to pass to the program
///
/// # Returns
///
/// * `Result<(), Box<dyn Error>>` - Success or error
///
/// # Errors
///
/// Returns an error if the command fails to execute or returns non-zero exit code.
pub fn run_command_checked(program: &str, args: &[&str]) -> Result<(), Box<dyn Error>> {
    run_command_with_output(program, args).map(|_| ())
}

/// Execute a command without capturing output (useful for interactive commands)
///
/// # Arguments
///
/// * `program` - Program to execute
/// * `args` - Arguments to pass to the program
///
/// # Returns
///
/// * `Result<(), Box<dyn Error>>` - Success or error
///
/// # Errors
///
/// Returns an error if the command fails to execute or returns non-zero exit code.
pub fn exec_cmd(program: &str, args: &[&str]) -> Result<(), Box<dyn Error>> {
    let status = build_command(program)
        .args(args)
        .status()
        .map_err(|e| error_utils::into_boxed_error(e, &format!("Failed to execute '{program}'")))?;

    if !status.success() {
        return Err(error_utils::new_error(&format!(
            "Command '{program}' exited with non-zero status: {status}"
        )));
    }

    Ok(())
}

/// Execute a command and return status code
///
/// # Arguments
///
/// * `program` - Program to execute
/// * `args` - Arguments to pass to the program
///
/// # Returns
///
/// * `Result<i32, Box<dyn Error>>` - Exit status code or error
///
/// # Errors
///
/// Returns an error if the command fails to execute.
pub fn exec_cmd_with_status(program: &str, args: &[&str]) -> Result<i32, Box<dyn Error>> {
    let status = build_command(program)
        .args(args)
        .status()
        .map_err(|e| error_utils::into_boxed_error(e, &format!("Failed to execute '{program}'")))?;

    Ok(status.code().unwrap_or(-1))
}
