use crate::args::Args;
use crate::image_build::buildfile::start;
use crate::image_build::container_file::{parse_container_file, ContainerFileError};
use crate::interfaces::{CommandHelper, ReadInteractiveInputHelper};
use crate::read_interactive_input::{GrammarFragment, GrammarType};
use crate::utils::podman_utils;

use chrono::{DateTime, Local};
use serde_yaml::Value;
use std::fs::File;
use std::path::Path;
use std::vec;
use thiserror::Error;
use walkdir::DirEntry;

#[derive(Debug, Error)]
pub enum RebuildError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parsing error: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    #[error("Container file parsing error: {0}")]
    ContainerFileParse(#[from] ContainerFileError),

    #[error("Path not found: {0}")]
    PathNotFound(String),

    #[error("Missing field in YAML: {0}")]
    MissingField(String),

    #[error("Invalid container configuration: {0}")]
    InvalidConfig(String),

    #[error("Command execution error: {0}")]
    CommandExecution(String),

    #[error("Date parsing error: {0}")]
    DateParse(String),

    #[error("Error: {0}")]
    Other(String),
}

#[derive(Debug, PartialEq)]
pub struct Image {
    pub name: Option<String>,
    pub container: Option<String>,
    pub skipall_by_this_name: bool,
}

pub struct RebuildManager<'a, C: CommandHelper, R: ReadInteractiveInputHelper> {
    images_already_processed: Vec<Image>,
    cmd_helper: &'a C,
    read_val_helper: &'a R,
}

impl<'a, C: CommandHelper, R: ReadInteractiveInputHelper> RebuildManager<'a, C, R> {
    pub fn new(cmd_helper: &'a C, read_val_helper: &'a R) -> Self {
        Self {
            images_already_processed: Vec::new(),
            cmd_helper,
            read_val_helper,
        }
    }

    /// Process a docker-compose.yml or .container file for rebuilding images
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Unable to get the file path as a string
    /// - Unable to read or parse the file
    /// - Service configurations are invalid
    pub fn rebuild(&mut self, entry: &DirEntry, args: &Args) -> Result<(), RebuildError> {
        // Get file path safely
        let file_path = entry.path().to_str().ok_or_else(|| {
            RebuildError::PathNotFound(format!("Invalid UTF-8 in path: {}", entry.path().display()))
        })?;

        // Determine file type and process accordingly
        if file_path.ends_with(".container") {
            self.process_container_file(entry, args)
        } else if file_path.ends_with("docker-compose.yml") {
            self.process_compose_file(entry, args)
        } else {
            Err(RebuildError::InvalidConfig(format!(
                "Unsupported file type: {file_path}"
            )))
        }
    }

    /// Process a docker-compose.yml file for rebuilding images
    fn process_compose_file(&mut self, entry: &DirEntry, args: &Args) -> Result<(), RebuildError> {
        let file_path = entry.path().to_str().ok_or_else(|| {
            RebuildError::PathNotFound(format!("Invalid UTF-8 in path: {}", entry.path().display()))
        })?;

        let yaml = Self::read_yaml_file(file_path)?;

        // Get services from YAML
        let services = yaml.get("services").ok_or_else(|| {
            RebuildError::MissingField("No 'services' section found in compose file".to_string())
        })?;

        let services_map = services.as_mapping().ok_or_else(|| {
            RebuildError::InvalidConfig("'services' is not a mapping".to_string())
        })?;

        // Process each service
        for (_, service_config) in services_map {
            // Get image name if present
            if let Some(image) = service_config.get("image") {
                // Get container name if present
                if let Some(container_name) = service_config.get("container_name") {
                    // Extract string values safely
                    let image_string = image
                        .as_str()
                        .ok_or_else(|| {
                            RebuildError::InvalidConfig("'image' is not a string".to_string())
                        })?
                        .to_string();

                    let container_nm_string = container_name
                        .as_str()
                        .ok_or_else(|| {
                            RebuildError::InvalidConfig(
                                "'container_name' is not a string".to_string(),
                            )
                        })?
                        .to_string();

                    // Check if this image should be skipped
                    let img_is_set_to_skip = self.images_already_processed.iter().any(|i| {
                        if let Some(ref name) = i.name {
                            name == &image_string && i.skipall_by_this_name
                        } else {
                            false
                        }
                    });

                    // Check if we've already processed this image+container combo
                    let img_and_container_previously_reviewed =
                        self.images_already_processed.iter().any(|i| {
                            if let Some(ref name) = i.name {
                                if let Some(ref container_name) = i.container {
                                    name == &image_string && container_name == &container_nm_string
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        });

                    // Skip if necessary, otherwise process
                    if !self.images_already_processed.is_empty()
                        && (img_is_set_to_skip || img_and_container_previously_reviewed)
                    {
                        continue;
                    }
                    
                    self.read_val_loop(
                        entry,
                        &image_string,
                        &args.build_args,
                        &container_nm_string,
                    )
                    .map_err(|e| RebuildError::Other(e.to_string()))?;

                    // Add to our list of checked images
                    let c = Image {
                        name: Some(image_string),
                        container: Some(container_nm_string),
                        skipall_by_this_name: true,
                    };
                    self.images_already_processed.push(c);
                }
            }
        }

        Ok(())
    }

    /// Build the interactive prompt grammars for rebuild
    fn build_rebuild_grammars(
        entry: &DirEntry,
        custom_img_nm: &str,
        container_name: &str,
    ) -> Vec<GrammarFragment> {
        let mut grammars: Vec<GrammarFragment> = vec![];

        let grm1 = GrammarFragment {
            original_val_for_prompt: Some("Refresh".to_string()),
            shortened_val_for_prompt: None,
            pos: 0,
            prefix: None,
            suffix: Some(" ".to_string()),
            grammar_type: GrammarType::Verbiage,
            can_shorten: false,
            display_at_all: true,
        };
        grammars.push(grm1);

        let grm2 = GrammarFragment {
            original_val_for_prompt: Some(custom_img_nm.to_string()),
            shortened_val_for_prompt: None,
            pos: 1,
            prefix: None,
            suffix: Some(" ".to_string()),
            grammar_type: GrammarType::Image,
            can_shorten: true,
            display_at_all: true,
        };
        grammars.push(grm2);

        let grm3 = GrammarFragment {
            original_val_for_prompt: Some("from".to_string()),
            shortened_val_for_prompt: None,
            pos: 2,
            prefix: None,
            suffix: Some(" ".to_string()),
            grammar_type: GrammarType::Verbiage,
            can_shorten: false,
            display_at_all: true,
        };
        grammars.push(grm3);

        let docker_compose_pth = entry
            .path()
            .parent()
            .unwrap_or_else(|| Path::new("/"))
            .display()
            .to_string();
        let grm4 = GrammarFragment {
            original_val_for_prompt: Some(docker_compose_pth),
            shortened_val_for_prompt: None,
            pos: 3,
            prefix: None,
            suffix: Some("? ".to_string()),
            grammar_type: GrammarType::DockerComposePath,
            can_shorten: true,
            display_at_all: true,
        };
        grammars.push(grm4);

        let grm5 = GrammarFragment {
            original_val_for_prompt: Some(container_name.to_string()),
            shortened_val_for_prompt: None,
            pos: 4,
            prefix: None,
            suffix: None,
            grammar_type: GrammarType::ContainerName,
            can_shorten: true,
            display_at_all: false,
        };
        grammars.push(grm5);

        let choices = ["p", "N", "d", "b", "s", "?"];
        for (i, &c) in choices.iter().enumerate() {
            let mut sep = Some("/".to_string());
            if i == choices.len() - 1 {
                sep = Some(": ".to_string());
            }
            let choice_grammar = GrammarFragment {
                original_val_for_prompt: Some(c.to_string()),
                shortened_val_for_prompt: None,
                pos: u8::try_from(i + 5).unwrap_or(255),
                prefix: None,
                suffix: sep,
                grammar_type: GrammarType::UserChoice,
                can_shorten: false,
                display_at_all: true,
            };
            grammars.push(choice_grammar);
        }

        grammars
    }

    fn read_val_loop(
        &mut self,
        entry: &DirEntry,
        custom_img_nm: &str,
        build_args: &[String],
        container_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // use extracted helper to build grammars
        let mut grammars = Self::build_rebuild_grammars(entry, custom_img_nm, container_name);

        loop {
            // Get the terminal width from the command helper instead of passing None
            let term_width = self.cmd_helper.get_terminal_display_width(None);
            let result = self
                .read_val_helper
                .read_val_from_cmd_line_and_proceed(&mut grammars, Some(term_width));

            match result.user_entered_val {
                None => {
                    // Check if it's a Ctrl+C signal
                    if result.was_interrupted {
                        println!("\nOperation cancelled by user");
                        std::process::exit(0);
                    }
                    break;
                }
                Some(user_entered_val) => {
                    if self.handle_user_choice(&user_entered_val, entry, custom_img_nm, build_args, container_name, &grammars)? {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_user_choice(
        &mut self,
        user_entered_val: &str,
        entry: &DirEntry,
        custom_img_nm: &str,
        build_args: &[String],
        container_name: &str,
        grammars: &[GrammarFragment],
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match user_entered_val {
            "p" => {
                self.pull_image(custom_img_nm)
                    .unwrap_or_else(|e| eprintln!("Error pulling image: {e}"));
                Ok(true)
            }
            "N" => Ok(true),
            "d" => {
                self.display_image_info(custom_img_nm, container_name, entry, grammars);
                Ok(false)
            }
            "?" => {
                Self::display_help();
                Ok(false)
            }
            "b" => {
                start(
                    entry,
                    custom_img_nm,
                    &build_args.iter().map(std::string::String::as_str).collect::<Vec<_>>(),
                )?;
                Ok(true)
            }
            "s" => {
                let c = Image {
                    name: Some(custom_img_nm.to_string()),
                    container: Some(container_name.to_string()),
                    skipall_by_this_name: true,
                };
                self.images_already_processed.push(c);
                Ok(true)
            }
            _ => {
                eprintln!("Invalid input. Please enter p/N/d/b/s/?: ");
                Ok(false)
            }
        }
    }

    fn display_image_info(
        &self,
        custom_img_nm: &str,
        container_name: &str,
        entry: &DirEntry,
        grammars: &[GrammarFragment],
    ) {
        println!("Image: {custom_img_nm}");
        println!("Container name: {container_name}");
        println!(
            "Compose file: {}",
            grammars[3].original_val_for_prompt.as_ref().unwrap()
        );
        
        // Display image creation time
        match podman_utils::get_podman_image_upstream_create_time(custom_img_nm) {
            Ok(created_time) => {
                println!("Created: {}", Self::format_time_ago(created_time));
            }
            Err(e) => {
                println!("Created: Error getting creation time - {e}");
            }
        }

        // Display image pull time
        match podman_utils::get_podman_ondisk_modify_time(custom_img_nm) {
            Ok(pull_time) => {
                println!("Pulled: {}", Self::format_time_ago(pull_time));
            }
            Err(e) => {
                println!("Pulled: Error getting pull time - {e}");
            }
        }

        // Get parent directory safely
        let parent_dir = entry.path().parent().unwrap_or_else(|| Path::new("/"));

        // Check if Dockerfile exists
        println!(
            "Dockerfile exists: {}",
            self.cmd_helper
                .file_exists_and_readable(&parent_dir.join("Dockerfile"))
        );

        // Check if Makefile exists
        println!(
            "Makefile exists: {}",
            self.cmd_helper
                .file_exists_and_readable(&parent_dir.join("Makefile"))
        );
    }

    fn display_help() {
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

    fn format_time_ago(dt: DateTime<Local>) -> String {
        let now = Local::now();
        let duration = now.signed_duration_since(dt);
        let days = duration.num_days();
        let hours = duration.num_hours();
        let minutes = duration.num_minutes();
        let seconds = duration.num_seconds();
        if days > 0 {
            format!("{days} days ago")
        } else if hours > 0 {
            format!("{hours} hours ago")
        } else if minutes > 0 {
            format!("{minutes} minutes ago")
        } else {
            format!("{seconds} seconds ago")
        }
    }

    // other methods...

    /// Read and parse a YAML file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Unable to open the file
    /// - Unable to parse the file as YAML
    fn read_yaml_file(file_path: &str) -> Result<Value, RebuildError> {
        // Open the file
        let file = File::open(file_path).map_err(RebuildError::Io)?;

        // Parse as YAML
        let yaml: Value = serde_yaml::from_reader(file).map_err(RebuildError::YamlParse)?;

        Ok(yaml)
    }

    /// Process a .container file for rebuilding images
    fn process_container_file(&mut self, entry: &DirEntry, args: &Args) -> Result<(), RebuildError> {
        let file_path = entry.path().to_str().ok_or_else(|| {
            RebuildError::PathNotFound(format!("Invalid UTF-8 in path: {}", entry.path().display()))
        })?;

        // Parse the .container file
        let container_info = parse_container_file(file_path)?;

        // Use the container name from the file, or fallback to the filename
        let container_name = container_info.name.unwrap_or_else(|| {
            entry
                .path()
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("unknown")
                .to_string()
        });

        // Check if this image should be skipped
        let img_is_set_to_skip = self.images_already_processed.iter().any(|i| {
            if let Some(ref name) = i.name {
                name == &container_info.image && i.skipall_by_this_name
            } else {
                false
            }
        });

        // Check if we've already processed this image+container combo
        let img_and_container_previously_reviewed = self.images_already_processed.iter().any(|i| {
            if let Some(ref name) = i.name {
                if let Some(ref container) = i.container {
                    name == &container_info.image && container == &container_name
                } else {
                    false
                }
            } else {
                false
            }
        });

        // Skip if necessary, otherwise process
        if !self.images_already_processed.is_empty()
            && (img_is_set_to_skip || img_and_container_previously_reviewed)
        {
            return Ok(());
        }

        // Process the container image
        self.read_val_loop(entry, &container_info.image, &args.build_args, &container_name)
            .map_err(|e| RebuildError::Other(e.to_string()))?;

        // Add to our list of checked images
        let processed_image = Image {
            name: Some(container_info.image),
            container: Some(container_name),
            skipall_by_this_name: true,
        };
        self.images_already_processed.push(processed_image);

        Ok(())
    }

    /// Pull a container image using podman
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The podman command fails to execute
    /// - The command execution returns a non-zero exit code
    fn pull_image(&mut self, image: &str) -> Result<(), RebuildError> {
        let podman_args = vec!["pull".to_string(), image.to_string()];

        self.cmd_helper
            .exec_cmd("podman", podman_args)
            .map_err(|e| {
                RebuildError::CommandExecution(format!("Failed to pull image {image}: {e}"))
            })
    }
}
