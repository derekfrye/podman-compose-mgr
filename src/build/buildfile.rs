use crate::read_val::GrammarFragment;
use walkdir::DirEntry;
use crate::build::buildfile_types::{BuildFile, WhatWereBuilding};
use crate::build::buildfile_error::BuildfileError;
use crate::build::buildfile_helpers::{setup_prompts, handle_display_info, handle_file_type_choice, find_buildfile};
use crate::build::buildfile_build::build_image_from_spec;

pub fn start(dir: &DirEntry, custom_img_nm: &str, build_args: Vec<&str>) -> Result<(), BuildfileError> {
    let buildfiles = find_buildfile(dir, custom_img_nm, build_args);
    if buildfiles.is_none()
        || buildfiles.as_ref().unwrap().is_empty()
        || buildfiles
            .as_ref()
            .unwrap()
            .iter()
            .all(|file| file.filepath.is_none())
    {
        eprintln!(
            "No Dockerfile or Makefile found at '{}'",
            dir.path().display()
        );
        // std::process::exit(1);
    } else if let Some(found_buildfiles) = buildfiles {
        let build_config = read_val_loop(found_buildfiles);
        // dbg!(&build_config);

        if build_config.file.filepath.is_some() {
            build_image_from_spec(build_config)?;
        }
    }
    Ok(())
}

/// Process user choice in build selection
fn process_build_choice<'a>(
    choice: &str,
    files: &[BuildFile],
    buildfile: &BuildFile,
    user_choices: &[&'a str],
    are_there_multiple_files: bool,
    choice_of_where_to_build: &mut WhatWereBuilding,
    _prompt_grammars: &mut [GrammarFragment]
) -> Option<(bool, Vec<&'a str>, bool)> {
    match choice {
        // Handle Dockerfile or Makefile choice
        "D" | "M" => {
            if let Some((chosen_file, _new_prompts, new_choices)) = 
                handle_file_type_choice(files, choice, buildfile) {
                choice_of_where_to_build.file = chosen_file;
                
                // Return values to update in the caller
                Some((false, new_choices, false))
            } else {
                None
            }
        }
        // Display help or info
        "d" | "?" => {
            handle_display_info(files, buildfile, user_choices, are_there_multiple_files);
            None
        }
        // Building at link target
        "1" => {
            choice_of_where_to_build.follow_link = true;
            Some((true, Vec::new(), true)) // Exit loop
        }
        // Building in dir symlink lives, not link target
        "2" => {
            choice_of_where_to_build.follow_link = false;
            Some((true, Vec::new(), true)) // Exit loop
        }
        _ => {
            eprintln!("Invalid choice '{}'", choice);
            None
        }
    }
}

fn read_val_loop(files: Vec<BuildFile>) -> WhatWereBuilding {
    let buildfile = files[0].clone();
    let (mut prompt_grammars, mut user_choices, are_there_multiple_files) = setup_prompts(&files);
    
    let mut choice_of_where_to_build: WhatWereBuilding = WhatWereBuilding {
        file: buildfile.clone(),
        follow_link: false,
    };
    
    if !prompt_grammars.is_empty() {
        loop {
            // Display prompt and get user input
            let result = crate::read_val::read_val_from_cmd_line_and_proceed_default(&mut prompt_grammars);
            
            if let Some(user_choice) = result.user_entered_val {
                // Process the user's choice
                if let Some((should_break, new_choices, _is_final_choice)) = 
                    process_build_choice(
                        &user_choice,
                        &files,
                        &buildfile,
                        &user_choices,
                        are_there_multiple_files,
                        &mut choice_of_where_to_build,
                        &mut prompt_grammars
                    ) {
                    
                    // If final choice made, exit the loop
                    if should_break {
                        break;
                    }
                    
                    // Update user choices and grammar if needed
                    if !new_choices.is_empty() {
                        user_choices = new_choices;
                        prompt_grammars.extend(crate::build::buildfile_helpers::make_choice_grammar(&user_choices, user_choice.len() as u8));
                    }
                }
            }
        }
    }
    
    choice_of_where_to_build
}