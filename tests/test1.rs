use std::cell::RefCell;
use std::io::{self};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use podman_compose_mgr::args::Mode;
use podman_compose_mgr::interfaces::{CommandHelper, ReadValHelper};
use podman_compose_mgr::read_val::{GrammarFragment, ReadValResult};
use podman_compose_mgr::start::walk_dirs_with_helpers;
use podman_compose_mgr::Args;

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
    
    fn get_commands_executed(&self) -> Vec<String> {
        self.commands_executed.borrow().clone()
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

#[test]
fn test1() -> io::Result<()> {
    // Set up the test directory structure
    let workspace_folder = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let test_dir = workspace_folder.join("tests/test1");
    
    // Create a test Args instance
    let args = Args {
        path: test_dir,
        mode: Mode::Rebuild,
        verbose: true,
        exclude_path_patterns: vec!["archive".to_string()],
        include_path_patterns: vec![],
        build_args: vec!["USERNAME=`id -un 1000`".to_string()],
        secrets_tmp_dir: None,
        secrets_client_id: None,
        secrets_client_secret_path: None,
        secrets_tenant_id: None,
        secrets_vault_name: None,
        secret_mode_output_json: None,
        secret_mode_input_json: None,
    };
    
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
