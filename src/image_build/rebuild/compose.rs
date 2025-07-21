use crate::args::Args;
use crate::interfaces::{CommandHelper, ReadInteractiveInputHelper};

use walkdir::DirEntry;

use super::errors::RebuildError;
use super::interaction::read_val_loop;
use super::types::Image;
use super::utils::read_yaml_file;

/// Process a docker-compose.yml file for rebuilding images
pub fn process_compose_file<C: CommandHelper, R: ReadInteractiveInputHelper>(
    cmd_helper: &C,
    read_val_helper: &R,
    images_already_processed: &mut Vec<Image>,
    entry: &DirEntry,
    args: &Args,
) -> Result<(), RebuildError> {
    let file_path = entry.path().to_str().ok_or_else(|| {
        RebuildError::PathNotFound(format!("Invalid UTF-8 in path: {}", entry.path().display()))
    })?;

    let yaml = read_yaml_file(file_path)?;

    // Get services from YAML
    let services = yaml.get("services").ok_or_else(|| {
        RebuildError::MissingField("No 'services' section found in compose file".to_string())
    })?;

    let services_map = services
        .as_mapping()
        .ok_or_else(|| RebuildError::InvalidConfig("'services' is not a mapping".to_string()))?;

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
                        RebuildError::InvalidConfig("'container_name' is not a string".to_string())
                    })?
                    .to_string();

                // Check if this image should be skipped
                let img_is_set_to_skip = images_already_processed.iter().any(|i| {
                    if let Some(ref name) = i.name {
                        name == &image_string && i.skipall_by_this_name
                    } else {
                        false
                    }
                });

                // Check if we've already processed this image+container combo
                let img_and_container_previously_reviewed =
                    images_already_processed.iter().any(|i| {
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
                if !images_already_processed.is_empty()
                    && (img_is_set_to_skip || img_and_container_previously_reviewed)
                {
                    continue;
                }

                read_val_loop(
                    cmd_helper,
                    read_val_helper,
                    images_already_processed,
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
                images_already_processed.push(c);
            }
        }
    }

    Ok(())
}
