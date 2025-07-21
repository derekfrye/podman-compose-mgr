use crate::args::Args;
use crate::interfaces::{CommandHelper, ReadInteractiveInputHelper};

use walkdir::DirEntry;

use super::compose::process_compose_file;
use super::container::process_container_file;
use super::errors::RebuildError;
use super::types::Image;

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
            process_container_file(
                self.cmd_helper,
                self.read_val_helper,
                &mut self.images_already_processed,
                entry,
                args,
            )
        } else if file_path.ends_with("docker-compose.yml") {
            process_compose_file(
                self.cmd_helper,
                self.read_val_helper,
                &mut self.images_already_processed,
                entry,
                args,
            )
        } else {
            Err(RebuildError::InvalidConfig(format!(
                "Unsupported file type: {file_path}"
            )))
        }
    }
}
