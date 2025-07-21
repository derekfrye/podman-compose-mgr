use std::path::Path;
use terminal_size::{self, Width};

/// Check if a file exists and is readable
///
/// # Arguments
/// * `file` - Path to check
///
/// # Returns
/// true if the file exists, is a file (not a directory), and can be read
#[must_use]
pub fn file_exists_and_readable(file: &Path) -> bool {
    match file.try_exists() {
        Ok(true) => file.is_file() && file.metadata().is_ok(),
        _ => false,
    }
}

/// Get the terminal display width
///
/// # Arguments
/// * `specify_size` - Optional size to force
///
/// # Returns
/// The terminal width in columns, or 80 if it can't be determined
#[must_use]
pub fn get_terminal_display_width(specify_size: Option<usize>) -> usize {
    if let Some(size) = specify_size {
        return size;
    }
    let size = terminal_size::terminal_size();
    if let Some((Width(w), _)) = size {
        w as usize
    } else {
        80
    }
}
