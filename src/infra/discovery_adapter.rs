use crate::domain::DiscoveredImage;
use crate::errors::PodmanComposeMgrError;
use crate::image_build::container_file::parse_container_file;
use crate::ports::{DiscoveryPort, ScanOptions};
use regex::Regex;
use std::collections::HashSet;
use std::path::Path;
use walkdir::WalkDir;

pub struct FsDiscovery;

impl DiscoveryPort for FsDiscovery {
    fn scan(&self, opts: &ScanOptions) -> Result<Vec<DiscoveredImage>, PodmanComposeMgrError> {
        let exclude_patterns = compile_regexes(&opts.exclude_patterns)?;
        let include_patterns = compile_regexes(&opts.include_patterns)?;
        let mut seen: HashSet<(String, Option<String>, std::path::PathBuf)> = HashSet::new();
        let mut rows: Vec<DiscoveredImage> = Vec::new();

        for entry in walk_entries(&opts.root) {
            if !entry.file_type().is_file() {
                continue;
            }
            let Some(path_str) = entry.path().to_str() else {
                continue;
            };
            if !should_keep_path(path_str, &exclude_patterns, &include_patterns) {
                continue;
            }

            if entry.file_name() == "docker-compose.yml" {
                collect_from_compose(&entry, path_str, &mut seen, &mut rows);
                continue;
            }

            collect_from_container(&entry, &mut seen, &mut rows);
        }

        rows.sort_by(|a, b| {
            a.image
                .cmp(&b.image)
                .then_with(|| a.container.cmp(&b.container))
        });
        Ok(rows)
    }
}

fn compile_regexes(patterns: &[String]) -> Result<Vec<Regex>, PodmanComposeMgrError> {
    patterns
        .iter()
        .map(|pattern| Regex::new(pattern))
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn walk_entries(root: &Path) -> impl Iterator<Item = walkdir::DirEntry> {
    WalkDir::new(root).into_iter().filter_map(Result::ok)
}

fn should_keep_path(path: &str, exclude_patterns: &[Regex], include_patterns: &[Regex]) -> bool {
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

fn collect_from_compose(
    entry: &walkdir::DirEntry,
    path_str: &str,
    seen: &mut HashSet<(String, Option<String>, std::path::PathBuf)>,
    rows: &mut Vec<DiscoveredImage>,
) {
    let Some(yaml) = read_yaml_file_local(path_str) else {
        return;
    };
    let Some(services) = yaml
        .get(serde_yaml::Value::String("services".into()))
        .and_then(|value| value.as_mapping())
    else {
        return;
    };

    for (svc_name, svc_cfg) in services {
        let Some(svc_cfg) = svc_cfg.as_mapping() else {
            continue;
        };
        let image = yaml_get_string(svc_cfg, "image");
        if image.is_none() {
            continue;
        }

        let mut container = yaml_get_string(svc_cfg, "container_name");
        if container.is_none()
            && let Some(name) = svc_name.as_str()
        {
            container = Some(name.to_string());
        }

        add_row(entry, image.unwrap(), container, seen, rows);
    }
}

fn collect_from_container(
    entry: &walkdir::DirEntry,
    seen: &mut HashSet<(String, Option<String>, std::path::PathBuf)>,
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

fn add_row(
    entry: &walkdir::DirEntry,
    image: String,
    container: Option<String>,
    seen: &mut HashSet<(String, Option<String>, std::path::PathBuf)>,
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
