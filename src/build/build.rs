use crate::helpers::cmd_helper_fns as cmd;
use std::path::PathBuf;

use crate::read_val::read_val_from_cmd_line_and_proceed;
use crate::read_val::{GrammarFragment, GrammarType};
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

pub fn start(dir: &DirEntry, custom_img_nm: &str, build_args: Vec<&str>) {
    let buildfiles = find_buildfile(dir, custom_img_nm, build_args);
    if buildfiles.is_none()
        || buildfiles.as_ref().unwrap().len() == 0
        || buildfiles
            .as_ref()
            .unwrap()
            .iter()
            .all(|x| x.filepath.is_none())
    {
        eprintln!(
            "No Dockerfile or Makefile found at '{}'",
            dir.path().display()
        );
        // std::process::exit(1);
    } else {
        if let Some(ax) = buildfiles {
            let x = read_val_loop(ax);
            // dbg!(&x);

            if x.file.filepath.is_some() {
                build_image_from_spec(x);
            }
        }
    }
}

fn read_val_loop(files: Vec<BuildFile>) -> WhatWereBuilding {
    let mut prompt_grammars: Vec<GrammarFragment> = vec![];
    let mut user_choices: Vec<&str> = vec![];

    let mut buildfile = files[0].clone();
    let are_there_multiple_files = files
        .iter()
        .filter(|x| x.filepath.is_some())
        .collect::<Vec<_>>()
        .len()
        > 1;

    if are_there_multiple_files {
        let mut grm1 = GrammarFragment::default();
        grm1.original_val_for_prompt = Some("Prefer Dockerfile or Makefile?".to_string());
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

    if prompt_grammars.len() > 0 {
        loop {
            let z = read_val_from_cmd_line_and_proceed(&mut prompt_grammars);
            if let Some(t) = z.user_entered_val {
                match t.as_str() {
                    // only set back up near line 95, if both Makefile and Dockerfile exist in dir
                    // and here, user picked D for Dockerfile
                    "D" | "M" => {
                        if files.len() > 1 {
                            buildfile = files
                                .iter()
                                .find(|x| {
                                    x.filetype
                                        == match t.as_str() {
                                            "M" => BuildChoice::Makefile,
                                            _ => BuildChoice::Dockerfile,
                                        }
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
                    "d" => {
                        for f in files
                            .iter()
                            .filter(|f| f.filetype == BuildChoice::Dockerfile)
                        {
                            let dockerfile = &f.filepath.as_ref().unwrap().to_str().unwrap();
                            println!("Dockerfile: {}", dockerfile);
                        }
                        for f in files.iter().filter(|f| f.filetype == BuildChoice::Makefile) {
                            let dockerfile = &f.filepath.as_ref().unwrap().to_str().unwrap();
                            println!("Dockerfile: {}", dockerfile);
                        }
                    }
                    "?" => {
                        // only show this prompt if we haven't already narrowed down the build choice
                        if are_there_multiple_files
                            && user_choices.is_empty()
                            && user_choices.iter().any(|f| *f == "D")
                        {
                            println!("D = Build an image from a Dockerfile.");
                            println!("M = Execute `make` on a Makefile.");
                        }
                        let location1 = buildfile.link_target_dir.as_ref().unwrap().display();
                        let location2 = buildfile.parent_dir.display();
                        println!("1 = Set build working dir to:\n{}", location1);
                        println!("2 = Set build working dir to:\n {}", location2);

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
    let mut grm1 = GrammarFragment::default();
    grm1.original_val_for_prompt = Some(
        format!(
            "Run `{}` in (1):",
            match buildfile.filetype {
                BuildChoice::Dockerfile => "podman build",
                BuildChoice::Makefile => "make",
            }
        )
        .to_string(),
    );
    prompt_grammars.push(grm1);

    let mut grm2 = GrammarFragment::default();
    grm2.original_val_for_prompt = Some(
        buildfile
            .link_target_dir
            .clone()
            .unwrap()
            .display()
            .to_string(),
    );
    grm2.pos = 1;
    grm2.grammar_type = GrammarType::FileName;
    prompt_grammars.push(grm2);

    let mut grm3 = GrammarFragment::default();
    grm3.original_val_for_prompt = Some(", or (2):".to_string());
    prompt_grammars.push(grm3);

    let mut grm4 = GrammarFragment::default();
    grm4.original_val_for_prompt = Some(buildfile.parent_dir.display().to_string());
    grm4.pos = 3;
    grm4.grammar_type = GrammarType::FileName;
    grm4.suffix = None;
    prompt_grammars.push(grm4);

    let mut grm5 = GrammarFragment::default();
    grm5.original_val_for_prompt = Some("?".to_string());
    grm5.pos = 4;
    grm5.suffix = Some(" ".to_string());
    grm5.prefix = None;
    prompt_grammars.push(grm5);

    prompt_grammars
}

fn make_choice_grammar(user_choices: &Vec<&str>, pos_to_start_from: u8) -> Vec<GrammarFragment> {
    let mut new_prompt_grammars = vec![];
    for i in 0..user_choices.len() {
        let mut choice_separator = Some("/".to_string());
        if i == user_choices.len() - 1 {
            choice_separator = Some(": ".to_string());
        }
        let choice_grammar = GrammarFragment {
            original_val_for_prompt: Some(user_choices[i].to_string()),
            shortened_val_for_prompt: None,
            pos: (i + pos_to_start_from as usize) as u8,
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

fn find_buildfile<'a>(
    dir: &'a DirEntry,
    custom_img_nm: &str,
    build_args: Vec<&str>,
) -> Option<Vec<BuildFile>> {
    let parent_dir = dir.path().to_path_buf().parent().unwrap().to_path_buf();
    let dockerfile = parent_dir.join("Dockerfile");
    let makefile = parent_dir.join("Makefile");
    let mut zz: Option<Vec<BuildFile>> = None;

    for i in [&dockerfile, &makefile].iter() {
        let zzz = BuildFile {
            filetype: match i {
                _ if *i == &makefile => BuildChoice::Makefile,
                _ => BuildChoice::Dockerfile,
            },
            filepath: if let Ok(metadata) = i.symlink_metadata() {
                if metadata.file_type().is_symlink() {
                    Some(std::fs::read_link(&i).unwrap().to_path_buf())
                } else if metadata.is_file() {
                    Some(i.to_path_buf())
                } else {
                    None
                }
            } else {
                None
            },
            parent_dir: parent_dir.clone(),
            link_target_dir: if std::fs::read_link(&i).is_ok() {
                Some(std::fs::read_link(&i).unwrap().to_path_buf())
            } else {
                None
            },
            base_image: Some(custom_img_nm.to_string()),
            custom_img_nm: Some(custom_img_nm.to_string()),
            build_args: build_args.iter().map(|x| x.to_string()).collect(),
        };
        // dbg!(&zzz.filepath);
        // dbg!(i);

        match zz {
            Some(ref mut x) => {
                x.push(zzz);
            }
            None => {
                zz = Some(vec![zzz]);
            }
        }
    }

    zz
}

fn build_image_from_spec(x: WhatWereBuilding) {
    match x.file.filetype {
        BuildChoice::Dockerfile => {
            let _ = cmd::pull_base_image(x.file.filepath.as_ref().unwrap());

            let z = x.file.filepath.as_ref().unwrap().to_str().unwrap();

            let mut xa = vec![];
            xa.push("build");
            xa.push("-t");
            xa.push(x.file.custom_img_nm.as_ref().unwrap());
            xa.push("-f");
            xa.push(&z);

            // x.push("--build-context=");
            // let build_context = format!(".:{}", dockerfile_dir.to_str().unwrap());
            // x.push(&build_context);

            // let mut abc = string::String::new();
            for arg in x.file.build_args.iter() {
                xa.push("--build-arg");
                xa.push(&arg);
            }

            xa.push(x.file.parent_dir.to_str().unwrap());

            cmd::exec_cmd("podman", xa);
        }
        BuildChoice::Makefile => {
            let _ = cmd::exec_cmd(
                "make",
                vec!["-C", x.file.parent_dir.to_str().unwrap(), "clean"],
            );
            let _ = cmd::exec_cmd("make", vec!["-C", x.file.parent_dir.to_str().unwrap()]);
        }
    }
}
