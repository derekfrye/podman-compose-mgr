use super::row_builders;
use super::state::{App, ItemRow, ViewMode};

impl App {
    pub fn rebuild_rows_for_view(&mut self) {
        match self.view_mode {
            ViewMode::ByContainer => {
                self.rows = row_builders::build_container_rows(self);
                self.selected = 0;
            }
            ViewMode::ByImage => {
                self.rows = self.build_rows_for_image_view();
                self.selected = 0;
            }
            ViewMode::ByFolderThenImage => {
                self.current_path.clear();
                self.rows = row_builders::build_folder_rows(self);
                self.selected = 0;
            }
            ViewMode::ByDockerfile => {
                self.rows = row_builders::build_dockerfile_rows(self);
                self.selected = 0;
            }
            ViewMode::ByMakefile => {
                self.rows = row_builders::build_makefile_rows(self);
                self.selected = 0;
            }
        }
    }

    #[must_use]
    pub fn build_rows_for_view_mode(&self, mode: ViewMode) -> Vec<ItemRow> {
        let mut clone = row_builders::clone_for_build(self);
        clone.view_mode = mode;
        match mode {
            ViewMode::ByContainer => row_builders::build_container_rows(&clone),
            ViewMode::ByImage => row_builders::build_image_rows(&clone),
            ViewMode::ByFolderThenImage => row_builders::build_folder_rows(&clone),
            ViewMode::ByDockerfile => row_builders::build_dockerfile_rows(&clone),
            ViewMode::ByMakefile => row_builders::build_makefile_rows(&clone),
        }
    }

    #[must_use]
    pub fn build_rows_for_container_view(&self) -> Vec<ItemRow> {
        row_builders::build_container_rows(self)
    }

    #[must_use]
    pub fn build_rows_for_folder_view(&self) -> Vec<ItemRow> {
        row_builders::build_folder_rows(self)
    }

    fn build_rows_for_image_view(&self) -> Vec<ItemRow> {
        row_builders::build_image_rows(self)
    }

    #[must_use]
    pub fn build_rows_for_dockerfile_view(&self) -> Vec<ItemRow> {
        row_builders::build_dockerfile_rows(self)
    }

    #[must_use]
    pub fn build_rows_for_makefile_view(&self) -> Vec<ItemRow> {
        row_builders::build_makefile_rows(self)
    }
}
