use crate::tui::app::state::RebuildJob;
use crate::tui::app::state::{App, ModalState, OutputStream, Services, UiState};
use chrono::Local;
use std::fs::File;
use std::io::Write;
use std::path::{Component, Path};

pub(super) fn handle_open_export_log(app: &mut App) {
    if !matches!(app.state, UiState::Rebuilding) {
        return;
    }
    let Some(rebuild) = app.rebuild.as_ref() else {
        return;
    };
    let Some(job) = rebuild.jobs.get(rebuild.active_idx) else {
        return;
    };

    let default_name = default_export_filename(job);
    app.modal = Some(ModalState::ExportLog {
        input: default_name,
        error: None,
    });
}

pub(super) fn handle_export_input(app: &mut App, ch: char) {
    if let Some(ModalState::ExportLog { input, error }) = app.modal.as_mut() {
        input.push(ch);
        *error = None;
    }
}

pub(super) fn handle_export_backspace(app: &mut App) {
    if let Some(ModalState::ExportLog { input, error }) = app.modal.as_mut() {
        input.pop();
        *error = None;
    }
}

pub(super) fn handle_export_cancel(app: &mut App) {
    if matches!(app.modal, Some(ModalState::ExportLog { .. })) {
        app.modal = None;
    }
}

pub(super) fn handle_export_submit(app: &mut App, services: Option<&Services>) {
    let Some(services) = services else {
        return;
    };
    let Some(rebuild) = app.rebuild.as_ref() else {
        return;
    };
    let Some(job) = rebuild.jobs.get(rebuild.active_idx) else {
        return;
    };
    let filename = match &app.modal {
        Some(ModalState::ExportLog { input, .. }) => input.clone(),
        _ => return,
    };
    let trimmed = filename.trim();
    if trimmed.is_empty() {
        set_export_error(app, Some("File name cannot be empty".to_string()));
        return;
    }

    let candidate = Path::new(trimmed);
    if candidate.is_absolute() {
        set_export_error(
            app,
            Some("Provide a relative filename (absolute paths are not allowed).".to_string()),
        );
        return;
    }

    if contains_forbidden_path_segments(candidate) {
        set_export_error(
            app,
            Some("File name cannot traverse parent directories.".to_string()),
        );
        return;
    }

    let mut destination = services.working_dir.clone();
    destination.push(candidate);

    let lines: Vec<String> = job.output.iter().map(|entry| entry.text.clone()).collect();
    let active_idx = rebuild.active_idx;

    match write_lines(&destination, &lines) {
        Ok(()) => {
            if let Some(rebuild_mut) = app.rebuild.as_mut()
                && let Some(job_mut) = rebuild_mut.jobs.get_mut(active_idx)
            {
                job_mut.push_output(
                    OutputStream::Stdout,
                    format!("Exported rebuild log to {}", destination.display()),
                    rebuild_mut.output_limit,
                );
                rebuild_mut.auto_scroll = true;
            }
            app.modal = None;
        }
        Err(err) => {
            set_export_error(app, Some(err));
        }
    }
}

fn set_export_error(app: &mut App, message: Option<String>) {
    if let Some(ModalState::ExportLog { error, .. }) = app.modal.as_mut() {
        *error = message;
    }
}

fn write_lines(path: &Path, lines: &[String]) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && let Err(err) = std::fs::create_dir_all(parent)
    {
        return Err(format!("Failed creating directory: {err}"));
    }

    let mut file = File::create(path).map_err(|err| format!("Could not create file: {err}"))?;
    for line in lines {
        writeln!(file, "{line}").map_err(|err| format!("Failed writing file: {err}"))?;
    }
    file.flush()
        .map_err(|err| format!("Failed to flush file: {err}"))
}

fn default_export_filename(job: &RebuildJob) -> String {
    let image = job.image.as_str();
    let (name_raw, tag_raw) = split_image_name_and_tag(image);
    let name = sanitize_filename_component(name_raw).unwrap_or_else(|| "image".to_string());
    let tag = sanitize_filename_component(tag_raw).unwrap_or_else(|| "tag".to_string());
    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
    format!("{name}-{tag}-{timestamp}.log")
}

fn split_image_name_and_tag(image: &str) -> (&str, &str) {
    if let Some(at_pos) = image.rfind('@') {
        let name = &image[..at_pos];
        let digest = &image[at_pos + 1..];
        return (name, digest);
    }

    let last_slash = image.rfind('/');
    if let Some(colon_pos) = image.rfind(':')
        && last_slash.is_none_or(|slash_pos| slash_pos < colon_pos)
    {
        let name = &image[..colon_pos];
        let tag = &image[colon_pos + 1..];
        return (name, tag);
    }

    (image, "latest")
}

fn sanitize_filename_component(input: &str) -> Option<String> {
    let filtered: String = input
        .chars()
        .filter(|ch| !matches!(ch, ':' | '/' | '\\' | '?' | '*' | '"' | '<' | '>' | ' '))
        .collect();
    let trimmed = filtered.trim_matches('.');
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn contains_forbidden_path_segments(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, Component::ParentDir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_image_with_tag() {
        let (name, tag) = split_image_name_and_tag("nginx:latest");
        assert_eq!(name, "nginx");
        assert_eq!(tag, "latest");
    }

    #[test]
    fn split_image_with_registry_port() {
        let (name, tag) = split_image_name_and_tag("registry.local:5000/foo/bar:dev");
        assert_eq!(name, "registry.local:5000/foo/bar");
        assert_eq!(tag, "dev");
    }

    #[test]
    fn split_image_with_digest() {
        let (name, tag) = split_image_name_and_tag("registry.example.com/foo@sha256:abcdef123456");
        assert_eq!(name, "registry.example.com/foo");
        assert_eq!(tag, "sha256:abcdef123456");
    }

    #[test]
    fn sanitize_component_removes_chars() {
        let sanitized = sanitize_filename_component("foo/bar:tag name").unwrap();
        assert_eq!(sanitized, "foobartagname");
    }

    #[test]
    fn sanitize_component_returns_none_when_empty() {
        assert!(sanitize_filename_component("   ").is_none());
        assert!(sanitize_filename_component("::").is_none());
    }

    #[test]
    fn contains_forbidden_segments_detects_parent_dir() {
        assert!(contains_forbidden_path_segments(Path::new("../foo")));
        assert!(!contains_forbidden_path_segments(Path::new("foo/bar.log")));
    }
}
