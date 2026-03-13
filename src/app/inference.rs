use super::AppCore;
use crate::domain::{
    DiscoveryResult, DockerfileInference, InferenceSource, LocalImageSummary, MakefileInference,
};

impl AppCore {
    pub(super) fn infer_dockerfiles(
        discovery: &DiscoveryResult,
        local_images: &[LocalImageSummary],
    ) -> Vec<DockerfileInference> {
        let mut inferred = Vec::new();
        for dockerfile in &discovery.dockerfiles {
            let inference = infer_dockerfile(dockerfile, local_images);
            inferred.push(inference);
        }
        inferred
    }

    pub(super) fn infer_makefiles(
        discovery: &DiscoveryResult,
        local_images: &[LocalImageSummary],
    ) -> Vec<MakefileInference> {
        let mut inferred = Vec::new();
        for makefile in &discovery.makefiles {
            let inference = infer_makefile(makefile, local_images);
            inferred.push(inference);
        }
        inferred
    }

    pub(super) fn find_created_for(
        image: &str,
        local_images: &[LocalImageSummary],
    ) -> Option<String> {
        match_localhost_image_exact(image, local_images).and_then(|entry| {
            entry
                .created
                .map(crate::utils::podman_utils::format_time_ago)
        })
    }
}

fn infer_dockerfile(
    dockerfile: &crate::domain::DiscoveredDockerfile,
    local_images: &[LocalImageSummary],
) -> DockerfileInference {
    let (inference_source, inferred_image, created_time_ago, note) =
        if let Some((source, image)) = dockerfile.neighbor_image.clone() {
            (
                source,
                Some(image.clone()),
                AppCore::find_created_for(&image, local_images),
                Some("single neighbor file".to_string()),
            )
        } else {
            infer_dockerfile_from_registry(dockerfile, local_images)
        };

    DockerfileInference {
        dockerfile_path: dockerfile.dockerfile_path.clone(),
        source_dir: dockerfile.source_dir.clone(),
        basename: dockerfile.basename.clone(),
        quadlet_basename: dockerfile.quadlet_basename.clone(),
        inferred_image,
        inference_source,
        created_time_ago,
        total_dockerfiles_in_dir: dockerfile.total_dockerfiles_in_dir,
        neighbor_file_count: dockerfile.neighbor_file_count,
        note,
    }
}

fn infer_dockerfile_from_registry(
    dockerfile: &crate::domain::DiscoveredDockerfile,
    local_images: &[LocalImageSummary],
) -> (
    InferenceSource,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let suffix = dockerfile
        .basename
        .strip_prefix("Dockerfile")
        .unwrap_or(&dockerfile.basename)
        .trim_start_matches('.');
    let Some(entry) = (!suffix.is_empty())
        .then(|| match_localhost_image(suffix, local_images))
        .flatten()
    else {
        return (InferenceSource::Unknown, None, None, None);
    };

    let note = if dockerfile.total_dockerfiles_in_dir > 1 {
        Some("registry matched (more than one Dockerfile in the dir)".to_string())
    } else {
        Some("registry matched".to_string())
    };

    (
        InferenceSource::LocalhostRegistry,
        Some(format!("{}:{}", entry.repository, entry.tag)),
        entry
            .created
            .map(crate::utils::podman_utils::format_time_ago),
        note,
    )
}

fn infer_makefile(
    makefile: &crate::domain::DiscoveredMakefile,
    local_images: &[LocalImageSummary],
) -> MakefileInference {
    let (inference_source, inferred_image, inferred_images, created_time_ago, note) =
        if let Some((source, image)) = makefile.neighbor_image.clone() {
            (
                source,
                Some(image.clone()),
                vec![image.clone()],
                AppCore::find_created_for(&image, local_images),
                Some("single neighbor file".to_string()),
            )
        } else if !makefile.target_images.is_empty() {
            let inferred_image = makefile.target_images.first().cloned();
            let created_time_ago = inferred_image
                .as_ref()
                .and_then(|image| AppCore::find_created_for(image, local_images));
            (
                InferenceSource::Unknown,
                inferred_image,
                makefile.target_images.clone(),
                created_time_ago,
                None,
            )
        } else {
            (InferenceSource::Unknown, None, Vec::new(), None, None)
        };

    MakefileInference {
        makefile_path: makefile.makefile_path.clone(),
        source_dir: makefile.source_dir.clone(),
        basename: makefile.basename.clone(),
        make_target: makefile.make_target.clone(),
        inferred_images,
        quadlet_basename: makefile.quadlet_basename.clone(),
        inferred_image,
        inference_source,
        created_time_ago,
        total_makefiles_in_dir: makefile.total_makefiles_in_dir,
        neighbor_file_count: makefile.neighbor_file_count,
        note,
    }
}

fn match_localhost_image<'a>(
    suffix: &str,
    local_images: &'a [LocalImageSummary],
) -> Option<&'a LocalImageSummary> {
    let mut candidates: Vec<&LocalImageSummary> = local_images
        .iter()
        .filter(|img| {
            img.repository.starts_with("localhost")
                && (img.repository.ends_with(&format!("/{suffix}"))
                    || img.repository.split('/').next_back() == Some(suffix))
        })
        .collect();
    candidates.sort_by(|a, b| b.created.cmp(&a.created));
    candidates.into_iter().next()
}

fn match_localhost_image_exact<'a>(
    name: &str,
    local_images: &'a [LocalImageSummary],
) -> Option<&'a LocalImageSummary> {
    local_images
        .iter()
        .find(|img| format!("{}:{}", img.repository, img.tag) == name)
}
