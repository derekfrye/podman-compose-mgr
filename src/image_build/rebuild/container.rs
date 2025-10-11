use crate::args::Args;
use crate::image_build::container_file::parse_container_file;
use crate::interfaces::{CommandHelper, ReadInteractiveInputHelper};
use crate::utils::build_logger::BuildLogger;

use walkdir::DirEntry;

use super::compose::{invoke_read_loop, should_skip_service};
use super::errors::RebuildError;
use super::types::Image;

/// Process a .container file for rebuilding images
pub fn process_container_file<C: CommandHelper, R: ReadInteractiveInputHelper>(
    cmd_helper: &C,
    read_val_helper: &R,
    images_already_processed: &mut Vec<Image>,
    entry: &DirEntry,
    args: &Args,
    logger: &dyn BuildLogger,
) -> Result<(), RebuildError> {
    let file_path = container_path(entry)?;
    let container_info = parse_container_file(file_path)?;
    let container_name = resolve_container_name(entry, &container_info);

    if should_skip_service(
        images_already_processed,
        &container_info.image,
        &container_name,
    ) {
        return Ok(());
    }

    invoke_read_loop(
        cmd_helper,
        read_val_helper,
        images_already_processed,
        entry,
        args,
        &container_info.image,
        &container_name,
        logger,
    )?;

    images_already_processed.push(Image {
        name: Some(container_info.image),
        container: Some(container_name),
        skipall_by_this_name: true,
    });

    Ok(())
}

fn container_path(entry: &DirEntry) -> Result<&str, RebuildError> {
    entry.path().to_str().ok_or_else(|| {
        RebuildError::PathNotFound(format!("Invalid UTF-8 in path: {}", entry.path().display()))
    })
}

fn resolve_container_name(
    entry: &DirEntry,
    container_info: &crate::image_build::container_file::ContainerInfo,
) -> String {
    container_info.name.clone().unwrap_or_else(|| {
        entry
            .path()
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("unknown")
            .to_string()
    })
}
