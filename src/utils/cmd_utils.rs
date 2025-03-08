use std::error::Error;
use std::process::{Command, Output};

use crate::utils::error_utils;

/// Execute a command and return its output as a Result
pub fn run_command(program: &str, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    Command::new(program)
        .args(args)
        .output()
        .map_err(|e| error_utils::into_boxed_error(e, &format!("Failed to execute '{}'", program)))
}

/// Execute a command and return stdout as a string
pub fn run_command_with_output(program: &str, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = run_command(program, args)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(error_utils::new_error(&format!(
            "Command '{}' failed: {}",
            program, stderr
        )));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| error_utils::into_boxed_error(e, "Invalid UTF-8 in command output"))
}

/// Execute a command with logging, returning stdout as a string
pub fn run_command_with_logging(program: &str, args: &[&str]) -> Result<String, Box<dyn Error>> {
    println!("Executing: {} {}", program, args.join(" "));
    let result = run_command_with_output(program, args);

    if let Err(ref e) = result {
        println!("Command failed: {}", e);
    }

    result
}

/// Execute a command, only caring about success/failure
pub fn run_command_checked(program: &str, args: &[&str]) -> Result<(), Box<dyn Error>> {
    run_command_with_output(program, args).map(|_| ())
}

/// Execute a command without capturing output (useful for interactive commands)
pub fn exec_cmd(program: &str, args: &[&str]) -> Result<(), Box<dyn Error>> {
    let status = Command::new(program).args(args).status().map_err(|e| {
        error_utils::into_boxed_error(e, &format!("Failed to execute '{}'", program))
    })?;

    if !status.success() {
        return Err(error_utils::new_error(&format!(
            "Command '{}' exited with non-zero status: {}",
            program, status
        )));
    }

    Ok(())
}

/// Execute a command and return status code
pub fn exec_cmd_with_status(program: &str, args: &[&str]) -> Result<i32, Box<dyn Error>> {
    let status = Command::new(program).args(args).status().map_err(|e| {
        error_utils::into_boxed_error(e, &format!("Failed to execute '{}'", program))
    })?;

    Ok(status.code().unwrap_or(-1))
}
