use crate::domain::{DiscoveredDockerfile, DiscoveredImage, InferenceSource};
use crate::errors::PodmanComposeMgrError;
use crate::image_build::container_file::parse_container_file;
use crate::infra::discovery_types::DirInfo;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;
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

#[must_use]
pub fn build_dockerfile_rows<S: BuildHasher>(
    dir_info: HashMap<std::path::PathBuf, DirInfo, S>,
) -> Vec<DiscoveredDockerfile> {
    let mut dockerfiles = Vec::new();
    for (dir, info) in dir_info {
        if info.dockerfiles.is_empty() {
            continue;
        }

        let neighbor_count = info.compose_files.len() + info.container_files.len();
        for dockerfile_path in &info.dockerfiles {
            let mut neighbor_image = None;
            let mut quadlet_basename = None;
            if info.dockerfiles.len() == 1 && neighbor_count == 1 {
                if info.container_files.len() == 1 {
                    if let Ok(parsed) = parse_container_file(&info.container_files[0].path) {
                        neighbor_image = Some((InferenceSource::Quadlet, parsed.image.clone()));
                        quadlet_basename = info.container_files[0]
                            .path
                            .file_name()
                            .map(|name| name.to_string_lossy().to_string());
                    }
                } else if info.compose_files.len() == 1
                    && let Some(image) = info.compose_files[0].first_image.clone()
                {
                    neighbor_image = Some((InferenceSource::Compose, image));
                }
            }

            let basename = dockerfile_path.file_name().map_or_else(
                || "Dockerfile".to_string(),
                |name| name.to_string_lossy().to_string(),
            );

            dockerfiles.push(DiscoveredDockerfile {
                dockerfile_path: dockerfile_path.clone(),
                source_dir: dir.clone(),
                basename,
                quadlet_basename,
                neighbor_image,
                total_dockerfiles_in_dir: info.dockerfiles.len(),
                neighbor_file_count: neighbor_count,
            });
        }
    }
    dockerfiles.sort_by(|a, b| a.basename.cmp(&b.basename));
    dockerfiles
}

/// Collect rows from a docker-compose.yml entry.
///
/// # Panics
///
/// Panics if `image` is unexpectedly `None` after being checked.
pub fn collect_from_compose<S: BuildHasher>(
    entry: &walkdir::DirEntry,
    path_str: &str,
    seen: &mut HashSet<(String, Option<String>, std::path::PathBuf), S>,
    rows: &mut Vec<DiscoveredImage>,
) -> Option<String> {
    let yaml = read_yaml_file_local(path_str)?;
    let services = yaml
        .get(serde_yaml::Value::String("services".into()))
        .and_then(|value| value.as_mapping())?;

    let mut first_image = None;
    for (svc_name, svc_cfg) in services {
        let Some(svc_cfg) = svc_cfg.as_mapping() else {
            continue;
        };
        let image = yaml_get_string(svc_cfg, "image");
        if image.is_none() {
            continue;
        }

        if first_image.is_none() {
            first_image.clone_from(&image);
        }

        let mut container = yaml_get_string(svc_cfg, "container_name");
        if container.is_none()
            && let Some(name) = svc_name.as_str()
        {
            container = Some(name.to_string());
        }

        add_row(entry, image.unwrap(), container, seen, rows);
    }
    first_image
}

pub fn collect_from_container<S: BuildHasher>(
    entry: &walkdir::DirEntry,
    seen: &mut HashSet<(String, Option<String>, std::path::PathBuf), S>,
    rows: &mut Vec<DiscoveredImage>,
) {
    if entry.path().extension().and_then(|s| s.to_str()) != Some("container") {
        return;
    }

    let Ok(info) = parse_container_file(entry.path()) else {
        return;
    };

    add_row(entry, info.image, info.name, seen, rows);
}

pub fn add_row<S: BuildHasher>(
    entry: &walkdir::DirEntry,
    image: String,
    container: Option<String>,
    seen: &mut HashSet<(String, Option<String>, std::path::PathBuf), S>,
    rows: &mut Vec<DiscoveredImage>,
) {
    let source_dir = entry
        .path()
        .parent()
        .unwrap_or_else(|| Path::new("/"))
        .to_path_buf();

    let entry_path = entry.path().to_path_buf();
    let key = (image.clone(), container.clone(), source_dir.clone());
    if seen.insert(key) {
        rows.push(DiscoveredImage {
            image,
            container,
            source_dir,
            entry_path,
        });
    }
}

fn yaml_get_string(m: &serde_yaml::Mapping, key: &str) -> Option<String> {
    m.get(serde_yaml::Value::String(key.to_string()))
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
}

fn read_yaml_file_local(path: &str) -> Option<serde_yaml::Value> {
    use std::fs::File;
    let file = File::open(path).ok()?;
    serde_yaml::from_reader(file).ok()
}
