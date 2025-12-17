use home::home_dir;
use std::path::{Path, PathBuf};

// A helper function to expand the tilde in a path to the user's home directory.
pub(super) fn expand_tilde(path: &Path) -> Result<PathBuf, String> {
    if path.starts_with("~") {
        if let Some(home) = home_dir() {
            Ok(home.join(path.strip_prefix("~").unwrap_or(path)))
        } else {
            Err("Home directory could not be determined.".to_string())
        }
    } else {
        Ok(path.to_path_buf())
    }
}
