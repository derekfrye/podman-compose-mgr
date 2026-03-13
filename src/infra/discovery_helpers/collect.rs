use crate::domain::DiscoveredImage;
use crate::image_build::container_file::parse_container_file;
use std::collections::HashSet;
use std::hash::BuildHasher;
use std::path::Path;

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
        let Some(image) = yaml_get_string(svc_cfg, "image") else {
            continue;
        };
        if first_image.is_none() {
            first_image = Some(image.clone());
        }

        let container = yaml_get_string(svc_cfg, "container_name")
            .or_else(|| svc_name.as_str().map(std::string::ToString::to_string));
        add_row(entry, image, container, seen, rows);
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
        .map(std::string::ToString::to_string)
}

fn read_yaml_file_local(path: &str) -> Option<serde_yaml::Value> {
    let file = std::fs::File::open(path).ok()?;
    serde_yaml::from_reader(file).ok()
}
