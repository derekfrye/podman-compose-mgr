use crate::image_build::container_file::parse_container_file;
use crate::utils::log_utils::Logger;
use regex::Regex;
use std::collections::HashSet;
use walkdir::WalkDir;

use crate::Args;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DiscoveredImage {
    pub image: String,
    pub container: Option<String>,
    pub source_dir: std::path::PathBuf,
}

pub fn scan_images(args: &Args, logger: &Logger) -> Vec<DiscoveredImage> {
    // Compile include/exclude
    let mut exclude_patterns: Vec<Regex> = Vec::new();
    let mut include_patterns: Vec<Regex> = Vec::new();
    for p in &args.exclude_path_patterns {
        if let Ok(r) = Regex::new(p) {
            exclude_patterns.push(r);
        }
    }
    for p in &args.include_path_patterns {
        if let Ok(r) = Regex::new(p) {
            include_patterns.push(r);
        }
    }

    let mut seen: HashSet<(String, Option<String>, std::path::PathBuf)> = HashSet::new();
    let mut rows: Vec<DiscoveredImage> = Vec::new();

    for entry in WalkDir::new(&args.path).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path_str = match entry.path().to_str() {
            Some(s) => s,
            None => continue,
        };

        // Exclude
        if !exclude_patterns.is_empty() && exclude_patterns.iter().any(|r| r.is_match(path_str)) {
            continue;
        }
        // Include (if specified)
        if !include_patterns.is_empty() && include_patterns.iter().all(|r| !r.is_match(path_str)) {
            continue;
        }

        // docker-compose.yml
        if entry.file_name() == "docker-compose.yml" {
            if let Some(yaml) = read_yaml_file_local(path_str)
                && let Some(services) = yaml
                    .get(serde_yaml::Value::String("services".into()))
                    .and_then(|v| v.as_mapping())
            {
                for (svc_name, svc_cfg) in services {
                    let svc_cfg = if let Some(m) = svc_cfg.as_mapping() { m } else { continue };
                    // image
                    let image = yaml_get_string(svc_cfg, "image");
                    if image.is_none() {
                        continue;
                    }
                    let mut container = yaml_get_string(svc_cfg, "container_name");
                    if container.is_none() {
                        // fallback to service key as name if scalar
                        if let Some(name) = svc_name.as_str() {
                            container = Some(name.to_string());
                        }
                    }
                        let source_dir = entry
                            .path()
                            .parent()
                            .unwrap_or_else(|| std::path::Path::new("/"))
                            .to_path_buf();
                        let key = (image.clone().unwrap(), container.clone(), source_dir.clone());
                        if seen.insert(key) {
                            rows.push(DiscoveredImage { image: image.unwrap(), container, source_dir });
                        }
                }
            }
            continue;
        }

        // .container
        if entry.path().extension().and_then(|s| s.to_str()) == Some("container") {
            match parse_container_file(entry.path()) {
                Ok(info) => {
                    let source_dir = entry
                        .path()
                        .parent()
                        .unwrap_or_else(|| std::path::Path::new("/"))
                        .to_path_buf();
                    let key = (info.image.clone(), info.name.clone(), source_dir.clone());
                    if seen.insert(key) {
                        rows.push(DiscoveredImage { image: info.image, container: info.name, source_dir });
                    }
                }
                Err(e) => {
                    logger.debug(&format!(
                        "Skipping invalid container file {}: {e}",
                        entry.path().display()
                    ));
                }
            }
        }
    }

    rows.sort_by(|a, b| a.image.cmp(&b.image).then_with(|| a.container.cmp(&b.container)));
    rows
}

fn yaml_get_string(m: &serde_yaml::Mapping, key: &str) -> Option<String> {
    m.get(serde_yaml::Value::String(key.to_string()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn read_yaml_file_local(path: &str) -> Option<serde_yaml::Value> {
    use std::fs::File;
    let file = File::open(path).ok()?;
    serde_yaml::from_reader(file).ok()
}
