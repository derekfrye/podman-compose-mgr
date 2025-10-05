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
        let mut exclude_patterns: Vec<Regex> = Vec::new();
        let mut include_patterns: Vec<Regex> = Vec::new();
        for p in &opts.exclude_patterns {
            exclude_patterns.push(Regex::new(p)?);
        }
        for p in &opts.include_patterns {
            include_patterns.push(Regex::new(p)?);
        }

        let mut seen: HashSet<(String, Option<String>, std::path::PathBuf)> = HashSet::new();
        let mut rows: Vec<DiscoveredImage> = Vec::new();

        for entry in WalkDir::new(&opts.root).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path_str = match entry.path().to_str() {
                Some(s) => s,
                None => continue,
            };

            if !exclude_patterns.is_empty() && exclude_patterns.iter().any(|r| r.is_match(path_str)) {
                continue;
            }
            if !include_patterns.is_empty() && include_patterns.iter().all(|r| !r.is_match(path_str)) {
                continue;
            }

            if entry.file_name() == "docker-compose.yml" {
                if let Some(yaml) = read_yaml_file_local(path_str)
                    && let Some(services) = yaml
                        .get(serde_yaml::Value::String("services".into()))
                        .and_then(|v| v.as_mapping())
                {
                    for (svc_name, svc_cfg) in services {
                        let svc_cfg = if let Some(m) = svc_cfg.as_mapping() { m } else { continue };
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
                        let source_dir = entry
                            .path()
                            .parent()
                            .unwrap_or_else(|| Path::new("/"))
                            .to_path_buf();
                        let key = (image.clone().unwrap(), container.clone(), source_dir.clone());
                        if seen.insert(key) {
                            rows.push(DiscoveredImage { image: image.unwrap(), container, source_dir });
                        }
                    }
                }
                continue;
            }

            if entry.path().extension().and_then(|s| s.to_str()) == Some("container")
                && let Ok(info) = parse_container_file(entry.path())
            {
                    let source_dir = entry
                        .path()
                        .parent()
                        .unwrap_or_else(|| Path::new("/"))
                        .to_path_buf();
                    let key = (info.image.clone(), info.name.clone(), source_dir.clone());
                    if seen.insert(key) {
                        rows.push(DiscoveredImage { image: info.image, container: info.name, source_dir });
                    }
            }
        }

        rows.sort_by(|a, b| a.image.cmp(&b.image).then_with(|| a.container.cmp(&b.container)));
        Ok(rows)
    }
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
