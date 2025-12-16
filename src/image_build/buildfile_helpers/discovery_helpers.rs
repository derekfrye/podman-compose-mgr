use crate::image_build::buildfile_types::{BuildChoice, BuildFile};
use std::path::{Path, PathBuf};
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
    no_cache: bool,
) -> Option<Vec<BuildFile>> {
    let parent_dir = dir.path().parent()?.to_path_buf();
    let mut buildfiles: Vec<BuildFile> = Vec::new();

    if let Some(candidate) = container_specific_dockerfile(dir)
        && let Some(buildfile) = buildfile_from_candidate(
            &candidate,
            BuildChoice::Dockerfile,
            &parent_dir,
            custom_img_nm,
            build_args,
            no_cache,
        )
    {
        buildfiles.push(buildfile);
    }

    for filename in ["Dockerfile", "Makefile"] {
        let filetype = match filename {
            "Makefile" => BuildChoice::Makefile,
            _ => BuildChoice::Dockerfile,
        };
        let candidate = parent_dir.join(filename);
        if let Some(buildfile) = buildfile_from_candidate(
            &candidate,
            filetype,
            &parent_dir,
            custom_img_nm,
            build_args,
            no_cache,
        ) {
            buildfiles.push(buildfile);
        }
    }

    if buildfiles.is_empty() {
        None
    } else {
        Some(buildfiles)
    }
}

fn container_specific_dockerfile(entry: &DirEntry) -> Option<PathBuf> {
    let path = entry.path();
    path.extension()
        .and_then(|ext| ext.to_str())
        .filter(|ext| *ext == "container")?;

    let base_name = path.file_stem()?.to_string_lossy();
    let parent_dir = path.parent()?;
    Some(parent_dir.join(format!("Dockerfile.{base_name}")))
}

fn buildfile_from_candidate(
    candidate: &Path,
    filetype: BuildChoice,
    parent_dir: &Path,
    custom_img_nm: &str,
    build_args: &[&str],
    no_cache: bool,
) -> Option<BuildFile> {
    let metadata = candidate.symlink_metadata().ok()?;
    if !metadata.is_file() && !metadata.file_type().is_symlink() {
        return None;
    }

    let (filepath, link_target_dir) = if metadata.file_type().is_symlink() {
        match std::fs::read_link(candidate) {
            Ok(target) => (Some(target.clone()), Some(target)),
            Err(_) => return None,
        }
    } else {
        (
            Some(candidate.to_path_buf()),
            std::fs::read_link(candidate).ok(),
        )
    };

    Some(BuildFile {
        filetype,
        filepath,
        parent_dir: parent_dir.to_path_buf(),
        link_target_dir,
        base_image: Some(custom_img_nm.to_string()),
        custom_img_nm: Some(custom_img_nm.to_string()),
        build_args: build_args.iter().map(|arg| (*arg).to_string()).collect(),
        no_cache,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;
    use walkdir::WalkDir;

    fn quadlet_entry(root: &Path, name: &str) -> DirEntry {
        let target = root.join(name);
        WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
            .find(|entry| entry.path() == target)
            .expect("quadlet entry should exist")
    }

    #[test]
    fn prefers_container_specific_dockerfile_when_present() {
        let tmp = tempdir().unwrap();
        let quadlet = tmp.path().join("web.container");
        fs::write(&quadlet, "[Container]\nImage=example\n").unwrap();
        let specific = tmp.path().join("Dockerfile.web");
        let generic = tmp.path().join("Dockerfile");
        fs::write(&specific, "FROM scratch").unwrap();
        fs::write(&generic, "FROM scratch").unwrap();

        let entry = quadlet_entry(tmp.path(), "web.container");
        let files = find_buildfile(&entry, "example", &[], false).expect("files found");
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].filepath.as_deref(), Some(specific.as_path()));
        assert_eq!(files[1].filepath.as_deref(), Some(generic.as_path()));
    }

    #[test]
    fn falls_back_to_generic_dockerfile_and_makefile() {
        let tmp = tempdir().unwrap();
        let quadlet = tmp.path().join("db.container");
        fs::write(&quadlet, "[Container]\nImage=example\n").unwrap();
        let generic = tmp.path().join("Dockerfile");
        let makefile = tmp.path().join("Makefile");
        fs::write(&generic, "FROM scratch").unwrap();
        fs::write(&makefile, "all:\n\techo ok\n").unwrap();

        let entry = quadlet_entry(tmp.path(), "db.container");
        let files = find_buildfile(&entry, "example", &[], false).expect("files found");
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].filepath.as_deref(), Some(generic.as_path()));
        assert_eq!(files[1].filepath.as_deref(), Some(makefile.as_path()));
    }
}
