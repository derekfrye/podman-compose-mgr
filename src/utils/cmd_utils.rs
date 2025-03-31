use std::error::Error;
use std::process::{Command, Output};
use std::io::{BufReader, Read};
use std::path::Path;
use terminal_size::{self, Width};

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

/// Parse Dockerfile and pull base image
pub fn pull_base_image(dockerfile: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    // Use the error utils to handle file opening errors
    let file = std::fs::File::open(dockerfile).map_err(|e| {
        crate::utils::error_utils::into_boxed_error(
            e,
            &format!("Failed to open Dockerfile: {}", dockerfile.display()),
        )
    })?;

    let mut reader = BufReader::new(file);

    let mut content = String::new();
    reader.read_to_string(&mut content).map_err(|e| {
        crate::utils::error_utils::into_boxed_error(e, "Failed to read Dockerfile contents")
    })?;

    let dockerfile = dockerfile_parser::Dockerfile::parse(&content).map_err(|e| {
        crate::utils::error_utils::into_boxed_error(e, "Failed to parse Dockerfile")
    })?;

    let from_img = dockerfile.instructions;
    let image_name = from_img
        .iter()
        .find_map(|instruction| {
            if let dockerfile_parser::Instruction::From(image, ..) = instruction {
                Some(image.image.clone())
            } else {
                None
            }
        })
        .ok_or_else(|| crate::utils::error_utils::new_error("No base image found in Dockerfile"))?;

    // Use the new command utilities
    // Convert SpannedString to regular String for the command args
    let image_name_str = image_name.to_string();
    run_command_checked("podman", &["pull", &image_name_str])
}

/// exists(), is_file() traversing links, and metadata.is_ok() traversing links
pub fn file_exists_and_readable(file: &Path) -> bool {
    match file.try_exists() {
        Ok(true) => file.is_file() && file.metadata().is_ok(),
        _ => false,
    }
}

/// Get terminal display width or fallback to default
pub fn get_terminal_display_width(specify_size: Option<usize>) -> usize {
    if let Some(size) = specify_size {
        return size;
    }
    let size = terminal_size::terminal_size();
    if let Some((Width(w), _)) = size {
        w as usize
    } else {
        80
    }
}
