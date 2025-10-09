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
    let file_path = compose_path(entry)?;
    let yaml = read_yaml_file(file_path)?;
    let services = extract_services(&yaml)?;

    for service_config in services.values() {
        let Some((image, container)) = service_names(service_config)? else {
            continue;
        };

        if should_skip_service(images_already_processed, &image, &container) {
            continue;
        }

        invoke_read_loop(
            cmd_helper,
            read_val_helper,
            images_already_processed,
            entry,
            args,
            &image,
            &container,
        )?;

        images_already_processed.push(Image {
            name: Some(image),
            container: Some(container),
            skipall_by_this_name: true,
        });
    }

    Ok(())
}

fn compose_path(entry: &DirEntry) -> Result<&str, RebuildError> {
    entry.path().to_str().ok_or_else(|| {
        RebuildError::PathNotFound(format!("Invalid UTF-8 in path: {}", entry.path().display()))
    })
}

fn extract_services(yaml: &serde_yaml::Value) -> Result<&serde_yaml::Mapping, RebuildError> {
    let services = yaml.get("services").ok_or_else(|| {
        RebuildError::MissingField("No 'services' section found in compose file".to_string())
    })?;

    services
        .as_mapping()
        .ok_or_else(|| RebuildError::InvalidConfig("'services' is not a mapping".to_string()))
}

fn service_names(
    service_config: &serde_yaml::Value,
) -> Result<Option<(String, String)>, RebuildError> {
    let Some(mapping) = service_config.as_mapping() else {
        return Ok(None);
    };

    let Some(image_value) = mapping.get("image") else {
        return Ok(None);
    };
    let Some(container_value) = mapping.get("container_name") else {
        return Ok(None);
    };

    let image = image_value.as_str().ok_or_else(|| {
        RebuildError::InvalidConfig("'image' is not a string".to_string())
    })?;
    let container = container_value.as_str().ok_or_else(|| {
        RebuildError::InvalidConfig("'container_name' is not a string".to_string())
    })?;

    Ok(Some((image.to_string(), container.to_string())))
}

pub(super) fn should_skip_service(images: &[Image], image: &str, container: &str) -> bool {
    if images.is_empty() {
        return false;
    }

    let skip_all = images
        .iter()
        .any(|item| item.name.as_deref() == Some(image) && item.skipall_by_this_name);
    let already_seen = images.iter().any(|item| {
        item.name.as_deref() == Some(image) && item.container.as_deref() == Some(container)
    });

    skip_all || already_seen
}

pub(super) fn invoke_read_loop<C: CommandHelper, R: ReadInteractiveInputHelper>(
    cmd_helper: &C,
    read_val_helper: &R,
    images_already_processed: &mut Vec<Image>,
    entry: &DirEntry,
    args: &Args,
    image: &str,
    container: &str,
) -> Result<(), RebuildError> {
    read_val_loop(
        cmd_helper,
        read_val_helper,
        images_already_processed,
        entry,
        image,
        &args.build_args,
        container,
    )
    .map_err(|e| RebuildError::Other(e.to_string()))
}
