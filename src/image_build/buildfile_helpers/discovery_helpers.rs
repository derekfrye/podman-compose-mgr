use crate::image_build::buildfile_types::{BuildChoice, BuildFile};
use walkdir::DirEntry;

/// Find buildfiles in a directory
///
/// # Panics
/// Panics if directory path operations fail or parent directory cannot be determined
#[must_use]
pub fn find_buildfile(
    dir: &DirEntry,
    custom_img_nm: &str,
    build_args: &[&str],
) -> Option<Vec<BuildFile>> {
    let parent_dir = dir.path().to_path_buf().parent().unwrap().to_path_buf();
    let dockerfile = parent_dir.join("Dockerfile");
    let makefile = parent_dir.join("Makefile");
    let mut buildfiles: Option<Vec<BuildFile>> = None;

    for file_path in &[&dockerfile, &makefile] {
        let buildfile = BuildFile {
            filetype: match file_path {
                _ if *file_path == &makefile => BuildChoice::Makefile,
                _ => BuildChoice::Dockerfile,
            },
            filepath: if let Ok(metadata) = file_path.symlink_metadata() {
                if metadata.file_type().is_symlink() {
                    Some(std::fs::read_link(file_path).unwrap().clone())
                } else if metadata.is_file() {
                    Some((*file_path).clone())
                } else {
                    None
                }
            } else {
                None
            },
            parent_dir: parent_dir.clone(),
            link_target_dir: if std::fs::read_link(file_path).is_ok() {
                Some(std::fs::read_link(file_path).unwrap().clone())
            } else {
                None
            },
            base_image: Some(custom_img_nm.to_string()),
            custom_img_nm: Some(custom_img_nm.to_string()),
            build_args: build_args.iter().map(|arg| (*arg).to_string()).collect(),
        };

        match &mut buildfiles {
            Some(files) => {
                files.push(buildfile);
            }
            None => {
                buildfiles = Some(vec![buildfile]);
            }
        }
    }

    buildfiles
}
