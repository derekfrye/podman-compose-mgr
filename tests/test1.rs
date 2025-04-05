use std::fs::{self};

use podman_compose_mgr::interfaces::{MockCommandHelper, MockReadValHelper};
use podman_compose_mgr::read_val::ReadValResult;
use podman_compose_mgr::start::walk_dirs_with_helpers;

use clap::Parser;
use podman_compose_mgr::Args;
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
    let config = launch_json
        .configurations
        .into_iter()
        .find(|c| c.name == "Rebuild")
        .ok_or("Configuration 'Rebuild' not found in launch.json")?;
    let mut clap_args = vec!["dummy_binary".to_string()];
    clap_args.extend(config.args);
    let args = Args::parse_from(clap_args); 

    // FIRST TEST: Width 60
    // Create a mockall implementation with width 60
    let mut cmd_helper = MockCommandHelper::new();
    cmd_helper
        .expect_get_terminal_display_width()
        .returning(|_| 60);
    cmd_helper
        .expect_file_exists_and_readable()
        .returning(|_| true);
    cmd_helper.expect_exec_cmd().returning(|cmd, args| {
        println!("Mock exec_cmd called with: {} {}", cmd, args.join(" "));
        Ok(())
    });
    cmd_helper.expect_pull_base_image().returning(|_| Ok(()));

    // Setup read_val_helper 
    let mut read_val_helper = MockReadValHelper::new();

    read_val_helper
        .expect_read_val_from_cmd_line_and_proceed()
        .returning(|grammars, _size| {
            // Create a copy of the grammars to format using the actual formatting functions
            let mut grammars_copy = grammars.to_vec();
            
            // Run the actual formatting logic used in production and print the result
            // Use explicit width instead of MockCommandHelper for test_format_prompt
            let _ = podman_compose_mgr::read_val::do_prompt_formatting(
                &mut grammars_copy, 
                60
            );
            let formatted = podman_compose_mgr::read_val::unroll_grammar_into_string(
                &grammars_copy, 
                false, 
                true
            );
            println!("prompt (width 60): {}", formatted);

            // Return a result with "N" as user input
            ReadValResult {
                user_entered_val: Some("N".to_string()),
            }
        })
        .times(3);

    // Call the function with our test helpers
    walk_dirs_with_helpers(&args, &cmd_helper, &read_val_helper)?;

    // SECOND TEST: Width 40
    // Create a new mock with width 40
    let mut cmd_helper = MockCommandHelper::new();
    cmd_helper
        .expect_get_terminal_display_width()
        .returning(|_| 40);
    cmd_helper
        .expect_file_exists_and_readable()
        .returning(|_| true);
    cmd_helper.expect_exec_cmd().returning(|cmd, args| {
        println!("Mock exec_cmd called with: {} {}", cmd, args.join(" "));
        Ok(())
    });
    cmd_helper.expect_pull_base_image().returning(|_| Ok(()));

    // Set up a new mock read_val_helper
    let mut read_val_helper = MockReadValHelper::new();
    
    read_val_helper
        .expect_read_val_from_cmd_line_and_proceed()
        .returning(|grammars, _size| {
            // Create a copy of the grammars to format using the actual formatting functions
            let mut grammars_copy = grammars.to_vec();
            
            // Run the actual formatting logic used in production and print the result
            // Use explicit width instead of MockCommandHelper for test_format_prompt
            let _ = podman_compose_mgr::read_val::do_prompt_formatting(
                &mut grammars_copy, 
                40
            );
            let formatted = podman_compose_mgr::read_val::unroll_grammar_into_string(
                &grammars_copy, 
                false, 
                true
            );
            println!("prompt (width 40): {}", formatted);

            // Return a result with "N" as user input
            ReadValResult {
                user_entered_val: Some("N".to_string()),
            }
        })
        .times(3);

    // Call the function with our test helpers
    walk_dirs_with_helpers(&args, &cmd_helper, &read_val_helper)?;

    // The test is successful if we made it here
    Ok(())
}

// Using mockall-generated mocks now, so we can delete the custom test implementations

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
