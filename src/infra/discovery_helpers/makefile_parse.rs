use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::Path;

pub fn makefile_parent_label(makefile_path: &Path) -> String {
    makefile_path
        .parent()
        .and_then(|parent| parent.file_name())
        .map(|name| name.to_string_lossy().to_string())
        .or_else(|| {
            makefile_path
                .parent()
                .map(|parent| parent.display().to_string())
        })
        .unwrap_or_else(|| "Makefile".to_string())
}

pub fn parse_makefile_targets(path: &Path) -> Vec<String> {
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut targets = BTreeSet::new();
    for line in content.lines() {
        if line.trim().is_empty() || line.starts_with('\t') || line.starts_with(' ') {
            continue;
        }
        let line = line.split('#').next().unwrap_or("").trim_end();
        if line.is_empty() || line.contains(":=") || line.contains("::=") {
            continue;
        }
        let Some((left, _)) = line.split_once(':') else {
            continue;
        };
        for target in left.split_whitespace() {
            if target.is_empty()
                || target.starts_with('.')
                || target.contains('%')
                || target.contains('$')
            {
                continue;
            }
            targets.insert(target.to_string());
        }
    }
    targets.into_iter().collect()
}

pub fn parse_makefile_target_images(path: &Path) -> HashMap<String, Vec<String>> {
    let Ok(content) = fs::read_to_string(path) else {
        return HashMap::new();
    };

    let logical_lines = join_makefile_lines(&content);
    let mut target_images = HashMap::new();
    let mut current_targets: Vec<String> = Vec::new();

    for line in logical_lines {
        let raw = line.as_str();
        let trimmed = raw.trim_end();
        if trimmed.trim().is_empty() {
            continue;
        }

        if raw.starts_with('\t') || raw.starts_with("    ") {
            push_recipe_images(&mut target_images, &current_targets, trimmed);
        } else {
            current_targets = parse_declared_targets(trimmed);
        }
    }

    target_images
}

fn join_makefile_lines(content: &str) -> Vec<String> {
    let mut logical = Vec::new();
    let mut current = String::new();

    for line in content.lines() {
        let continued = line.trim_end().ends_with('\\');
        let segment = line
            .trim_end()
            .strip_suffix('\\')
            .unwrap_or(line.trim_end());
        if current.is_empty() {
            current.push_str(segment);
        } else {
            current.push(' ');
            current.push_str(segment.trim_start());
        }
        if !continued {
            logical.push(std::mem::take(&mut current));
        }
    }

    if !current.is_empty() {
        logical.push(current);
    }
    logical
}

fn parse_declared_targets(line: &str) -> Vec<String> {
    let line = line.split('#').next().unwrap_or("").trim_end();
    if line.is_empty() || line.contains(":=") || line.contains("::=") {
        return Vec::new();
    }
    let Some((left, _)) = line.split_once(':') else {
        return Vec::new();
    };
    left.split_whitespace()
        .filter(|target| {
            !target.is_empty()
                && !target.starts_with('.')
                && !target.contains('%')
                && !target.contains('$')
        })
        .map(std::string::ToString::to_string)
        .collect()
}

fn push_recipe_images(
    target_images: &mut HashMap<String, Vec<String>>,
    current_targets: &[String],
    trimmed: &str,
) {
    if current_targets.is_empty() {
        return;
    }
    let images = parse_build_tags_from_recipe(trimmed);
    if images.is_empty() {
        return;
    }
    for target in current_targets {
        let entry = target_images.entry(target.clone()).or_default();
        for image in &images {
            if !entry.contains(image) {
                entry.push(image.clone());
            }
        }
    }
}

fn parse_build_tags_from_recipe(line: &str) -> Vec<String> {
    let cleaned = line.trim();
    if !(cleaned.contains("podman build") || cleaned.contains("docker build")) {
        return Vec::new();
    }

    let tokens: Vec<&str> = cleaned.split_whitespace().collect();
    let mut images = Vec::new();
    let mut idx = 0;
    while idx < tokens.len() {
        match tokens[idx] {
            "-t" | "--tag" => {
                if let Some(image) = tokens.get(idx + 1)
                    && !images.iter().any(|seen| seen == image)
                {
                    images.push((*image).to_string());
                }
                idx += 2;
            }
            token if token.starts_with("--tag=") => {
                let image = token.trim_start_matches("--tag=");
                if !image.is_empty() && !images.iter().any(|seen| seen == image) {
                    images.push(image.to_string());
                }
                idx += 1;
            }
            _ => idx += 1,
        }
    }
    images
}

#[cfg(test)]
mod tests {
    use super::parse_makefile_target_images;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn parse_makefile_target_images_extracts_tags_per_target() {
        let tmp = tempdir().expect("temp dir");
        let makefile = tmp.path().join("Makefile");
        fs::write(
            &makefile,
            ".PHONY: all clean helper_x\n\nall:\n\tpodman build -f Dockerfile.m-openssh -t djf/m-openssh\n\tpodman build -f Dockerfile.m-ffmpeg_base -t djf/m-ffmpeg_base\n\tpodman build -f Dockerfile.m-ffmpeg -t djf/m-ffmpeg\n\tpodman build -t djf/m-helper_x \\\n\t\t--build-arg USERNAME=$(shell id -un) \\\n\t\t-f Dockerfile.m-helper_x\n\nhelper_x:\n\tpodman build -t djf/m-helper_x -f Dockerfile.m-helper_x\n\nffmpeg:\n\tpodman build -f Dockerfile.m-ffmpeg_base -t djf/m-ffmpeg_base\n\tpodman build -f Dockerfile.m-ffmpeg -t djf/m-ffmpeg\n\nssh:\n\tdocker build --tag=djf/m-openssh -f Dockerfile.m-openssh .\n\nclean:\n\t@:\n",
        )
        .expect("write makefile");

        let parsed = parse_makefile_target_images(&makefile);
        assert_eq!(
            parsed.get("all"),
            Some(&vec![
                "djf/m-openssh".to_string(),
                "djf/m-ffmpeg_base".to_string(),
                "djf/m-ffmpeg".to_string(),
                "djf/m-helper_x".to_string(),
            ])
        );
        assert_eq!(
            parsed.get("helper_x"),
            Some(&vec!["djf/m-helper_x".to_string()])
        );
        assert_eq!(
            parsed.get("ffmpeg"),
            Some(&vec![
                "djf/m-ffmpeg_base".to_string(),
                "djf/m-ffmpeg".to_string()
            ])
        );
        assert_eq!(parsed.get("ssh"), Some(&vec!["djf/m-openssh".to_string()]));
        assert_eq!(parsed.get("clean"), None);
    }
}
