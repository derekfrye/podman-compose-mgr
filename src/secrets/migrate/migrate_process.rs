use crate::args::types::Args;
use crate::read_interactive_input::format::{Prompt, PromptGrammar};
use crate::secrets::models::JsonOutput;
use crate::secrets::validation::ui::prompt_for_diff_save_migrate;
use crate::utils::error_utils::ErrorFromStr;
use hostname::get as get_hostname;
use std::io::{self, Write};

/// Process the migration of secrets from remote hosts to localhost
pub fn migrate(args: &Args, entries: &JsonOutput) -> Result<(), Box<dyn std::error::Error>> {
    // Get the current hostname
    let current_hostname = get_hostname()?
        .to_string_lossy()
        .to_string();

    // Find entries that have a different hostname than the current machine
    let foreign_entries: Vec<_> = entries
        .iter()
        .filter(|entry| entry.hostname != current_hostname)
        .collect();

    if foreign_entries.is_empty() {
        println!("No entries found with different hostnames to migrate.");
        return Ok(());
    }

    println!(
        "Found {} entries from other hosts to potentially migrate.",
        foreign_entries.len()
    );

    // Create prompt grammar for migration
    let migrate_prompt = PromptGrammar {
        text: "Migrate".to_string(),
        can_shorten: false,
        display_at_all: true,
        suffix: " ".to_string(),
    };

    let file_name_prompt = PromptGrammar {
        text: "file_name".to_string(),
        can_shorten: true,
        display_at_all: true,
        suffix: "?".to_string(),
    };

    // Process each foreign entry
    for entry in foreign_entries {
        let prompt = Prompt {
            full_prompt: format!("Migrate {} from {} to {}", entry.file_name, entry.hostname, current_hostname),
            grammar: vec![migrate_prompt.clone(), file_name_prompt.clone()],
        };

        // Display entry information
        println!("\nEntry details:");
        println!("  File: {}", entry.file_name);
        println!("  Source Host: {}", entry.hostname);
        println!("  Destination Cloud: {}", entry.destination_cloud);
        
        // Display additional information if available
        if let Some(ref sha) = entry.sha256 {
            println!("  SHA256: {}", sha);
        }
        if let Some(ref last_updated) = entry.last_updated {
            println!("  Last Updated: {}", last_updated);
        }
        
        // Prompt for action
        let response = prompt_for_diff_save_migrate(&prompt, args.verbose > 0)?;
        
        match response.as_str() {
            "Y" | "y" | "S" | "s" => {
                // Call the function to migrate to localhost
                migrate_to_localhost(args, &entry)?;
                println!("Migration of {} completed.", entry.file_name);
            },
            "N" | "n" => {
                println!("Skipping migration of {}.", entry.file_name);
            },
            "D" | "d" | "?" => {
                println!("Migration would retrieve the secret from {} and store it locally.", entry.hostname);
                println!("This will allow you to access the secret from this machine.");
                
                // Ask again
                print!("Proceed with migration? [y/n] ");
                io::stdout().flush()?;
                
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                
                if input.trim().to_lowercase() == "y" {
                    migrate_to_localhost(args, &entry)?;
                    println!("Migration of {} completed.", entry.file_name);
                } else {
                    println!("Skipping migration of {}.", entry.file_name);
                }
            },
            _ => {
                println!("Unexpected response. Skipping migration of {}.", entry.file_name);
            }
        }
    }

    Ok(())
}

/// Migrate a secret from a remote host to localhost
/// This function will be implemented in the future
pub fn migrate_to_localhost(_args: &Args, entry: &crate::secrets::models::JsonEntry) -> Result<(), Box<dyn std::error::Error>> {
    // This function will be implemented later
    println!("Would migrate {} from {} to localhost", entry.file_name, entry.hostname);
    
    // For now, just return an error saying it's not implemented
    Err(Box::new(ErrorFromStr("Secret migration functionality is not yet implemented".to_string())))
}