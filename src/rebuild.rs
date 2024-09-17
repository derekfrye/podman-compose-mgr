use crate::args::Args;
use crate::helpers::cmd_helper_fns as cmd;
use crate::read_val::{self, Grammer, GrammerType};

// use regex::Regex;
use serde_yaml::Value;
use std::fs;
use std::fs::File;
use std::vec;
use walkdir::DirEntry;

#[derive(Debug, PartialEq)]
pub struct Image {
    pub name: Option<String>,
    pub container: Option<String>,
    pub skipall_by_this_name: bool,
}

pub struct RebuildManager {
    images_checked: Vec<Image>,
}

impl RebuildManager {
    pub fn new( ) -> Self {
        Self {
            images_checked: Vec::new(),
        }
    }

pub fn rebuild(&mut self,entry: &DirEntry, args: &Args) {
    let yaml = self. read_yaml_file(entry.path().to_str().unwrap());
    if let Some(services) = yaml.get("services") {
        if let Some(services_map) = services.as_mapping() {
            for (_, service_config) in services_map {
                // println!("Service: {:?}", service_name);
                if let Some(image) = service_config.get("image") {
                    // println!("  Image: {:?}", image);
                    if let Some(container_name) = service_config.get("container_name") {
                        let image_string = image.as_str().unwrap().to_string();
                        let container_nm_string = container_name.as_str().unwrap().to_string();
                        // image ck is only empty on first check, so as long as we're non-empty, we might skip this image_string, move to next test
                        if !self.images_checked.is_empty()
                            && (
                                // if this image is in the vec as a skippable image, skip this iter entry (aka continue)
                                self.images_checked.iter().any(|i| {
                                i.name == Some(image_string.clone()) && i.skipall_by_this_name
                            })
                            // or, if this image is not in the list of images we've already checked, continue
                            || self.images_checked.iter().any(|i| {
                                i.name == Some(image_string.clone()) && i.container == Some(container_nm_string.clone())
                            })
                            )
                        {
                            continue;
                        } else {
                            // read_val_loop(
                            //     &entry,
                            //     &image_string,
                            //     args.build_args.clone(),
                            //     &container_nm_string,
                            //     images_checked,
                            // );
    //                         let docker_compose_pth = entry
    //     .path()
    //     .parent()
    //     .unwrap_or(std::path::Path::new("/"))
    //     .display();
    // println!("docker_compose_pth: {}", docker_compose_pth);

self.read_val_loop(entry, &image_string,& args.build_args, &container_nm_string);


                            let c = Image {
                                name:Some( image_string),
                                container: Some(container_nm_string),
                                skipall_by_this_name: false,
                            };
                         self.   images_checked.push(c);
                        }
                    }
                }
            }
        }
    }
}

fn read_val_loop(&mut self, entry: &DirEntry, image: &str, build_args: &Vec<String>, container_name: &str) {
    // let mut images_checked: Vec<Image> = vec![];

    // let sentence = vec!["Refresh", "from"];
let choices = vec!["p", "N", "d", "b", "s", "?"];
let mut grammes: Vec<Grammer> = vec![];

let grm1 = Grammer {
    word: "Refresh".to_string(),
    pos: 0,
    prefix: None,
    suffix: Some(" ".to_string()),
    grammer_type: GrammerType::Verbiage,
    include_in_base_string: true,
    display_at_all:true,
};
grammes.push(grm1);

let grm2 = Grammer {
    word: image.to_string(),
    pos: 1,
    prefix: None,
    suffix: Some(" ".to_string()),
    grammer_type: GrammerType::Image,
    include_in_base_string: false,display_at_all:true,
};
grammes.push(grm2);

let grm3 = Grammer {
    word: "from".to_string(),
    pos: 2,
    prefix: None,
    suffix: Some(" ".to_string()),
    grammer_type: GrammerType::Verbiage,
    include_in_base_string: true,display_at_all:true,
};
grammes.push(grm3);


let docker_compose_pth = entry
        .path()
        .parent()
        .unwrap_or(std::path::Path::new("/"))
        .display();

    let docker_compose_pth_fmtted = format!("{}", docker_compose_pth);


    let grm4 = Grammer {
        word: docker_compose_pth_fmtted,
        pos: 3,
        prefix: None,
        suffix: Some("? ".to_string()),
        grammer_type: GrammerType::DockerComposePath,
        include_in_base_string: false,display_at_all:true,
    };
    grammes.push(grm4);

for i in 0..choices.len()-1 {
    let mut xsuffix = Some("/".to_string());
    if i==choices.len()-1{
        xsuffix = Some(": ".to_string());}
  let abd=  Grammer {
        word: choices[i].to_string(),
        pos: (i+3) as u8,
        prefix: None,
        suffix: xsuffix,
        grammer_type: GrammerType::UserChoice,
        include_in_base_string: true,display_at_all:true,
    };
    grammes.push(abd);
}

let grm5 = Grammer {
    word: container_name.to_string(),
    pos: 3,
    prefix: None,
    suffix: None,
    grammer_type: GrammerType::ContainerName,
    include_in_base_string: false,display_at_all:false,
};
grammes.push(grm5);



    loop {
        let result = read_val::read_val_from_cmd_line_and_proceed(
            &entry,
            // image,
        //    & build_args,
            // container_name,
            // &sentence
            // ,&choices,
            // &mut images_checked,
            &grammes,
        );
        if let Some(user_entered_val) = result.user_entered_val {
            match user_entered_val.as_str() {
                "p" => {
                    self. pull_it(image);
                }
                "N" => {
                    break;
                }
                "d" => {
                self.    build_image_from_dockerfile(&entry, image, build_args.iter().map(|s| s.as_str()).collect());
                }
                "b" => {
                    break;
                }
                "s" => {
                    let mut x = vec![];
                    x.push("stop");
                    x.push(container_name);
                    cmd::exec_cmd("podman", x);
                    let mut x = vec![];
                    x.push("rm");
                    x.push(container_name);
                    cmd::exec_cmd("podman", x);
          self.          build_image_from_dockerfile(&entry, image, build_args.iter().map(|s| s.as_str()).collect());
                }
                _ => {
                    println!("Invalid input. Please enter p/N/d/b/s/?: ");
                }
            }
        }
    }
}


fn build_image_from_dockerfile(&mut self, dir: &DirEntry, image_name: &str, build_args: Vec<&str>) {
    let mut dockerfile = dir.path().to_path_buf().parent().unwrap().to_path_buf();
    dockerfile.push("Dockerfile");

    if !dockerfile.is_file()
        || !fs::metadata(&dockerfile).is_ok()
        || !fs::File::open(&dockerfile).is_ok()
    {
        eprintln!("No Dockerfile found at '{}'", dockerfile.display());
        std::process::exit(1);
    }

    let _ = cmd::pull_base_image(&dockerfile);

    let z = dockerfile.display().to_string();

    let mut x = vec![];
    x.push("build");
    x.push("-t");
    x.push(image_name);
    x.push("-f");
    x.push(&z);

    // let mut abc = string::String::new();
    for arg in build_args {
        x.push("--build-arg");
        x.push(&arg);
    }

    cmd::exec_cmd("podman", x);
}

fn read_yaml_file(&mut self, file: &str) -> Value {
    let file = File::open(file).expect("file not found");
    let yaml: Value = serde_yaml::from_reader(file).expect("Error reading file");
    yaml
}

fn pull_it(&mut self,image: &str) {
    let mut x = vec![];

    x.push("pull");
    x.push(image);
    cmd::exec_cmd("podman", x);
}
}