use super::super::types::{BuildChoice, BuildFile, WhatWereBuilding};
use super::grammar::{make_build_prompt_grammar, make_choice_grammar};
use super::setup::buildfile_prompt_grammars;

pub fn read_val_loop(files: &[BuildFile]) -> WhatWereBuilding {
    let (mut prompt_grammars, user_choices) = buildfile_prompt_grammars(files);

    let mut choice_of_where_to_build = WhatWereBuilding {
        file: files[0].clone(),
        follow_link: false,
    };

    if !prompt_grammars.is_empty() {
        loop {
            let result = crate::read_interactive_input::read_val_from_cmd_line_and_proceed_default(
                &mut prompt_grammars,
            );
            if let Some(choice) = result.user_entered_val {
                match choice.as_str() {
                    "D" | "M" => {
                        if files.len() > 1 {
                            choice_of_where_to_build.file = files
                                .iter()
                                .find(|x| {
                                    x.filetype
                                        == (match choice.as_str() {
                                            "M" => BuildChoice::Makefile,
                                            _ => BuildChoice::Dockerfile,
                                        })
                                })
                                .unwrap()
                                .clone();
                            prompt_grammars = make_build_prompt_grammar(&files[0]);
                            let user_choices = vec!["1", "2", "d", "?"];

                            prompt_grammars.extend(make_choice_grammar(
                                &user_choices,
                                u8::try_from(prompt_grammars.len()).unwrap_or(255),
                            ));
                        } else {
                            eprintln!(
                                "No {} found at '{}'",
                                match choice.as_str() {
                                    "M" => "Makefile",
                                    _ => "Dockerfile",
                                },
                                files[0].parent_dir.display()
                            );
                        }
                    }
                    "d" | "?" => {
                        for f in files
                            .iter()
                            .filter(|f| f.filetype == BuildChoice::Dockerfile)
                        {
                            let dockerfile = &f.filepath.as_ref().unwrap().to_str().unwrap();
                            println!("Dockerfile: {dockerfile}");
                        }
                        for f in files.iter().filter(|f| f.filetype == BuildChoice::Makefile) {
                            let makefile = &f.filepath.as_ref().unwrap().to_str().unwrap();
                            println!("Makefile: {makefile}");
                        }

                        println!("Choices:");

                        if files.iter().filter(|x| x.filepath.is_some()).count() > 1
                            && !user_choices.is_empty()
                            && user_choices.contains(&"D")
                            && user_choices.contains(&"M")
                        {
                            println!("D = Build an image from a Dockerfile.");
                            println!("M = Execute `make` on a Makefile.");
                        } else {
                            if files[0].link_target_dir.is_some() {
                                let location1 =
                                    match files[0].link_target_dir.as_ref().unwrap().parent() {
                                        Some(parent) => parent.display().to_string(),
                                        None => files[0]
                                            .link_target_dir
                                            .as_ref()
                                            .unwrap()
                                            .display()
                                            .to_string(),
                                    };
                                println!("1 = Set build working dir to:\n\t{location1}");
                            }

                            let location2 = files[0].parent_dir.display();
                            println!("2 = Set build working dir to:\n\t{location2}");
                        }
                        println!("d = Display info about Dockerfile and/or Makefile.");
                        println!("? = Display this help.");
                    }
                    "1" => {
                        choice_of_where_to_build.follow_link = true;
                        break;
                    }
                    "2" => {
                        choice_of_where_to_build.follow_link = false;
                        break;
                    }
                    _ => {
                        eprintln!("Invalid choice '{choice}'");
                    }
                }
            }
        }
    }

    choice_of_where_to_build
}
