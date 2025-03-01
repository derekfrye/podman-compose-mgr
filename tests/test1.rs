use std::cell::RefCell;
use std::fs::{self};
use std::path::{Path, PathBuf};


use podman_compose_mgr::interfaces::{CommandHelper, ReadValHelper};
use podman_compose_mgr::read_val::{GrammarFragment, ReadValResult};
use podman_compose_mgr::start::walk_dirs_with_helpers;

use podman_compose_mgr::Args;
use clap::Parser;
use regex::Regex;
// use podman_compose_mgr::{CommandHelper, ReadValHelper};
use serde::Deserialize;

#[test]
fn test1() -> Result<(), Box<dyn std::error::Error>> {
    // Set up the test directory structure
    // let test_dir = "../tests/test1";
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
    
    // Create our mock implementations
    let cmd_helper = TestCommandHelper::new();
    let read_val_helper = TestReadValHelper::new();
    
    // Call the function with our test helpers
    walk_dirs_with_helpers(&args, &cmd_helper, &read_val_helper);
    
    // Get the captured prompts for verification (safely)
    let captured_prompts = read_val_helper.get_captured_prompts();
    
    // Verify at least one prompt was captured
    assert!(!captured_prompts.is_empty(), "No prompts were captured");
    
    // Verify the prompt contains the expected text
    for prompt in &captured_prompts {
        println!("Verifying prompt: {}", prompt);
        if prompt.contains("Refresh") && prompt.contains("djf/rusty-golf") {
            // We found the prompt we're looking for
            assert!(prompt.contains("Refresh"), "Prompt doesn't contain 'Refresh'");
            assert!(prompt.contains("from"), "Prompt doesn't contain 'from'");
            return Ok(());
        }
    }
    
    panic!("Expected prompt with 'Refresh djf/rusty-golf' not found");
}


// Safe mock implementation of CommandHelper for testing using RefCell
struct TestCommandHelper {
    commands_executed: RefCell<Vec<String>>,
}

impl TestCommandHelper {
    fn new() -> Self {
        Self {
            commands_executed: RefCell::new(Vec::new()),
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
    
    fn get_terminal_display_width(&self) -> usize {
        // Always return 80 for tests
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
}

impl ReadValHelper for TestReadValHelper {
    fn read_val_from_cmd_line_and_proceed(&self, grammars: &mut [GrammarFragment]) -> ReadValResult {
        // Construct the prompt from the grammar fragments
        let mut prompt = String::new();
        for grammar in grammars.iter().filter(|g| g.display_at_all) {
            if let Some(prefix) = &grammar.prefix {
                prompt.push_str(prefix);
            }
            
            if grammar.shortened_val_for_prompt.is_some() {
                prompt.push_str(grammar.shortened_val_for_prompt.as_ref().unwrap());
            } else {
                prompt.push_str(grammar.original_val_for_prompt.as_ref().unwrap());
            }
            
            if let Some(suffix) = &grammar.suffix {
                prompt.push_str(suffix);
            }
        }
        
        // Store the captured prompt using RefCell for safe interior mutability
        println!("Captured prompt: {}", prompt);
        self.captured_prompts.borrow_mut().push(prompt);
        
        // For this test, always respond with "?"
        let response = Some("?".to_string());
        
        // If the response is "?", print the help text
        if response.as_deref() == Some("?") {
            println!("p = Pull image from upstream.");
            println!("N = Do nothing, skip this image.");
            println!("d = Display info (image name, docker-compose.yml path, upstream img create date, and img on-disk modify date).");
            println!("b = Build image from the Dockerfile residing in same path as the docker-compose.yml.");
            println!("s = Skip all subsequent images with this same name (regardless of container name).");
            println!("? = Display this help.");
        }
        
        ReadValResult {
            user_entered_val: response,
        }
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
    // Optionally, if you need the cargo args, you could add a field like:
    // cargo: Option<Cargo>,
}

