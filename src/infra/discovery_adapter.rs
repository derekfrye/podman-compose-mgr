use crate::domain::{DiscoveredImage, DiscoveryResult};
use crate::errors::PodmanComposeMgrError;
use crate::infra::discovery_helpers::{
    build_dockerfile_rows, build_makefile_rows, collect_from_compose, collect_from_container,
    compile_regexes, should_keep_path, walk_entries,
};
use crate::infra::discovery_types::{ComposeInfo, ContainerInfo, DirInfo};
use crate::ports::{DiscoveryPort, ScanOptions};
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub struct FsDiscovery;

impl DiscoveryPort for FsDiscovery {
    fn scan(&self, opts: &ScanOptions) -> Result<DiscoveryResult, PodmanComposeMgrError> {
        let exclude_patterns = compile_regexes(&opts.exclude_patterns)?;
        let include_patterns = compile_regexes(&opts.include_patterns)?;
        let mut seen: HashSet<(String, Option<String>, std::path::PathBuf)> = HashSet::new();
        let mut rows: Vec<DiscoveredImage> = Vec::new();
        let mut dir_info: HashMap<std::path::PathBuf, DirInfo> = HashMap::new();

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
                let first_image = collect_from_compose(&entry, path_str, &mut seen, &mut rows);
                dir_info
                    .entry(
                        entry
                            .path()
                            .parent()
                            .unwrap_or_else(|| Path::new("/"))
                            .to_path_buf(),
                    )
                    .or_default()
                    .compose_files
                    .push(ComposeInfo { first_image });
                continue;
            }

            if entry.path().extension().and_then(|s| s.to_str()) == Some("container") {
                collect_from_container(&entry, &mut seen, &mut rows);
                dir_info
                    .entry(
                        entry
                            .path()
                            .parent()
                            .unwrap_or_else(|| Path::new("/"))
                            .to_path_buf(),
                    )
                    .or_default()
                    .container_files
                    .push(ContainerInfo {
                        path: entry.path().to_path_buf(),
                    });
                continue;
            }

            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("Dockerfile") {
                    dir_info
                        .entry(
                            entry
                                .path()
                                .parent()
                                .unwrap_or_else(|| Path::new("/"))
                                .to_path_buf(),
                        )
                        .or_default()
                        .dockerfiles
                        .push(entry.path().to_path_buf());
                    continue;
                }
                if name == "Makefile" {
                    dir_info
                        .entry(
                            entry
                                .path()
                                .parent()
                                .unwrap_or_else(|| Path::new("/"))
                                .to_path_buf(),
                        )
                        .or_default()
                        .makefiles
                        .push(entry.path().to_path_buf());
                }
            }
        }

        rows.sort_by(|a, b| {
            a.image
                .cmp(&b.image)
                .then_with(|| a.container.cmp(&b.container))
        });

        let dockerfiles = build_dockerfile_rows(&dir_info);
        let makefiles = build_makefile_rows(&dir_info);

        Ok(DiscoveryResult {
            images: rows,
            dockerfiles,
            makefiles,
        })
    }
}
