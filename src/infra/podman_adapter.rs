use crate::domain::LocalImageSummary;
use crate::errors::PodmanComposeMgrError;
use crate::ports::PodmanPort;
use chrono::{DateTime, Local};
use serde::de::Error as DeError;
use serde_json::Error as SerdeError;
use std::path::Path;
use std::process::Command;

pub struct PodmanCli;

impl PodmanPort for PodmanCli {
    fn image_created(&self, image: &str) -> Result<DateTime<Local>, PodmanComposeMgrError> {
        crate::utils::podman_utils::image::get_podman_image_upstream_create_time(image)
            .map_err(|e| PodmanComposeMgrError::CommandExecution(Box::new(e)))
    }

    fn image_modified(&self, image: &str) -> Result<DateTime<Local>, PodmanComposeMgrError> {
        crate::utils::podman_utils::image::get_podman_ondisk_modify_time(image)
            .map_err(|e| PodmanComposeMgrError::CommandExecution(Box::new(e)))
    }

    fn file_exists_and_readable(&self, file: &Path) -> bool {
        crate::utils::podman_utils::terminal::file_exists_and_readable(file)
    }

    fn list_local_images(&self) -> Result<Vec<LocalImageSummary>, PodmanComposeMgrError> {
        let output = Command::new(crate::utils::podman_utils::resolve_podman_binary())
            .args(["image", "ls", "--format", "json"])
            .output()
            .map_err(|e| PodmanComposeMgrError::CommandExecution(Box::new(e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            return Err(PodmanComposeMgrError::CommandExecution(Box::new(
                std::io::Error::new(std::io::ErrorKind::Other, stderr),
            )));
        }

        let json = parse_json_output(&output.stdout)
            .map_err(|e| PodmanComposeMgrError::CommandExecution(Box::new(e)))?;
        let mapped = local_images_from_json(json)
            .map_err(|e| PodmanComposeMgrError::CommandExecution(Box::new(e)))?;
        Ok(mapped)
    }
}

/// Parse `podman image ls --format json` output, handling both JSON arrays and NDJSON.
pub(crate) fn parse_json_output(bytes: &[u8]) -> Result<serde_json::Value, SerdeError> {
    serde_json::from_slice(bytes).or_else(|_| {
        let mut entries = Vec::new();
        let iter = serde_json::Deserializer::from_slice(bytes).into_iter::<serde_json::Value>();
        for item in iter {
            match item {
                Ok(entry) => entries.push(entry),
                Err(_) => break,
            }
        }
        if entries.is_empty() {
            return Err(SerdeError::custom("no entries parsed from json output"));
        }
        Ok(serde_json::Value::Array(entries))
    })
}

/// Convert parsed JSON into the simplified LocalImageSummary collection the app uses.
pub(crate) fn local_images_from_json(
    json: serde_json::Value,
) -> Result<Vec<LocalImageSummary>, SerdeError> {
    let arr = json
        .as_array()
        .ok_or_else(|| SerdeError::custom("podman image ls json must be an array"))?;

    let mut images = Vec::new();
    for entry in arr {
        if let Some(obj) = entry.as_object() {
            let created = obj
                .get("CreatedAt")
                .and_then(|v| v.as_str())
                .and_then(|s| crate::utils::podman_utils::convert_str_to_date(s).ok());
            parse_refs(obj.get("RepoTags"), created, &mut images);
            parse_refs(obj.get("Names"), created, &mut images);
            parse_digest_refs(obj.get("RepoDigests"), created, &mut images);
            if let (Some(repo), Some(tag)) = (
                obj.get("Repository").and_then(|v| v.as_str()),
                obj.get("Tag").and_then(|v| v.as_str()),
            ) {
                if repo.starts_with("localhost/") {
                    images.push(LocalImageSummary {
                        repository: repo.to_string(),
                        tag: tag.to_string(),
                        created,
                    });
                }
            }
        }
    }

    Ok(images)
}

fn parse_refs(
    value: Option<&serde_json::Value>,
    created: Option<chrono::DateTime<chrono::Local>>,
    out: &mut Vec<LocalImageSummary>,
) {
    if let Some(arr) = value.and_then(|v| v.as_array()) {
        for tag_val in arr {
            if let Some(tag_str) = tag_val.as_str() {
                if let Some((repository, tag)) = split_repo_tag(tag_str) {
                    out.push(LocalImageSummary {
                        repository,
                        tag,
                        created,
                    });
                }
            }
        }
    }
}

fn parse_digest_refs(
    value: Option<&serde_json::Value>,
    created: Option<chrono::DateTime<chrono::Local>>,
    out: &mut Vec<LocalImageSummary>,
) {
    if let Some(arr) = value.and_then(|v| v.as_array()) {
        for tag_val in arr {
            if let Some(tag_str) = tag_val.as_str() {
                let without_digest = tag_str.split('@').next().unwrap_or("");
                if !without_digest.starts_with("localhost/") {
                    continue;
                }
                let (repository, tag) = without_digest
                    .rsplit_once(':')
                    .map(|(r, t)| (r.to_string(), t.to_string()))
                    .unwrap_or_else(|| (without_digest.to_string(), "latest".to_string()));
                out.push(LocalImageSummary {
                    repository,
                    tag,
                    created,
                });
            }
        }
    }
}

fn split_repo_tag(raw: &str) -> Option<(String, String)> {
    if raw.contains('@') {
        return None;
    }
    let (repo, tag) = raw.rsplit_once(':')?;
    if !repo.starts_with("localhost/") {
        return None;
    }
    Some((repo.to_string(), tag.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn parses_names_and_digests_for_localhost_images() {
        let bytes = fs::read("tests/test08/golden.json").expect("fixture");
        let json = parse_json_output(&bytes).expect("parse json");
        let images = local_images_from_json(json).expect("parse images");
        let names: Vec<String> = images
            .into_iter()
            .map(|i| format!("{}:{}", i.repository, i.tag))
            .collect();

        assert!(
            names.contains(&"localhost/djf/ffmpeg:latest".to_string()),
            "should include ffmpeg image from Names array"
        );
        assert!(
            names.contains(&"localhost/djf/helper_x:latest".to_string()),
            "should include helper_x image from RepoDigests"
        );
        assert!(
            names.contains(&"localhost/djf/ffmpeg_base:latest".to_string()),
            "should include ffmpeg_base image when Repository/Tag missing"
        );
        assert!(
            names.contains(&"localhost/djf/openssh:latest".to_string()),
            "should include openssh image from Names array"
        );
    }
}
