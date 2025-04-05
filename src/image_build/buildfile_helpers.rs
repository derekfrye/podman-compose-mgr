use crate::read_interactive_input::{GrammarFragment, GrammarType};
use crate::image_build::buildfile_types::{BuildFile, BuildChoice};
use walkdir::DirEntry;

/// Setup prompts for buildfile selection
pub fn setup_prompts(files: &[BuildFile]) -> (Vec<GrammarFragment>, Vec<&str>, bool) {
    let mut prompt_grammars: Vec<GrammarFragment> = vec![];
    let mut user_choices: Vec<&str> = vec![];
    
    let buildfile = &files[0];
    let are_there_multiple_files = files
        .iter()
        .filter(|x| x.filepath.is_some())
        .count() > 1;
    
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
        let t = make_build_prompt_grammar(buildfile);
        user_choices = vec!["1", "2", "d", "?"];
        prompt_grammars.extend(make_choice_grammar(&user_choices, t.len() as u8));
    }
    
    (prompt_grammars, user_choices, are_there_multiple_files)
}

/// Display information about available buildfiles
pub fn handle_display_info(files: &[BuildFile], buildfile: &BuildFile, user_choices: &[&str], are_there_multiple_files: bool) {
    // Show Dockerfile and Makefile paths
    for f in files.iter().filter(|f| f.filetype == BuildChoice::Dockerfile) {
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
            let location1 = match buildfile.link_target_dir.as_ref().unwrap().parent() {
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

/// Handle file type choice
pub fn handle_file_type_choice<'a>(files: &[BuildFile], choice: &str, buildfile: &BuildFile) -> Option<(BuildFile, Vec<GrammarFragment>, Vec<&'a str>)> {
    if files.len() > 1 {
        // Find the file matching the chosen type
        let chosen_file = files
            .iter()
            .find(|x| {
                x.filetype
                    == (match choice {
                        "M" => BuildChoice::Makefile,
                        _ => BuildChoice::Dockerfile,
                    })
            })
            .unwrap()
            .clone();
            
        // Setup prompts for working directory choice
        let prompt_grammars = make_build_prompt_grammar(buildfile);
        let user_choices = vec!["1", "2", "d", "?"];
        
        Some((chosen_file, prompt_grammars, user_choices))
    } else {
        eprintln!(
            "No {} found at '{}'",
            match choice {
                "M" => "Makefile",
                _ => "Dockerfile",
            },
            buildfile.parent_dir.display()
        );
        None
    }
}

/// Create grammar fragments for build prompts
pub fn make_build_prompt_grammar(buildfile: &BuildFile) -> Vec<GrammarFragment> {
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

/// Create grammar fragments for choice options
pub fn make_choice_grammar(user_choices: &[&str], pos_to_start_from: u8) -> Vec<GrammarFragment> {
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

/// Find buildfiles in a directory
pub fn find_buildfile(
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

        match &mut buildfiles {
            Some(files) => {
                files.push(buildfile);
            }
            None => {
                buildfiles = Some(vec![buildfile]);
            }
        }
    }

    buildfiles
}