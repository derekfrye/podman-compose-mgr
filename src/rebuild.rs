use crate::args::Args;
use crate::helpers::cmd_helper_fns as cmd;
use crate::helpers::podman_helper_fns;
use crate::read_val::{ self, Grammar, GrammerType };

// use regex::Regex;
use chrono::{ DateTime, Local };
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
    pub fn new() -> Self {
        Self {
            images_checked: Vec::new(),
        }
    }

    pub fn rebuild(&mut self, entry: &DirEntry, args: &Args) {
        let yaml = self.read_yaml_file(entry.path().to_str().unwrap());
        if let Some(services) = yaml.get("services") {
            if let Some(services_map) = services.as_mapping() {
                for (_, service_config) in services_map {
                    // println!("Service: {:?}", service_name);
                    if let Some(image) = service_config.get("image") {
                        // println!("  Image: {:?}", image);
                        if let Some(container_name) = service_config.get("container_name") {
                            let image_string = image.as_str().unwrap().to_string();
                            let container_nm_string = container_name.as_str().unwrap().to_string();

                            // if this image is in the vec as a skippable image, skip this iter entry (aka continue)
                            let img_is_set_to_skip = self.images_checked.iter().any(|i| {
                                if let Some(ref name) = i.name {
                                    name == &image_string && i.skipall_by_this_name
                                } else {
                                    false
                                }
                            });

                            // or, if this image is not in the list of images we've already checked, continue
                            let img_and_container_previously_reviewed = self.images_checked
                                .iter()
                                .any(|i| {
                                    if let Some(ref name) = i.name {
                                        if let Some(ref contner) = i.container {
                                            name == &image_string && contner == &container_nm_string
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    }
                                });

                            // image ck is only empty on first check, so as long as we're non-empty, we might skip this image_string, move to next test
                            if
                                !self.images_checked.is_empty() &&
                                (img_is_set_to_skip || img_and_container_previously_reviewed)
                            {
                                continue;
                            } else {
                                self.read_val_loop(
                                    entry,
                                    &image_string,
                                    &args.build_args,
                                    &container_nm_string
                                );

                                let c = Image {
                                    name: Some(image_string),
                                    container: Some(container_nm_string),
                                    skipall_by_this_name: false,
                                };
                                self.images_checked.push(c);
                            }
                        }
                    }
                }
            }
        }
    }

    fn read_val_loop(
        &mut self,
        entry: &DirEntry,
        image: &str,
        build_args: &Vec<String>,
        container_name: &str
    ) {
        // let mut images_checked: Vec<Image> = vec![];

        // let sentence = vec!["Refresh", "from"];

        let mut grammars: Vec<Grammar> = vec![];

        let grm1 = Grammar {
            original_val_for_prompt: Some("Refresh".to_string()),
            shortend_val_for_prompt: None,
            pos: 0,
            prefix: None,
            suffix: Some(" ".to_string()),
            grammer_type: GrammerType::Verbiage,
            part_of_static_prompt: true,
            display_at_all: true,
        };

        grammars.push(grm1);

        let grm2 = Grammar {
            original_val_for_prompt: Some(image.to_string()),
            shortend_val_for_prompt: None,
            pos: 1,
            prefix: None,
            suffix: Some(" ".to_string()),
            grammer_type: GrammerType::Image,
            part_of_static_prompt: false,
            display_at_all: true,
        };
        grammars.push(grm2);

        let grm3 = Grammar {
            original_val_for_prompt: Some("from".to_string()),
            shortend_val_for_prompt: None,
            pos: 2,
            prefix: None,
            suffix: Some(" ".to_string()),
            grammer_type: GrammerType::Verbiage,
            part_of_static_prompt: true,
            display_at_all: true,
        };
        grammars.push(grm3);

        let docker_compose_pth = entry
            .path()
            .parent()
            .unwrap_or(std::path::Path::new("/"))
            .display();
        let docker_compose_pth_fmtted = format!("{}", docker_compose_pth);
        let grm4 = Grammar {
            original_val_for_prompt: Some(docker_compose_pth_fmtted.clone()),
            shortend_val_for_prompt: None,
            pos: 3,
            prefix: None,
            suffix: Some("? ".to_string()),
            grammer_type: GrammerType::DockerComposePath,
            part_of_static_prompt: false,
            display_at_all: true,
        };
        grammars.push(grm4);

        let grm5 = Grammar {
            original_val_for_prompt: Some(container_name.to_string()),
            shortend_val_for_prompt: None,
            pos: 4,
            prefix: None,
            suffix: None,
            grammer_type: GrammerType::ContainerName,
            part_of_static_prompt: false,
            display_at_all: false,
        };
        grammars.push(grm5);

        let choices = vec!["p", "N", "d", "b", "s", "?"];
        for i in 0..choices.len() {
            let mut choice_separator = Some("/".to_string());
            if i == choices.len() - 1 {
                choice_separator = Some(": ".to_string());
            }
            let choice_grammar = Grammar {
                original_val_for_prompt: Some(choices[i].to_string()),
                shortend_val_for_prompt: None,
                pos: (i + 5) as u8,
                prefix: None,
                suffix: choice_separator,
                grammer_type: GrammerType::UserChoice,
                part_of_static_prompt: true,
                display_at_all: true,
            };
            grammars.push(choice_grammar);
        }

        loop {
            let result = read_val::read_val_from_cmd_line_and_proceed(
                &mut grammars,
                GrammerType::DockerComposePath,
                GrammerType::Image
            );

            match result.user_entered_val {
                None => {
                    break;
                }
                Some(user_entered_val) =>
                    match user_entered_val.as_str() {
                        "p" => {
                            self.pull_it(image);
                            break;
                        }
                        "N" => {
                            break;
                        }
                        "d" | "?" =>
                            match user_entered_val.as_str() {
                                "d" => {
                                    println!("Image: {}", image.to_string());
                                    println!("Container name: {}", container_name);
                                    println!("Compose file: {}", docker_compose_pth_fmtted);
                                    println!(
                                        "Created: {}",
                                        self.format_time_ago(
                                            podman_helper_fns
                                                ::get_podman_image_upstream_create_time(&image)
                                                .unwrap()
                                        )
                                    );
                                    println!(
                                        "Pulled: {}",
                                        self.format_time_ago(
                                            podman_helper_fns
                                                ::get_podman_ondisk_modify_time(&image)
                                                .unwrap()
                                        )
                                    );
                                    println!(
                                        "Dockerfile exists: {}",
                                        cmd::dockerfile_exists_and_readable(
                                            &entry
                                                .path()
                                                .parent()
                                                .unwrap()
                                                .join("Dockerfile")
                                                .to_path_buf()
                                        )
                                    );
                                }
                                "?" => {
                                    println!("p = Pull image from upstream.");
                                    println!("N = Do nothing, skip this image.");
                                    println!(
                                        "d = Display info (image name, docker-compose.yml path, upstream img create date, and img on-disk modify date)."
                                    );
                                    println!(
                                        "b = Build image from the Dockerfile residing in same path as the docker-compose.yml."
                                    );
                                    println!(
                                        "s = Skip all subsequent images with this same name (regardless of container name)."
                                    );
                                    println!("? = Display this help.");
                                }
                                _ => {}
                            }
                        "b" => {
                            self.build_image_from_dockerfile(
                                &entry,
                                image,
                                build_args
                                    .iter()
                                    .map(|s| s.as_str())
                                    .collect()
                            );
                            break;
                        }
                        "s" => {
                            let c = Image {
                                name: Some(image.to_string()),
                                container: Some(container_name.to_string()),
                                skipall_by_this_name: true,
                            };
                            self.images_checked.push(c);
                            break;
                        }
                        _ => {
                            println!("Invalid input. Please enter p/N/d/b/s/?: ");
                        }
                    }
            }
        }
    }

    fn format_time_ago(&mut self, dt: DateTime<Local>) -> String {
        let now = Local::now();
        let duration = now.signed_duration_since(dt);
        let days = duration.num_days();
        let hours = duration.num_hours();
        let minutes = duration.num_minutes();
        let seconds = duration.num_seconds();
        if days > 0 {
            format!("{} days ago", days)
        } else if hours > 0 {
            format!("{} hours ago", hours)
        } else if minutes > 0 {
            format!("{} minutes ago", minutes)
        } else {
            format!("{} seconds ago", seconds)
        }
    }

    fn build_image_from_dockerfile(
        &mut self,
        dir: &DirEntry,
        image_name: &str,
        build_args: Vec<&str>
    ) {
        let mut dockerfile = dir.path().to_path_buf().parent().unwrap().to_path_buf();
        dockerfile.push("Dockerfile");

        if
            !dockerfile.is_file() ||
            !fs::metadata(&dockerfile).is_ok() ||
            !fs::File::open(&dockerfile).is_ok()
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

    // other methods...

    fn read_yaml_file(&mut self, file: &str) -> Value {
        let file = File::open(file).expect("file not found");
        let yaml: Value = serde_yaml::from_reader(file).expect("Error reading file");
        yaml
    }

    fn pull_it(&mut self, image: &str) {
        let mut x = vec![];

        x.push("pull");
        x.push(image);
        cmd::exec_cmd("podman", x);
    }
}
