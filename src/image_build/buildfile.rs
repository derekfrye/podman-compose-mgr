use crate::read_interactive_input::{GrammarFragment, GrammarType};
use crate::utils::cmd_utils;
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

pub fn start(
    dir: &DirEntry,
    custom_img_nm: &str,
    build_args: Vec<&str>,
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
        let build_config = read_val_loop(found_buildfiles);
        // dbg!(&build_config);

        if build_config.file.filepath.is_some() {
            build_image_from_spec(build_config)?;
        }
    }
    Ok(())
}

fn read_val_loop(files: Vec<BuildFile>) -> WhatWereBuilding {
    let mut prompt_grammars: Vec<GrammarFragment> = vec![];
    let mut user_choices: Vec<&str> = vec![];

    let buildfile = files[0].clone();
    let are_there_multiple_files = files
        .iter()
        .filter(|x| x.filepath.is_some())
        .collect::<Vec<_>>()
        .len()
        > 1;

    if are_there_multiple_files {
        let grm1 = GrammarFragment {
            original_val_for_prompt: Some("Prefer Dockerfile or Makefile?".to_string()),
            ..Default::default()
        };
        prompt_grammars.push(grm1);

        user_choices = vec!["D", "M", "d", "?"];
        prompt_grammars.extend(make_choice_grammar(
            &user_choices,
            prompt_grammars.len() as u8,
        ));
    } else if buildfile.link_target_dir.is_some() {
        let t = make_build_prompt_grammar(&buildfile);
        user_choices = vec!["1", "2", "d", "?"];
        prompt_grammars.extend(make_choice_grammar(&user_choices, t.len() as u8));
    }

    let mut choice_of_where_to_build: WhatWereBuilding = WhatWereBuilding {
        file: buildfile.clone(),
        follow_link: false,
    };

    if !prompt_grammars.is_empty() {
        loop {
            let z = crate::read_interactive_input::read_val_from_cmd_line_and_proceed_default(
                &mut prompt_grammars,
            );
            if let Some(t) = z.user_entered_val {
                match t.as_str() {
                    // only set back up near line 95, if both Makefile and Dockerfile exist in dir
                    // and here, user picked D for Dockerfile
                    "D" | "M" => {
                        if files.len() > 1 {
                            choice_of_where_to_build.file = files
                                .iter()
                                .find(|x| {
                                    x.filetype
                                        == (match t.as_str() {
                                            "M" => BuildChoice::Makefile,
                                            _ => BuildChoice::Dockerfile,
                                        })
                                })
                                .unwrap()
                                .clone();
                            // but now we need to figure out if they want to set build dir to link's dir, or target of link
                            prompt_grammars = make_build_prompt_grammar(&buildfile);
                            user_choices = vec!["1", "2", "d", "?"];

                            prompt_grammars
                                .extend(make_choice_grammar(&user_choices, t.len() as u8));
                        } else {
                            eprintln!(
                                "No {} found at '{}'",
                                match t.as_str() {
                                    "M" => "Makefile",
                                    _ => "Dockerfile",
                                },
                                buildfile.parent_dir.display()
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
                            println!("Dockerfile: {}", dockerfile);
                        }
                        for f in files.iter().filter(|f| f.filetype == BuildChoice::Makefile) {
                            let makefile = &f.filepath.as_ref().unwrap().to_str().unwrap();
                            println!("Makefile: {}", makefile);
                        }

                        println!("Choices:");

                        if are_there_multiple_files
                            && !user_choices.is_empty()
                            && user_choices.iter().any(|f| *f == "D")
                            && user_choices.iter().any(|f| *f == "M")
                        {
                            println!("D = Build an image from a Dockerfile.");
                            println!("M = Execute `make` on a Makefile.");
                        } else {
                            if buildfile.link_target_dir.is_some() {
                                let location1 =
                                    match buildfile.link_target_dir.as_ref().unwrap().parent() {
                                        Some(parent) => parent.display().to_string(),
                                        None => buildfile
                                            .link_target_dir
                                            .as_ref()
                                            .unwrap()
                                            .display()
                                            .to_string(),
                                    };
                                println!("1 = Set build working dir to:\n\t{}", location1);
                            }

                            let location2 = buildfile.parent_dir.display();
                            println!("2 = Set build working dir to:\n\t{}", location2);
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
                        eprintln!("Invalid choice '{}'", t);
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
            pos: (i + (pos_to_start_from as usize)) as u8,
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
    build_args: Vec<&str>,
) -> Option<Vec<BuildFile>> {
    let parent_dir = dir.path().to_path_buf().parent().unwrap().to_path_buf();
    let dockerfile = parent_dir.join("Dockerfile");
    let makefile = parent_dir.join("Makefile");
    let mut buildfiles: Option<Vec<BuildFile>> = None;

    for file_path in [&dockerfile, &makefile].iter() {
        let buildfile = BuildFile {
            filetype: match file_path {
                _ if *file_path == &makefile => BuildChoice::Makefile,
                _ => BuildChoice::Dockerfile,
            },
            filepath: if let Ok(metadata) = file_path.symlink_metadata() {
                if metadata.file_type().is_symlink() {
                    Some(std::fs::read_link(file_path).unwrap().to_path_buf())
                } else if metadata.is_file() {
                    Some(file_path.to_path_buf())
                } else {
                    None
                }
            } else {
                None
            },
            parent_dir: parent_dir.clone(),
            link_target_dir: if std::fs::read_link(file_path).is_ok() {
                Some(std::fs::read_link(file_path).unwrap().to_path_buf())
            } else {
                None
            },
            base_image: Some(custom_img_nm.to_string()),
            custom_img_nm: Some(custom_img_nm.to_string()),
            build_args: build_args.iter().map(|arg| arg.to_string()).collect(),
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

fn build_image_from_spec(build_config: WhatWereBuilding) -> Result<(), BuildfileError> {
    match build_config.file.filetype {
        BuildChoice::Dockerfile => {
            let _ = crate::helpers::cmd_helper_fns::pull_base_image(
                build_config.file.filepath.as_ref().unwrap(),
            );

            let dockerfile_path = build_config
                .file
                .filepath
                .as_ref()
                .unwrap()
                .to_str()
                .unwrap();

            let mut podman_args = vec![
                "build",
                "-t",
                build_config.file.custom_img_nm.as_ref().unwrap(),
                "-f",
                dockerfile_path,
            ];

            // podman_args.push("--build-context=");
            // let build_context = format!(".:{}", dockerfile_dir.to_str().unwrap());
            // podman_args.push(&build_context);

            for arg in build_config.file.build_args.iter() {
                podman_args.push("--build-arg");
                podman_args.push(arg);
            }

            podman_args.push(build_config.file.parent_dir.to_str().unwrap());

            Ok(cmd_utils::exec_cmd("podman", &podman_args[..]).map_err(BuildfileError::from)?)
        }
        BuildChoice::Makefile => {
            let chg_dir = if build_config.follow_link {
                build_config
                    .file
                    .link_target_dir
                    .as_ref()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .to_str()
                    .unwrap()
            } else {
                build_config.file.parent_dir.to_str().unwrap()
            };

            cmd_utils::exec_cmd("make", &["-C", chg_dir, "clean"])?;
            Ok(cmd_utils::exec_cmd("make", &["-C", chg_dir])?)
        }
    }
}
