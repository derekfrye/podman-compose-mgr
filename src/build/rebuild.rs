use crate::args::Args;
use crate::build::buildfile::start;
use crate::interfaces::{CommandHelper, ReadValHelper};
use crate::build::image::Image;
use crate::build::prompt::{create_rebuild_grammars, add_choice_options};
use crate::build::rebuild_error::RebuildError;
use crate::build::rebuild_yaml::{read_yaml_file, extract_services, extract_image_info};
use crate::build::rebuild_helpers::{pull_image, display_image_info, should_skip_image};
use std::path::Path;
use walkdir::DirEntry;

pub struct RebuildManager<'a, C: CommandHelper, R: ReadValHelper> {
    images_checked: Vec<Image>,
    cmd_helper: &'a C,
    read_val_helper: &'a R,
}

impl<'a, C: CommandHelper, R: ReadValHelper> RebuildManager<'a, C, R> {
    pub fn new(cmd_helper: &'a C, read_val_helper: &'a R) -> Self {
        Self {
            images_checked: Vec::new(),
            cmd_helper,
            read_val_helper,
        }
    }
    
    /// Process a single service from docker-compose.yml
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Unable to extract image info
    /// - Unable to process the image
    fn process_service(
        &mut self, 
        service_config: &serde_yaml::Value, 
        entry: &DirEntry, 
        args: &Args
    ) -> Result<(), RebuildError> {
        if let Some((image_string, container_nm_string)) = extract_image_info(service_config)? {
            // Skip if necessary, otherwise process
            if should_skip_image(&self.images_checked, &image_string, &container_nm_string) {
                return Ok(());
            }
            
            // Process the image
            self.read_val_loop(
                entry,
                &image_string,
                &args.build_args,
                &container_nm_string,
            ).map_err(|e| RebuildError::Other(e.to_string()))?;

            // Add to our list of checked images
            let c = Image {
                name: Some(image_string),
                container: Some(container_nm_string),
                skipall_by_this_name: false,
            };
            self.images_checked.push(c);
        }
        
        Ok(())
    }

    /// Process a docker-compose.yml file for rebuilding images
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Unable to get the file path as a string
    /// - Unable to read or parse the YAML file
    /// - Service configurations are invalid
    pub fn rebuild(
        &mut self,
        entry: &DirEntry,
        args: &Args,
    ) -> Result<(), RebuildError> {
        // Get file path safely
        let file_path = entry.path().to_str()
            .ok_or_else(|| RebuildError::PathNotFound(format!("Invalid UTF-8 in path: {:?}", entry.path())))?;
        
        // Read and parse YAML
        let yaml = read_yaml_file(file_path)?;
        
        // Extract services from YAML
        let services_map = extract_services(&yaml)?;
        
        // Process each service
        for (_, service_config) in services_map {
            self.process_service(&service_config, entry, args)?;
        }
        
        Ok(())
    }

    /// Handle specific user choice action
    fn handle_user_choice(
        &mut self,
        choice: &str,
        entry: &DirEntry,
        custom_img_nm: &str,
        build_args: &[String],
        container_name: &str,
        docker_compose_pth: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match choice {
            // Pull the image
            "p" => {
                pull_image(self.cmd_helper, custom_img_nm)
                    .unwrap_or_else(|e| eprintln!("Error pulling image: {}", e));
                Ok(true) // Exit loop
            }
            // Do nothing
            "N" => Ok(true), // Exit loop
            // Display info
            "d" => {
                display_image_info(self.cmd_helper, custom_img_nm, container_name, docker_compose_pth, entry);
                Ok(false) // Continue loop
            },
            // Display help
            "?" => {
                crate::build::ui::display_help();
                Ok(false) // Continue loop
            },
            // Build image
            "b" => {
                start(
                    entry,
                    custom_img_nm,
                    build_args.iter().map(|s| s.as_str()).collect(),
                )?;
                Ok(true) // Exit loop
            }
            // Skip all images with this name
            "s" => {
                let c = Image {
                    name: Some(custom_img_nm.to_string()),
                    container: Some(container_name.to_string()),
                    skipall_by_this_name: true,
                };
                self.images_checked.push(c);
                Ok(true) // Exit loop
            }
            // Invalid input
            _ => {
                eprintln!("Invalid input. Please enter p/N/d/b/s/?: ");
                Ok(false) // Continue loop
            }
        }
    }

    fn read_val_loop(
        &mut self,
        entry: &DirEntry,
        custom_img_nm: &str,
        build_args: &[String],
        container_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Setup grammars for the prompt
        let mut grammars = create_rebuild_grammars(custom_img_nm, entry, container_name);
        add_choice_options(&mut grammars);
        
        // Get compose path for display purposes
        let docker_compose_pth = entry
            .path()
            .parent()
            .unwrap_or_else(|| Path::new("/"))
            .display()
            .to_string();

        loop {
            // Display prompt and get user input
            let term_width = self.cmd_helper.get_terminal_display_width(None);
            let result = self
                .read_val_helper
                .read_val_from_cmd_line_and_proceed(&mut grammars, Some(term_width));

            match result.user_entered_val {
                None => break,
                Some(user_entered_val) => {
                    // Handle user choice and determine if we should exit the loop
                    let should_break = self.handle_user_choice(
                        &user_entered_val,
                        entry,
                        custom_img_nm,
                        build_args,
                        container_name,
                        &docker_compose_pth,
                    )?;
                    
                    if should_break {
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}