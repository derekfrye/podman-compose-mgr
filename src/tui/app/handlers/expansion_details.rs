use crate::app::AppCore;
use crate::tui::app::state::{DockerfileRowExtra, MakefileRowExtra, ViewMode};

pub(super) fn compute_details_for(
    core: &AppCore,
    image: &str,
    source_dir: &std::path::Path,
    entry_path: Option<&std::path::Path>,
    view_mode: ViewMode,
    dockerfile_extra: Option<&DockerfileRowExtra>,
    makefile_extra: Option<&MakefileRowExtra>,
) -> Vec<String> {
    if view_mode == ViewMode::ByDockerfile {
        return dockerfile_details(dockerfile_extra);
    }
    if view_mode == ViewMode::ByMakefile {
        return makefile_details(core, image, source_dir, entry_path, makefile_extra);
    }
    generic_image_details(core, image, source_dir, entry_path, view_mode)
}

fn dockerfile_details(dockerfile_extra: Option<&DockerfileRowExtra>) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(extra) = dockerfile_extra {
        push_inference_source_line(&mut lines, &extra.source, extra.quadlet_basename.as_deref());
        let image_name = extra
            .image_name
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        lines.push(format!("Image: {image_name}"));
        if let Some(created) = &extra.created_time_ago {
            lines.push(format!("Created: {created}"));
        }
        if let Some(note) = &extra.note {
            lines.push(note.clone());
        }
    }
    lines
}

fn makefile_details(
    core: &AppCore,
    image: &str,
    source_dir: &std::path::Path,
    entry_path: Option<&std::path::Path>,
    makefile_extra: Option<&MakefileRowExtra>,
) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(extra) = makefile_extra {
        push_inference_source_line(&mut lines, &extra.source, extra.quadlet_basename.as_deref());
        if let Some(target) = &extra.make_target {
            lines.push(format!("Target: {target}"));
        } else {
            lines.push("Target: (default)".to_string());
        }
        let images = makefile_detail_images(extra, image);
        if images.is_empty() {
            lines.push("Image: unknown".to_string());
        } else {
            for image_name in images {
                lines.push(format!("Image: {image_name}"));
                if let Some(created) =
                    resolve_makefile_created_time(core, &image_name, source_dir, entry_path, extra)
                {
                    lines.push(format!("Created: {created}"));
                }
            }
        }
        if let Some(note) = &extra.note {
            lines.push(note.clone());
        }
    }
    lines
}

fn generic_image_details(
    core: &AppCore,
    image: &str,
    source_dir: &std::path::Path,
    entry_path: Option<&std::path::Path>,
    view_mode: ViewMode,
) -> Vec<String> {
    use crate::domain::ImageDetails;

    let mut lines = Vec::new();
    if matches!(
        view_mode,
        ViewMode::ByContainer | ViewMode::ByFolderThenImage
    ) {
        lines.push(format!("Compose dir: {}", source_dir.display()));
    }
    match core.image_details(image, source_dir, entry_path) {
        Ok(ImageDetails {
            created_time_ago,
            pulled_time_ago,
            dockerfile_name,
            has_makefile,
        }) => {
            if let Some(created) = created_time_ago {
                lines.push(format!("Created: {created}"));
            }
            if let Some(pulled) = pulled_time_ago {
                lines.push(format!("Pulled: {pulled}"));
            }
            match dockerfile_name {
                Some(name) => lines.push(format!("Dockerfile: {name}")),
                None => lines.push("Dockerfile: not found".to_string()),
            }
            if has_makefile {
                lines.push("Found Makefile".to_string());
            }
        }
        Err(err) => lines.push(format!("error: {err}")),
    }
    lines
}

fn push_inference_source_line(
    lines: &mut Vec<String>,
    source: &crate::domain::InferenceSource,
    quadlet_basename: Option<&str>,
) {
    use crate::domain::InferenceSource;

    match source {
        InferenceSource::Quadlet => {
            if let Some(name) = quadlet_basename {
                lines.push(format!("Inferred from quadlet: {name}"));
            } else {
                lines.push("Inferred from quadlet".to_string());
            }
        }
        InferenceSource::Compose => lines.push("Inferred from compose".to_string()),
        InferenceSource::LocalhostRegistry => {
            lines.push("Inferred from localhost registry".to_string());
        }
        InferenceSource::Unknown => lines.push("Inferred from unknown source".to_string()),
    }
}

fn makefile_detail_images(extra: &MakefileRowExtra, image: &str) -> Vec<String> {
    if !extra.image_names.is_empty() {
        return extra.image_names.clone();
    }

    extra
        .image_name
        .clone()
        .filter(|name| !name.is_empty() && name != "unknown")
        .or_else(|| {
            if image.is_empty() || image == "unknown" {
                None
            } else {
                Some(image.to_string())
            }
        })
        .into_iter()
        .collect()
}

fn resolve_makefile_created_time(
    core: &AppCore,
    image: &str,
    source_dir: &std::path::Path,
    entry_path: Option<&std::path::Path>,
    extra: &MakefileRowExtra,
) -> Option<String> {
    if let Some(created) = &extra.created_time_ago {
        let uses_single_image = extra.image_names.len() <= 1;
        let matches_primary = extra.image_name.as_deref() == Some(image);
        if uses_single_image && matches_primary {
            return Some(created.clone());
        }
    }

    let inferred_image = if image.is_empty() || image == "unknown" {
        extra.image_name.as_deref().unwrap_or(image)
    } else {
        image
    };
    if inferred_image.is_empty() || inferred_image == "unknown" {
        return None;
    }

    core.image_details(inferred_image, source_dir, entry_path)
        .ok()
        .and_then(|details| details.created_time_ago)
}
