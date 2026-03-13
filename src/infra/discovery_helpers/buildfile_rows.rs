use crate::domain::{DiscoveredDockerfile, DiscoveredMakefile, InferenceSource};
use crate::image_build::container_file::parse_container_file;
use crate::infra::discovery_types::DirInfo;
use std::collections::HashMap;
use std::hash::BuildHasher;

use super::makefile_parse::{
    makefile_parent_label, parse_makefile_target_images, parse_makefile_targets,
};

#[must_use]
pub fn build_dockerfile_rows<S: BuildHasher>(
    dir_info: &HashMap<std::path::PathBuf, DirInfo, S>,
) -> Vec<DiscoveredDockerfile> {
    let mut dockerfiles = Vec::new();
    for (dir, info) in dir_info {
        if info.dockerfiles.is_empty() {
            continue;
        }

        let neighbor_count = info.compose_files.len() + info.container_files.len();
        for dockerfile_path in &info.dockerfiles {
            let (neighbor_image, quadlet_basename) =
                neighbor_image_and_quadlet(info, neighbor_count);
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

#[must_use]
pub fn build_makefile_rows<S: BuildHasher>(
    dir_info: &HashMap<std::path::PathBuf, DirInfo, S>,
) -> Vec<DiscoveredMakefile> {
    let mut makefiles = Vec::new();
    for (dir, info) in dir_info {
        if info.makefiles.is_empty() {
            continue;
        }

        let neighbor_count = info.compose_files.len() + info.container_files.len();
        for makefile_path in &info.makefiles {
            let (neighbor_image, quadlet_basename) =
                neighbor_image_and_quadlet(info, neighbor_count);
            let target_images_by_name = parse_makefile_target_images(makefile_path);
            let ctx = MakefileRowContext {
                dir,
                total_makefiles_in_dir: info.makefiles.len(),
                neighbor_count,
                makefile_path,
                parent_label: makefile_parent_label(makefile_path),
                targets: parse_makefile_targets(makefile_path),
                target_images_by_name,
                quadlet_basename,
                neighbor_image,
            };
            push_makefile_rows(&mut makefiles, &ctx);
        }
    }
    makefiles.sort_by(|a, b| {
        a.basename
            .cmp(&b.basename)
            .then_with(|| a.makefile_path.cmp(&b.makefile_path))
    });
    makefiles
}

fn neighbor_image_and_quadlet(
    info: &DirInfo,
    neighbor_count: usize,
) -> (Option<(InferenceSource, String)>, Option<String>) {
    if !(info.makefiles.len() == 1 || info.dockerfiles.len() == 1) || neighbor_count != 1 {
        return (None, None);
    }

    if info.container_files.len() == 1
        && let Ok(parsed) = parse_container_file(&info.container_files[0].path)
    {
        return (
            Some((InferenceSource::Quadlet, parsed.image.clone())),
            info.container_files[0]
                .path
                .file_name()
                .map(|name| name.to_string_lossy().to_string()),
        );
    }

    if info.compose_files.len() == 1
        && let Some(image) = info.compose_files[0].first_image.clone()
    {
        return (Some((InferenceSource::Compose, image)), None);
    }

    (None, None)
}

struct MakefileRowContext<'a> {
    dir: &'a std::path::Path,
    total_makefiles_in_dir: usize,
    neighbor_count: usize,
    makefile_path: &'a std::path::Path,
    parent_label: String,
    targets: Vec<String>,
    target_images_by_name: HashMap<String, Vec<String>>,
    quadlet_basename: Option<String>,
    neighbor_image: Option<(InferenceSource, String)>,
}

fn push_makefile_rows(out: &mut Vec<DiscoveredMakefile>, ctx: &MakefileRowContext<'_>) {
    if ctx.targets.is_empty() {
        out.push(DiscoveredMakefile {
            makefile_path: ctx.makefile_path.to_path_buf(),
            source_dir: ctx.dir.to_path_buf(),
            basename: format!("{}: (default)", ctx.parent_label),
            make_target: None,
            target_images: Vec::new(),
            quadlet_basename: ctx.quadlet_basename.clone(),
            neighbor_image: ctx.neighbor_image.clone(),
            total_makefiles_in_dir: ctx.total_makefiles_in_dir,
            neighbor_file_count: ctx.neighbor_count,
        });
        return;
    }

    for target in &ctx.targets {
        out.push(DiscoveredMakefile {
            makefile_path: ctx.makefile_path.to_path_buf(),
            source_dir: ctx.dir.to_path_buf(),
            basename: format!("{}: {target}", ctx.parent_label),
            make_target: Some(target.clone()),
            target_images: ctx
                .target_images_by_name
                .get(target)
                .cloned()
                .unwrap_or_default(),
            quadlet_basename: ctx.quadlet_basename.clone(),
            neighbor_image: ctx.neighbor_image.clone(),
            total_makefiles_in_dir: ctx.total_makefiles_in_dir,
            neighbor_file_count: ctx.neighbor_count,
        });
    }
}
