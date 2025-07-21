use crate::args::Args;
use crate::image_build::container_file::parse_container_file;
use crate::interfaces::{CommandHelper, ReadInteractiveInputHelper};

use walkdir::DirEntry;

use super::errors::RebuildError;
use super::interaction::read_val_loop;
use super::types::Image;

/// Process a .container file for rebuilding images
pub fn process_container_file<C: CommandHelper, R: ReadInteractiveInputHelper>(
    cmd_helper: &C,
    read_val_helper: &R,
    images_already_processed: &mut Vec<Image>,
    entry: &DirEntry,
    args: &Args,
) -> Result<(), RebuildError> {
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
    let img_is_set_to_skip = images_already_processed.iter().any(|i| {
        if let Some(ref name) = i.name {
            name == &container_info.image && i.skipall_by_this_name
        } else {
            false
        }
    });

    // Check if we've already processed this image+container combo
    let img_and_container_previously_reviewed = images_already_processed.iter().any(|i| {
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
    if !images_already_processed.is_empty()
        && (img_is_set_to_skip || img_and_container_previously_reviewed)
    {
        return Ok(());
    }

    // Process the container image
    read_val_loop(
        cmd_helper,
        read_val_helper,
        images_already_processed,
        entry,
        &container_info.image,
        &args.build_args,
        &container_name,
    )
    .map_err(|e| RebuildError::Other(e.to_string()))?;

    // Add to our list of checked images
    let processed_image = Image {
        name: Some(container_info.image),
        container: Some(container_name),
        skipall_by_this_name: true,
    };
    images_already_processed.push(processed_image);

    Ok(())
}
