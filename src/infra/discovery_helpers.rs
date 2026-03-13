mod buildfile_rows;
mod collect;
mod makefile_parse;

pub use buildfile_rows::{build_dockerfile_rows, build_makefile_rows};
pub use collect::{add_row, collect_from_compose, collect_from_container};

use crate::errors::PodmanComposeMgrError;
use regex::Regex;
use std::path::Path;
use walkdir::WalkDir;

/// Compile the provided regex patterns.
///
/// # Errors
///
/// Returns an error if any pattern is not a valid regex.
pub fn compile_regexes(patterns: &[String]) -> Result<Vec<Regex>, PodmanComposeMgrError> {
    patterns
        .iter()
        .map(|pattern| Regex::new(pattern))
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn walk_entries(root: &Path) -> impl Iterator<Item = walkdir::DirEntry> {
    WalkDir::new(root).into_iter().filter_map(Result::ok)
}

#[must_use]
pub fn should_keep_path(
    path: &str,
    exclude_patterns: &[Regex],
    include_patterns: &[Regex],
) -> bool {
    if !exclude_patterns.is_empty()
        && exclude_patterns
            .iter()
            .any(|pattern| pattern.is_match(path))
    {
        return false;
    }

    if !include_patterns.is_empty()
        && include_patterns
            .iter()
            .all(|pattern| !pattern.is_match(path))
    {
        return false;
    }

    true
}
