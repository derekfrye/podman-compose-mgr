use podman_compose_mgr::interfaces::MockReadInteractiveInputHelper;
use podman_compose_mgr::read_interactive_input::ReadValResult;
use podman_compose_mgr::secrets::upload::prompt_for_upload_with_helper;

#[test]
fn test_prompt_for_upload_with_varying_terminal_sizes() -> Result<(), Box<dyn std::error::Error>> {
    // Test parameters
    let file_path = "/path/to/test/file.txt";
    let encoded_name = "test-secret-name";
    let size_kib = 1.5;
    
    // FIRST TEST: Terminal width 60
    {
        // Test with "Y" response (upload)
        let mut read_val_helper = MockReadInteractiveInputHelper::new();
        read_val_helper
            .expect_read_val_from_cmd_line_and_proceed()
            .returning(|grammars, _size| {
                // Format the prompt for verification
                let mut grammars_copy = grammars.to_vec();
                let _ = podman_compose_mgr::read_interactive_input::do_prompt_formatting(
                    &mut grammars_copy,
                    60
                );
                let formatted = podman_compose_mgr::read_interactive_input::unroll_grammar_into_string(
                    &grammars_copy,
                    false,
                    true
                );
                println!("prompt (width 60): {}", formatted);
                
                ReadValResult {
                    user_entered_val: Some("Y".to_string()),
                }
            });
        
        // Call the function with our test helpers
        let result = prompt_for_upload_with_helper(
            file_path,
            encoded_name,
            size_kib,
            &read_val_helper
        )?;
        
        assert!(result, "Expected true (upload) for width 60 with 'Y' response");
        
        // Test with "n" response (skip)
        let mut read_val_helper = MockReadInteractiveInputHelper::new();
        read_val_helper
            .expect_read_val_from_cmd_line_and_proceed()
            .returning(|grammars, _size| {
                // Format the prompt for verification
                let mut grammars_copy = grammars.to_vec();
                let _ = podman_compose_mgr::read_interactive_input::do_prompt_formatting(
                    &mut grammars_copy,
                    60
                );
                let formatted = podman_compose_mgr::read_interactive_input::unroll_grammar_into_string(
                    &grammars_copy,
                    false,
                    true
                );
                println!("prompt (width 60): {}", formatted);
                
                ReadValResult {
                    user_entered_val: Some("n".to_string()),
                }
            });
        
        let result = prompt_for_upload_with_helper(
            file_path,
            encoded_name,
            size_kib,
            &read_val_helper
        )?;
        
        assert!(!result, "Expected false (skip) for width 60 with 'n' response");
        
        // Test with "d" then "Y" responses (display details then upload)
        let mut d_then_y_counter = 0;
        let mut read_val_helper = MockReadInteractiveInputHelper::new();
        read_val_helper
            .expect_read_val_from_cmd_line_and_proceed()
            .times(2)
            .returning(move |grammars, _size| {
                // Create a copy of the grammars to format
                let mut grammars_copy = grammars.to_vec();
                let _ = podman_compose_mgr::read_interactive_input::do_prompt_formatting(
                    &mut grammars_copy,
                    60
                );
                let formatted = podman_compose_mgr::read_interactive_input::unroll_grammar_into_string(
                    &grammars_copy, 
                    false,
                    true
                );
                println!("prompt (width 60): {}", formatted);
                
                // First return "d", then return "Y"
                d_then_y_counter += 1;
                if d_then_y_counter == 1 {
                    ReadValResult {
                        user_entered_val: Some("d".to_string()),
                    }
                } else {
                    ReadValResult {
                        user_entered_val: Some("Y".to_string()),
                    }
                }
            });
        
        let result = prompt_for_upload_with_helper(
            file_path,
            encoded_name,
            size_kib,
            &read_val_helper
        )?;
        
        assert!(result, "Expected true (upload) for width 60 with 'd' then 'Y' responses");
        
        // Test with "?" then "n" responses (help then skip)
        let mut help_then_n_counter = 0;
        let mut read_val_helper = MockReadInteractiveInputHelper::new();
        read_val_helper
            .expect_read_val_from_cmd_line_and_proceed()
            .times(2)
            .returning(move |grammars, _size| {
                // Create a copy of the grammars to format
                let mut grammars_copy = grammars.to_vec();
                let _ = podman_compose_mgr::read_interactive_input::do_prompt_formatting(
                    &mut grammars_copy,
                    60
                );
                let formatted = podman_compose_mgr::read_interactive_input::unroll_grammar_into_string(
                    &grammars_copy, 
                    false,
                    true
                );
                println!("prompt (width 60): {}", formatted);
                
                // First return "?", then return "n"
                help_then_n_counter += 1;
                if help_then_n_counter == 1 {
                    ReadValResult {
                        user_entered_val: Some("?".to_string()),
                    }
                } else {
                    ReadValResult {
                        user_entered_val: Some("n".to_string()),
                    }
                }
            });
        
        let result = prompt_for_upload_with_helper(
            file_path,
            encoded_name,
            size_kib,
            &read_val_helper
        )?;
        
        assert!(!result, "Expected false (skip) for width 60 with '?' then 'n' responses");
    }
    
    // SECOND TEST: Terminal width 40
    {
        // Test with "Y" response (upload)
        let mut read_val_helper = MockReadInteractiveInputHelper::new();
        read_val_helper
            .expect_read_val_from_cmd_line_and_proceed()
            .returning(|grammars, _size| {
                // Format the prompt for verification
                let mut grammars_copy = grammars.to_vec();
                let _ = podman_compose_mgr::read_interactive_input::do_prompt_formatting(
                    &mut grammars_copy,
                    40
                );
                let formatted = podman_compose_mgr::read_interactive_input::unroll_grammar_into_string(
                    &grammars_copy,
                    false,
                    true
                );
                println!("prompt (width 40): {}", formatted);
                
                ReadValResult {
                    user_entered_val: Some("Y".to_string()),
                }
            });
        
        let result = prompt_for_upload_with_helper(
            file_path,
            encoded_name,
            size_kib,
            &read_val_helper
        )?;
        
        assert!(result, "Expected true (upload) for width 40 with 'Y' response");
        
        // Test with invalid then valid response
        let mut invalid_then_n_counter = 0;
        let mut read_val_helper = MockReadInteractiveInputHelper::new();
        read_val_helper
            .expect_read_val_from_cmd_line_and_proceed()
            .times(2)
            .returning(move |grammars, _size| {
                // Create a copy of the grammars to format
                let mut grammars_copy = grammars.to_vec();
                let _ = podman_compose_mgr::read_interactive_input::do_prompt_formatting(
                    &mut grammars_copy,
                    40
                );
                let formatted = podman_compose_mgr::read_interactive_input::unroll_grammar_into_string(
                    &grammars_copy, 
                    false,
                    true
                );
                println!("prompt (width 40): {}", formatted);
                
                // First return invalid "x", then return "n"
                invalid_then_n_counter += 1;
                if invalid_then_n_counter == 1 {
                    ReadValResult {
                        user_entered_val: Some("x".to_string()),
                    }
                } else {
                    ReadValResult {
                        user_entered_val: Some("n".to_string()),
                    }
                }
            });
        
        let result = prompt_for_upload_with_helper(
            file_path,
            encoded_name,
            size_kib,
            &read_val_helper
        )?;
        
        assert!(!result, "Expected false (skip) for width 40 with 'x' then 'n' responses");
    }
    
    Ok(())
}