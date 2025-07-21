use crate::image_build::buildfile_build;
use crate::image_build::buildfile_types::{BuildChoice as BuildChoiceExternal, BuildFile as BuildFileExternal, WhatWereBuilding as WhatWereBuildingExternal};
use crate::read_interactive_input::{GrammarFragment, GrammarType};
use std::path::PathBuf;
use thiserror::Error;
use walkdir::DirEntry;

#[derive(Debug, PartialEq, Clone)]
struct BuildFile {
    filetype: BuildChoice,
    filepath: Option<PathBuf>,
    parent_dir: PathBuf,
    link_target_dir: Option<PathBuf>,
    base_image: Option<String>,
    custom_img_nm: Option<String>,
    build_args: Vec<String>,
}

#[derive(Debug, PartialEq, Clone)]
enum BuildChoice {
    Dockerfile,
    Makefile,
}

#[derive(Debug, PartialEq, Clone)]
struct WhatWereBuilding {
    file: BuildFile,
    follow_link: bool,
}

#[derive(Debug, Error)]
pub enum BuildfileError {
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),

    #[error("Path contains invalid UTF-8: {0}")]
    InvalidPath(String),

    #[error("Rebuild error: {0}")]
    RebuildError(String),

    #[error("Command execution error: {0}")]
    CommandExecution(#[from] Box<dyn std::error::Error>),
}

/// Start the build process for a directory
/// 
/// # Arguments
/// 
/// * `dir` - Directory entry to search for build files
/// * `custom_img_nm` - Custom image name to use
/// * `build_args` - Build arguments to pass to the build process
/// 
/// # Returns
/// 
/// * `Result<(), BuildfileError>` - Success or error
/// 
/// # Errors
/// 
/// Returns an error if no build files are found or if the build process fails.
/// 
/// # Panics
/// 
/// Panics if build files cannot be processed or if internal state is invalid.
pub fn start(
    dir: &DirEntry,
    custom_img_nm: &str,
    build_args: &[&str],
) -> Result<(), BuildfileError> {
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
        let build_config = read_val_loop(&found_buildfiles);
        // dbg!(&build_config);

        if build_config.file.filepath.is_some() {
            // Convert internal types to external types
            let external_build_config = WhatWereBuildingExternal {
                file: BuildFileExternal {
                    filetype: match build_config.file.filetype {
                        BuildChoice::Dockerfile => BuildChoiceExternal::Dockerfile,
                        BuildChoice::Makefile => BuildChoiceExternal::Makefile,
                    },
                    filepath: build_config.file.filepath.clone(),
                    parent_dir: build_config.file.parent_dir.clone(),
                    link_target_dir: build_config.file.link_target_dir.clone(),
                    base_image: build_config.file.base_image.clone(),
                    custom_img_nm: build_config.file.custom_img_nm.clone(),
                    build_args: build_config.file.build_args.clone(),
                },
                follow_link: build_config.follow_link,
            };
            
            buildfile_build::build_image_from_spec(&external_build_config)
                .map_err(|e| BuildfileError::CommandExecution(Box::new(e)))?;
        }
    }
    Ok(())
}

/// Build the interactive prompt grammars for buildfile selection
fn buildfile_prompt_grammars(files: &[BuildFile]) -> (Vec<GrammarFragment>, Vec<&'static str>) {
    let mut prompt_grammars: Vec<GrammarFragment> = vec![];
    let mut user_choices: Vec<&str> = vec![];

    let buildfile = files[0].clone();
    let multiple = files.iter().filter(|x| x.filepath.is_some()).count() > 1;

    if multiple {
        prompt_grammars.push(GrammarFragment {
            original_val_for_prompt: Some("Prefer Dockerfile or Makefile?".to_string()),
            ..Default::default()
        });
        user_choices = vec!["D", "M", "d", "?"];
        prompt_grammars.extend(make_choice_grammar(&user_choices, 1));
    } else if buildfile.link_target_dir.is_some() {
        prompt_grammars = make_build_prompt_grammar(&buildfile);
        user_choices = vec!["1", "2", "d", "?"];
        prompt_grammars.extend(make_choice_grammar(
            &user_choices,
            u8::try_from(prompt_grammars.len()).unwrap_or(255),
        ));
    }

    (prompt_grammars, user_choices)
}

fn read_val_loop(files: &[BuildFile]) -> WhatWereBuilding {
    // use helper for grammars
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
                    // only set back up near line 95, if both Makefile and Dockerfile exist in dir
                    // and here, user picked D for Dockerfile
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
                            // but now we need to figure out if they want to set build dir to link's dir, or target of link
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
                    // print details / explanation
                    "d" | "?" => {
                        // only show this prompt if we haven't already narrowed down the build choice
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
                    // building at link target
                    "1" => {
                        choice_of_where_to_build.follow_link = true;
                        break;
                    }
                    // building in dir symlink lives, not link target
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

fn make_build_prompt_grammar(buildfile: &BuildFile) -> Vec<GrammarFragment> {
    let mut prompt_grammars: Vec<GrammarFragment> = vec![];
    // let mut user_choices: Vec<&str> ;
    let grm1 = GrammarFragment {
        original_val_for_prompt: Some(
            format!(
                "Run `{}` in (1):",
                match buildfile.filetype {
                    BuildChoice::Dockerfile => "podman build",
                    BuildChoice::Makefile => "make",
                }
            )
            .to_string(),
        ),
        ..Default::default()
    };
    prompt_grammars.push(grm1);

    let grm2 = GrammarFragment {
        original_val_for_prompt: Some(
            buildfile
                .link_target_dir
                .clone()
                .unwrap()
                .display()
                .to_string(),
        ),
        pos: 1,
        grammar_type: GrammarType::FileName,
        suffix: None,
        ..Default::default()
    };

    prompt_grammars.push(grm2);

    let grm3 = GrammarFragment {
        original_val_for_prompt: Some(", or (2):".to_string()),
        pos: 2,
        ..Default::default()
    };
    prompt_grammars.push(grm3);

    let grm4 = GrammarFragment {
        original_val_for_prompt: Some(buildfile.parent_dir.display().to_string()),
        pos: 3,
        grammar_type: GrammarType::FileName,
        suffix: None,
        ..Default::default()
    };

    prompt_grammars.push(grm4);

    let grm5 = GrammarFragment {
        original_val_for_prompt: Some("?".to_string()),
        pos: 4,
        suffix: Some(" ".to_string()),
        prefix: None,
        ..Default::default()
    };
    prompt_grammars.push(grm5);

    prompt_grammars
}

fn make_choice_grammar(user_choices: &[&str], pos_to_start_from: u8) -> Vec<GrammarFragment> {
    let mut new_prompt_grammars = vec![];
    for i in 0..user_choices.len() {
        let mut choice_separator = Some("/".to_string());
        if i == user_choices.len() - 1 {
            choice_separator = Some(": ".to_string());
        }
        let choice_grammar = GrammarFragment {
            original_val_for_prompt: Some(user_choices[i].to_string()),
            shortened_val_for_prompt: None,
            pos: u8::try_from(i + (pos_to_start_from as usize)).unwrap_or(255),
            prefix: None,
            suffix: choice_separator,
            grammar_type: GrammarType::UserChoice,
            can_shorten: false,
            display_at_all: true,
        };
        new_prompt_grammars.push(choice_grammar);
    }
    new_prompt_grammars
}

fn find_buildfile(
    dir: &DirEntry,
    custom_img_nm: &str,
    build_args: &[&str],
) -> Option<Vec<BuildFile>> {
    let parent_dir = dir.path().to_path_buf().parent().unwrap().to_path_buf();
    let dockerfile = parent_dir.join("Dockerfile");
    let makefile = parent_dir.join("Makefile");
    let mut buildfiles: Option<Vec<BuildFile>> = None;

    for file_path in &[&dockerfile, &makefile] {
        let buildfile = BuildFile {
            filetype: match file_path {
                _ if *file_path == &makefile => BuildChoice::Makefile,
                _ => BuildChoice::Dockerfile,
            },
            filepath: if let Ok(metadata) = file_path.symlink_metadata() {
                if metadata.file_type().is_symlink() {
                    Some(std::fs::read_link(file_path).unwrap().clone())
                } else if metadata.is_file() {
                    Some((*file_path).clone())
                } else {
                    None
                }
            } else {
                None
            },
            parent_dir: parent_dir.clone(),
            link_target_dir: if std::fs::read_link(file_path).is_ok() {
                Some(std::fs::read_link(file_path).unwrap().clone())
            } else {
                None
            },
            base_image: Some(custom_img_nm.to_string()),
            custom_img_nm: Some(custom_img_nm.to_string()),
            build_args: build_args.iter().map(|arg| (*arg).to_string()).collect(),
        };
        // dbg!(&buildfile.filepath);
        // dbg!(file_path);

        match buildfiles {
            Some(ref mut files) => {
                files.push(buildfile);
            }
            None => {
                buildfiles = Some(vec![buildfile]);
            }
        }
    }

    buildfiles
}

