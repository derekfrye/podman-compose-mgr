use std::fs::File;
use std::io::{self};
use podman_compose_mgr::start::walk_dirs;
use podman_compose_mgr::Args;
use tempfile::NamedTempFile;

#[test]
fn test1() -> io::Result<()> {
    let temp_file = NamedTempFile::new_in("/tmp")?;
    let file_path = temp_file.path().to_owned();

    // writeln!(temp_file.as_file(), "This is a temporary file!")?;
    // println!("Created unique file: {}", file_path.display());
    // If you want to persist the file instead of it being deleted when `temp_file` is dropped:
    let _persisted_file = File::create(&file_path)?;

    // walk_dirs(&Args {
    //     exclude_path_patterns: vec![],
    //     include_path_patterns: vec![],
    //     ..Default::default()
    // });

    //now remove it
    temp_file.close()?;

    Ok(())
}
