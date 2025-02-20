use std::path::PathBuf;
use crate::helpers::cmd_helper_fns as cmd;

use crate::read_val::read_val_from_cmd_line_and_proceed;
use crate::{helpers::cmd_helper_fns::{self, file_exists_and_readable}, read_val::{GrammarFragment, GrammarType, ReadValResult}};
use walkdir::DirEntry;

 struct DockerfileAndMakefile<'a> {
     dockerfile: PathBuf,
     dockerfile_is_link: bool,
     dockerfile_exists: bool,
     dockerfile_dir: PathBuf,
     dockerfile_link_target_dir: PathBuf,

     make_cmds: Vec<&'a str>,
     makefile: PathBuf,
     makefile_is_link: bool,
     makefile_exists: bool,
        makefile_dir: PathBuf,
        makefile_link_target_dir: PathBuf,
}

struct WhatWereBuilding<'a> {
    file: &'a PathBuf,
    follow_link: bool,

}

pub fn start( dir: &DirEntry, image_name: &str, build_args: Vec<&str>)
{
    let dockerfile_and_makefile = find_dockerfile_and_makefile(dir, image_name, build_args);
    if !dockerfile_and_makefile.dockerfile_exists && !dockerfile_and_makefile.makefile_exists {
        eprintln!("No Dockerfile or Makefile found at '{}'", dir.path().display());
        std::process::exit(1);
    }

    read_val_loop(dockerfile_and_makefile);
}


 fn read_val_loop(files: DockerfileAndMakefile){
    let mut prompt_grammars: Vec<GrammarFragment> = vec![];
    let mut user_choices: Vec<&str> = vec![];
    let template_grammar = GrammarFragment {
        original_val_for_prompt: Some("Template".to_string()),
        shortened_val_for_prompt: None,
        pos: 0,
        prefix: None,
        suffix: Some(" ".to_string()),
        grammar_type: GrammarType::Verbiage,
        part_of_static_prompt: true,
        display_at_all: true,
    };

if files.makefile_exists && files.dockerfile_exists {
    let mut grm1 = template_grammar.clone();
      grm1.original_val_for_prompt= Some("Prefer Dockerfile or Makefile?".to_string());
        prompt_grammars.push(grm1);

    user_choices= vec!["D", "M", "d", "?"];        
}
else if files.makefile_exists && files.makefile_is_link {
    let mut grm1 = template_grammar.clone();
    grm1.original_val_for_prompt= Some("Run `make` in".to_string());
      prompt_grammars.push(grm1);

      let mut grm2 = template_grammar.clone();
    grm2.original_val_for_prompt= Some(files.makefile_dir.display().to_string());
    grm2.pos =1;
    grm2.grammar_type= GrammarType::FileName;
    prompt_grammars.push(grm2);

    let mut grm3 = template_grammar.clone();
    grm3.original_val_for_prompt= Some("or".to_string());
      prompt_grammars.push(grm3);

    let mut grm4 = template_grammar.clone();
    grm4.original_val_for_prompt= Some(files.makefile_link_target_dir.display().to_string());
    grm4.pos =3;
    grm4.grammar_type= GrammarType::FileName;
    prompt_grammars.push(grm4);

  user_choices= vec!["1", "2", "d", "?"];        
}
else if files.dockerfile_exists && files.dockerfile_is_link {
    let mut grm1 = template_grammar.clone();
    grm1.original_val_for_prompt= Some("Run `podman build` in".to_string());
      prompt_grammars.push(grm1);

      let mut grm2 = template_grammar.clone();
    grm2.original_val_for_prompt= Some(files.makefile_dir.display().to_string());
    grm2.pos =1;
    grm2.grammar_type= GrammarType::FileName;
    prompt_grammars.push(grm2);

    let mut grm3 = template_grammar.clone();
    grm3.original_val_for_prompt= Some("or".to_string());
      prompt_grammars.push(grm3);

    let mut grm4 = template_grammar.clone();
    grm4.original_val_for_prompt= Some(files.makefile_link_target_dir.display().to_string());
    grm4.pos =3;
    grm4.grammar_type= GrammarType::FileName;
    prompt_grammars.push(grm4);

  user_choices= vec!["1", "2", "d", "?"];    
}

        for i in 0..user_choices.len() {
            let mut choice_separator = Some("/".to_string());
            if i == user_choices.len() - 1 {
                choice_separator = Some(": ".to_string());
            }
            let choice_grammar = GrammarFragment {
                original_val_for_prompt: Some(user_choices[i].to_string()),
                shortened_val_for_prompt: None,
                pos: (i + prompt_grammars.len()) as u8,
                prefix: None,
                suffix: choice_separator,
                grammar_type: GrammarType::UserChoice,
                part_of_static_prompt: true,
                display_at_all: true,
            };
            prompt_grammars.push(choice_grammar);
        }

        loop{
            let mut choice_of_where_to_build:WhatWereBuilding ;
        let z = read_val_from_cmd_line_and_proceed(&mut prompt_grammars, GrammarType::Verbiage, GrammarType::UserChoice);
        if let Some(t) = z.user_entered_val {
            match t.as_str() {
                "D" => {
                    choice_of_where_to_build = WhatWereBuilding {
                        file: &files.dockerfile,
                        follow_link: files.dockerfile_is_link,
                    };
                }
                "M" => {
                    build_image_from_spec(files.dir, image_name, build_args, true);
                }
                "d" => {
                    let dockerfile = files.dockerfile.to_str().unwrap();
                    let makefile = files.makefile.to_str().unwrap();
                    println!("Dockerfile: {}", dockerfile);
                    println!("Makefile: {}", makefile);
                }
                "?" => {
                    println!("1 = Build the image in the first directory choice.");
                    println!("2 = Build the image in the second directory choice.");
                    println!("D = Build an image from a Dockerfile.");
                    println!("2 = Execute `make` on a Makefile.");
                    println!(
                                "d = Display info about Dockerfile and Makefile, if they exist."
                            );
                    println!(
                                "b = Build image from the Dockerfile residing in same path as the docker-compose.yml."
                            );
                    println!("? = Display this help.");   
                }
                _ => {
                    eprintln!("Invalid choice '{}'", t);
                }
            }
        }
    }

}
 
 fn make_choice_grammar(user_choices: Vec<&str>,pos_to_start_from:u8) ->Vec<GrammarFragment>{
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
            part_of_static_prompt: true,
            display_at_all: true,
        };
        new_prompt_grammars.push(choice_grammar);
    }
    new_prompt_grammars
 }

 fn find_dockerfile_and_makefile<'a>( dir: &'a DirEntry, image_name: &'a str, build_args: Vec<&'a str>) -> DockerfileAndMakefile<'a> {
    let parent_dir = dir.path().to_path_buf().parent().unwrap().to_path_buf();
    let mut ret_res = DockerfileAndMakefile {
        dockerfile: parent_dir.join("Dockerfile"),
        dockerfile_is_link: false,
        make_cmds: vec![],
        makefile: parent_dir.join("Makefile"),
        makefile_is_link: false,
        dockerfile_exists: false,
        makefile_exists: false,
        dockerfile_dir: parent_dir.clone(),
        dockerfile_link_target_dir: parent_dir.clone(),
        makefile_dir: parent_dir.clone(),
        makefile_link_target_dir: parent_dir.clone(),
    };

    for i in [ret_res.dockerfile.clone(), ret_res.makefile.clone()].iter() {
        let link_or_file = i.symlink_metadata();
        if let Ok(metadata) = link_or_file {

            if metadata.file_type().is_symlink() {
                if i == &ret_res.dockerfile {
                    ret_res.dockerfile = std::fs::read_link(&i).unwrap().to_path_buf();
                    ret_res.dockerfile_dir = ret_res.dockerfile.parent().unwrap().to_path_buf();
                    
                    ret_res.dockerfile_is_link = true;
                } else {
                    ret_res.makefile = std::fs::read_link(&i).unwrap().to_path_buf();
                    ret_res.makefile_dir = ret_res.makefile.parent().unwrap().to_path_buf();
                    ret_res.makefile_is_link = true;
                    
                }
            }
            else if metadata.file_type().is_file() {
                if i == &ret_res.dockerfile {
                    ret_res.dockerfile_exists = true;
                    ret_res.dockerfile_is_link=false;
                    ret_res.dockerfile_dir = parent_dir.clone();
                } else {
                    ret_res.makefile_exists = true;
                    ret_res.makefile_is_link=false;
                    ret_res.makefile_dir = parent_dir.clone();
                }
            }
        }
        else  {
            if i == &ret_res.dockerfile {
                
                ret_res.dockerfile_exists=false;
            } else {
                ret_res.makefile_exists=false;
                
            }
        }
    }

    ret_res
}

fn build_image_from_spec(dir: &DirEntry, image_name: &str, build_args: Vec<&str>, follow_symlinks: bool) {
    let parent_dir = dir.path().to_path_buf().parent().unwrap().to_path_buf();

    let dockerfile = parent_dir.join("Dockerfile");
    let makefile = parent_dir.join("Makefile");
    let symlink_test = makefile.symlink_metadata();

    if symlink_test.is_ok() {
        // let makefile = makefile.unwrap();
   let target =     if follow_symlinks&&  symlink_test.unwrap().file_type().is_symlink() {
             std::fs::read_link(&makefile).unwrap().to_path_buf().parent().unwrap().to_path_buf()
            }
             else 
             {
                    parent_dir.clone()
             };
            
             if file_exists_and_readable(&makefile) {
                let _ = cmd::exec_cmd("make", vec!["-C", target.to_str().unwrap(), "clean"]);
                let _ = cmd::exec_cmd("make", vec!["-C", target.to_str().unwrap()]);
            } else {
                if !file_exists_and_readable(&dockerfile) {
                    eprintln!("No Dockerfile found at '{}'", parent_dir.display());
                    std::process::exit(1);
                }

        }
    }


        let _ = cmd::pull_base_image(&dockerfile);

        let z = dockerfile.to_str().unwrap();

        let mut x = vec![];
        x.push("build");
        x.push("-t");
        x.push(image_name);
        x.push("-f");
        x.push(&z);

        // x.push("--build-context=");
        // let build_context = format!(".:{}", dockerfile_dir.to_str().unwrap());
        // x.push(&build_context);

        // let mut abc = string::String::new();
        for arg in build_args {
            x.push("--build-arg");
            x.push(&arg);
        }

        x.push(parent_dir.to_str().unwrap());

        cmd::exec_cmd("podman", x);
    }
