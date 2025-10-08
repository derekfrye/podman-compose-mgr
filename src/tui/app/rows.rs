use super::state::{App, ItemRow, ViewMode};
use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;

impl App {
    pub fn rebuild_rows_for_view(&mut self) {
        match self.view_mode {
            ViewMode::ByContainer => {
                self.rows = self.build_rows_for_container_view();
                self.selected = 0;
            }
            ViewMode::ByImage => {
                self.rows = self.build_rows_for_image_view();
                self.selected = 0;
            }
            ViewMode::ByFolderThenImage => {
                self.current_path.clear();
                self.rows = self.build_rows_for_folder_view();
                self.selected = 0;
            }
        }
    }

    pub fn build_rows_for_view_mode(&self, mode: ViewMode) -> Vec<ItemRow> {
        let mut clone = self.clone_for_build();
        clone.view_mode = mode;
        match mode {
            ViewMode::ByContainer => clone.build_rows_for_container_view(),
            ViewMode::ByImage => clone.build_rows_for_image_view(),
            ViewMode::ByFolderThenImage => clone.build_rows_for_folder_view(),
        }
    }

    pub fn build_rows_for_container_view(&self) -> Vec<ItemRow> {
        self.all_items
            .iter()
            .map(|d| ItemRow {
                checked: false,
                image: d.image.clone(),
                container: d.container.clone(),
                source_dir: d.source_dir.clone(),
                expanded: false,
                details: Vec::new(),
                is_dir: false,
                dir_name: None,
            })
            .collect()
    }

    pub fn build_rows_for_folder_view(&self) -> Vec<ItemRow> {
        let mut subdirs: BTreeSet<String> = BTreeSet::new();
        let mut images: BTreeSet<String> = BTreeSet::new();
        for discovered in &self.all_items {
            if let Ok(relative) = discovered.source_dir.strip_prefix(&self.root_path) {
                let components: Vec<String> = relative
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().to_string())
                    .collect();
                if !self.path_matches(&components) {
                    continue;
                }
                let remainder = &components[self.current_path.len()..];
                if remainder.is_empty() {
                    images.insert(discovered.image.clone());
                } else {
                    subdirs.insert(remainder[0].clone());
                }
            }
        }
        self.build_rows_from_sets(subdirs, images)
    }

    fn build_rows_for_image_view(&self) -> Vec<ItemRow> {
        let mut seen: HashSet<String> = HashSet::new();
        let mut rows: Vec<ItemRow> = Vec::new();
        for discovered in &self.all_items {
            if seen.insert(discovered.image.clone()) {
                rows.push(ItemRow {
                    checked: false,
                    image: discovered.image.clone(),
                    container: None,
                    source_dir: discovered.source_dir.clone(),
                    expanded: false,
                    details: Vec::new(),
                    is_dir: false,
                    dir_name: None,
                });
            }
        }
        rows
    }

    fn path_matches(&self, components: &[String]) -> bool {
        components.len() >= self.current_path.len()
            && components
                .iter()
                .take(self.current_path.len())
                .eq(self.current_path.iter())
    }

    fn build_rows_from_sets(
        &self,
        subdirs: BTreeSet<String>,
        images: BTreeSet<String>,
    ) -> Vec<ItemRow> {
        let mut rows: Vec<ItemRow> = Vec::new();
        let current_root = self.current_root();
        for dir in subdirs {
            rows.push(ItemRow {
                checked: false,
                image: String::new(),
                container: None,
                source_dir: current_root.join(&dir),
                expanded: false,
                details: Vec::new(),
                is_dir: true,
                dir_name: Some(dir),
            });
        }
        for image in images {
            rows.push(ItemRow {
                checked: false,
                image,
                container: None,
                source_dir: current_root.clone(),
                expanded: false,
                details: Vec::new(),
                is_dir: false,
                dir_name: None,
            });
        }
        rows
    }

    fn current_root(&self) -> PathBuf {
        self.root_path
            .join(self.current_path.iter().collect::<PathBuf>())
    }

    fn clone_for_build(&self) -> App {
        App {
            title: self.title.clone(),
            should_quit: self.should_quit,
            state: self.state,
            rows: Vec::new(),
            selected: 0,
            spinner_idx: 0,
            view_mode: self.view_mode,
            modal: None,
            all_items: self.all_items.clone(),
            root_path: self.root_path.clone(),
            current_path: self.current_path.clone(),
        }
    }
}
