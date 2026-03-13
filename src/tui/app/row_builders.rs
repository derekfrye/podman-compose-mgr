use super::state::{App, ItemRow};
use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;

pub fn build_container_rows(app: &App) -> Vec<ItemRow> {
    app.all_items
        .iter()
        .map(|d| ItemRow {
            checked: false,
            image: d.image.clone(),
            container: d.container.clone(),
            source_dir: d.source_dir.clone(),
            entry_path: Some(d.entry_path.clone()),
            expanded: false,
            details: Vec::new(),
            is_dir: false,
            dir_name: None,
            dockerfile_extra: None,
            makefile_extra: None,
        })
        .collect()
}

pub fn build_folder_rows(app: &App) -> Vec<ItemRow> {
    let mut subdirs: BTreeSet<String> = BTreeSet::new();
    let mut images: BTreeSet<String> = BTreeSet::new();
    for discovered in &app.all_items {
        if let Ok(relative) = discovered.source_dir.strip_prefix(&app.root_path) {
            let components: Vec<String> = relative
                .components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .collect();
            if !path_matches(app, &components) {
                continue;
            }
            let remainder = &components[app.current_path.len()..];
            if remainder.is_empty() {
                images.insert(discovered.image.clone());
            } else {
                subdirs.insert(remainder[0].clone());
            }
        }
    }
    build_rows_from_sets(app, subdirs, images)
}

pub fn build_image_rows(app: &App) -> Vec<ItemRow> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut rows = Vec::new();
    for discovered in &app.all_items {
        if seen.insert(discovered.image.clone()) {
            rows.push(ItemRow {
                checked: false,
                image: discovered.image.clone(),
                container: None,
                source_dir: discovered.source_dir.clone(),
                entry_path: Some(discovered.entry_path.clone()),
                expanded: false,
                details: Vec::new(),
                is_dir: false,
                dir_name: None,
                dockerfile_extra: None,
                makefile_extra: None,
            });
        }
    }
    rows
}

pub fn build_dockerfile_rows(app: &App) -> Vec<ItemRow> {
    app.dockerfile_items
        .iter()
        .map(|df| ItemRow {
            checked: false,
            image: df
                .inferred_image
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            container: None,
            source_dir: df.source_dir.clone(),
            entry_path: Some(df.dockerfile_path.clone()),
            expanded: false,
            details: Vec::new(),
            is_dir: false,
            dir_name: None,
            dockerfile_extra: Some(super::state::DockerfileRowExtra {
                source: df.inference_source.clone(),
                dockerfile_name: df.basename.clone(),
                quadlet_basename: df.quadlet_basename.clone(),
                image_name: df.inferred_image.clone(),
                created_time_ago: df.created_time_ago.clone(),
                note: df.note.clone(),
            }),
            makefile_extra: None,
        })
        .collect()
}

pub fn build_makefile_rows(app: &App) -> Vec<ItemRow> {
    app.makefile_items
        .iter()
        .map(|mf| ItemRow {
            checked: false,
            image: mf
                .inferred_image
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            container: None,
            source_dir: mf.source_dir.clone(),
            entry_path: Some(mf.makefile_path.clone()),
            expanded: false,
            details: Vec::new(),
            is_dir: false,
            dir_name: None,
            dockerfile_extra: None,
            makefile_extra: Some(super::state::MakefileRowExtra {
                source: mf.inference_source.clone(),
                makefile_name: mf.basename.clone(),
                make_target: mf.make_target.clone(),
                quadlet_basename: mf.quadlet_basename.clone(),
                image_name: mf.inferred_image.clone(),
                image_names: mf.inferred_images.clone(),
                created_time_ago: mf.created_time_ago.clone(),
                note: mf.note.clone(),
            }),
        })
        .collect()
}

pub fn current_root(app: &App) -> PathBuf {
    app.root_path
        .join(app.current_path.iter().collect::<PathBuf>())
}

pub fn clone_for_build(app: &App) -> App {
    App {
        title: app.title.clone(),
        should_quit: app.should_quit,
        state: app.state,
        rows: Vec::new(),
        selected: 0,
        spinner_idx: 0,
        view_mode: app.view_mode,
        modal: None,
        all_items: app.all_items.clone(),
        dockerfile_items: app.dockerfile_items.clone(),
        makefile_items: app.makefile_items.clone(),
        root_path: app.root_path.clone(),
        current_path: app.current_path.clone(),
        rebuild: None,
        auto_rebuild_all: app.auto_rebuild_all,
        auto_rebuild_triggered: app.auto_rebuild_triggered,
    }
}

fn path_matches(app: &App, components: &[String]) -> bool {
    components.len() >= app.current_path.len()
        && components
            .iter()
            .take(app.current_path.len())
            .eq(app.current_path.iter())
}

fn build_rows_from_sets(
    app: &App,
    subdirs: BTreeSet<String>,
    images: BTreeSet<String>,
) -> Vec<ItemRow> {
    let mut rows = Vec::new();
    let current_root = current_root(app);
    for dir in subdirs {
        rows.push(ItemRow {
            checked: false,
            image: String::new(),
            container: None,
            source_dir: current_root.join(&dir),
            entry_path: None,
            expanded: false,
            details: Vec::new(),
            is_dir: true,
            dir_name: Some(dir),
            dockerfile_extra: None,
            makefile_extra: None,
        });
    }
    for image in images {
        let entry_path = app
            .all_items
            .iter()
            .find(|item| item.image == image)
            .map(|item| item.entry_path.clone());
        rows.push(ItemRow {
            checked: false,
            image,
            container: None,
            source_dir: current_root.clone(),
            entry_path,
            expanded: false,
            details: Vec::new(),
            is_dir: false,
            dir_name: None,
            dockerfile_extra: None,
            makefile_extra: None,
        });
    }
    rows
}
