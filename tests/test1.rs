use std::cell::RefCell;
use std::fs::{self};
use std::path::{Path, PathBuf};


use podman_compose_mgr::interfaces::{CommandHelper, ReadValHelper};
use podman_compose_mgr::read_val::{GrammarFragment, ReadValResult};
use podman_compose_mgr::start::walk_dirs_with_helpers;

use podman_compose_mgr::Args;
use clap::Parser;
use regex::Regex;
use serde::Deserialize;

#[test]
fn test1() -> Result<(), Box<dyn std::error::Error>> {
    // Set up the test directory structure
    let contents = fs::read_to_string(".vscode/launch.json")?;
    let re = Regex::new(r"^\s+//").unwrap();
    
    // Filter out the lines matching the regex
    let filtered: String = contents
        .lines()
        .filter(|line| !re.is_match(line))
        .collect::<Vec<&str>>()
        .join("\n");

    let filtered = filtered.replace("${env:HOME}/docker", "tests/test1");

    let launch_json: LaunchJson = serde_json::from_str(&filtered)?;
    let config = launch_json.configurations
        .into_iter()
        .find(|c| c.name == "Rebuild")
        .ok_or("Configuration 'Rebuild' not found in launch.json")?;
    let mut clap_args = vec!["dummy_binary".to_string()];
    clap_args.extend(config.args);
    let args = Args::parse_from(clap_args);
    
    // FIRST TEST: Width 60
    // Create our mock implementations with width 60
    let cmd_helper = TestCommandHelper::new_with_width(Some(60));
    let read_val_helper = TestReadValHelper::new();
    
    // Call the function with our test helpers
    walk_dirs_with_helpers(&args, &cmd_helper, &read_val_helper);
    
    // Get the captured prompts for verification
    let captured_prompts = read_val_helper.get_captured_prompts();
    
    // We should have 3 prompts with width 60
    assert_eq!(captured_prompts.len(), 3);
    
    // Verify width 60 prompts
    let mut found_width_60_prompt = false;
    for prompt in &captured_prompts {
        println!("Verifying width 60 prompt: {}", prompt);
        if prompt.contains("Refresh") && prompt.contains("djf/rusty-g") {
            assert!(prompt.contains("from"), "Prompt doesn't contain 'from'");
            assert!(prompt.len() <= 60, "Width 60 prompt longer than expected");
            found_width_60_prompt = true;
            break;
        }
    }
    
    assert!(found_width_60_prompt, "Width 60 prompt not found");

    // SECOND TEST: Width 40
    // Clear captured prompts before second test
    read_val_helper.clear_captured_prompts();
    
    // Create command helper with width 40
    let cmd_helper = TestCommandHelper::new_with_width(Some(40));

    // Call the function with our test helpers
    walk_dirs_with_helpers(&args, &cmd_helper, &read_val_helper);
    
    // Get the captured prompts for verification
    let captured_prompts = read_val_helper.get_captured_prompts();
    
    // Verify 3 prompts with width 40
    assert_eq!(captured_prompts.len(), 3);
    
    // Verify width 40 prompts
    let mut found_width_40_prompt = false;
    for prompt in &captured_prompts {
        println!("Verifying width 40 prompt: {}", prompt);
        if prompt.contains("Refresh") && prompt.contains("d...") {
            assert!(prompt.contains("from"), "Prompt doesn't contain 'from'");
            assert!(prompt.len() <= 40, "Width 40 prompt longer than expected");
            found_width_40_prompt = true;
            break;
        }
    }
    
    assert!(found_width_40_prompt, "Width 40 prompt not found");
    
    // The test is successful if we made it here
    Ok(())
}

// Safe mock implementation of CommandHelper for testing using RefCell
struct TestCommandHelper {
    commands_executed: RefCell<Vec<String>>,
    width: Option<usize>,
}

impl TestCommandHelper {
    fn new_with_width(width: Option<usize>) -> Self {
        Self {
            commands_executed: RefCell::new(Vec::new()),
            width,
        }
    }
}

impl CommandHelper for TestCommandHelper {
    fn exec_cmd(&self, cmd: &str, args: Vec<String>) {
        let command = format!("{} {}", cmd, args.join(" "));
        println!("Mock exec_cmd called with: {}", command);
        
        // Use RefCell for interior mutability
        self.commands_executed.borrow_mut().push(command);
    }
    
    fn pull_base_image(&self, dockerfile: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        println!("Mock pull_base_image called with: {:?}", dockerfile);
        Ok(())
    }
    
    fn get_terminal_display_width(&self, siz: Option<usize>) -> usize {
        // First priority: explicit size passed as parameter
        if let Some(s) = siz {
            return s;
        }
        
        // Second priority: width set in the TestCommandHelper instance
        if let Some(w) = self.width {
            return w;
        }
        
        // Default fallback
        80
    }
    
    fn file_exists_and_readable(&self, file: &Path) -> bool {
        println!("Mock file_exists_and_readable called with: {:?}", file);
        // Return true for files we expect to exist in the test
        true
    }
}

// Safe mock implementation of ReadValHelper for testing using RefCell
struct TestReadValHelper {
    captured_prompts: RefCell<Vec<String>>,
}

impl TestReadValHelper {
    fn new() -> Self {
        Self {
            captured_prompts: RefCell::new(Vec::new()),
        }
    }
    
    fn get_captured_prompts(&self) -> Vec<String> {
        self.captured_prompts.borrow().clone()
    }
    
    fn clear_captured_prompts(&self) {
        self.captured_prompts.borrow_mut().clear();
    }
    
    // Function to capture prints during test
    fn test_print(&self, s: &str) {
        // Store the printed text in our captured_prompts
        self.captured_prompts.borrow_mut().push(s.to_string());
        // Also print to console for debugging
        println!("Captured print: {}", s);
    }
}

// Monkey patch the test_print_fn to capture output in the TestReadValHelper
impl ReadValHelper for TestReadValHelper {
    fn read_val_from_cmd_line_and_proceed(&self, grammars: &mut [GrammarFragment], size: Option<usize>) -> ReadValResult {
        println!("ReadValHelper called with width: {:?}", size);
        
        // Use the provided size parameter for testing
        let cmd_helper = TestCommandHelper::new_with_width(size);
        
        // Now we can use a closure that captures self for the print function
        let print_fn = Box::new(|s: &str| self.test_print(s));
        
        // Create a test stdin helper that returns "N"
        let test_stdin = podman_compose_mgr::read_val::TestStdinHelper {
            response: "N".to_string()
        };
        
        // Now we can directly test the function with dependency injection
        podman_compose_mgr::read_val::read_val_from_cmd_line_and_proceed_with_deps(
            grammars,
            &cmd_helper,
            print_fn,
            size,  // Pass the actual size parameter through
            Some(&test_stdin)
        )
    }
}

#[derive(Debug, Deserialize)]
struct LaunchJson {
    // version: String,
    configurations: Vec<Configuration>,
}

// This struct represents a configuration block in launch.json.
#[derive(Debug, Deserialize)]
struct Configuration {
    name: String,
    // This field will capture the command-line arguments.
    #[serde(default)]
    args: Vec<String>,
}