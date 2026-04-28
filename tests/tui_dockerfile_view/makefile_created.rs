use std::fs;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use crate::render_support::{render_app, tui_args};
use chrono::{Duration, Local};
use crossbeam_channel as xchan;
use podman_compose_mgr::domain::LocalImageSummary;
use podman_compose_mgr::errors::PodmanComposeMgrError;
use podman_compose_mgr::infra::discovery_adapter::FsDiscovery;
use podman_compose_mgr::ports::{DiscoveryPort, PodmanPort};
use podman_compose_mgr::tui::app::{self, App, Msg, Services, UiState, ViewMode};
use tempfile::tempdir;

struct FakePodmanWithCreated {
    expected_image: String,
    created_at: chrono::DateTime<Local>,
}

impl PodmanPort for FakePodmanWithCreated {
    fn image_created(&self, image: &str) -> Result<chrono::DateTime<Local>, PodmanComposeMgrError> {
        if image == self.expected_image {
            Ok(self.created_at)
        } else {
            Err(PodmanComposeMgrError::CommandExecution(Box::new(
                std::io::Error::other("image not found"),
            )))
        }
    }

    fn image_modified(
        &self,
        _image: &str,
    ) -> Result<chrono::DateTime<Local>, PodmanComposeMgrError> {
        Err(PodmanComposeMgrError::CommandExecution(Box::new(
            std::io::Error::other("not needed"),
        )))
    }

    fn file_exists_and_readable(&self, file: &std::path::Path) -> bool {
        file.is_file()
    }

    fn list_local_images(&self) -> Result<Vec<LocalImageSummary>, PodmanComposeMgrError> {
        Ok(Vec::new())
    }
}

#[test]
fn tui_render_makefile_expand_shows_created_time() {
    let tmp = tempdir().expect("temp dir");
    let root = tmp.path();
    let app_dir = root.join("golf");
    fs::create_dir_all(&app_dir).expect("create app dir");
    fs::write(app_dir.join("Makefile"), "clean:\n\t@echo clean\n").expect("write makefile");
    fs::write(
        app_dir.join("m-miniflare.container"),
        "[Container]\nImage=localhost/djf/m-golf-srvless:latest\n",
    )
    .expect("write container");

    let args = tui_args(root, Vec::new());
    let image = "localhost/djf/m-golf-srvless:latest";
    let discovery: Arc<dyn DiscoveryPort> = Arc::new(FsDiscovery);
    let podman: Arc<dyn PodmanPort> = Arc::new(FakePodmanWithCreated {
        expected_image: image.to_string(),
        created_at: Local::now() - Duration::hours(3),
    });
    let core = Arc::new(podman_compose_mgr::app::AppCore::new(
        discovery.clone(),
        podman,
    ));
    let scan = core
        .scan_images(
            args.path.clone(),
            args.include_path_patterns.clone(),
            args.exclude_path_patterns.clone(),
        )
        .expect("scan");

    let mut app = makefile_app(root, scan);
    let (tx, rx) = xchan::unbounded::<Msg>();
    let services = Services {
        core,
        root: args.path.clone(),
        include: args.include_path_patterns.clone(),
        exclude: args.exclude_path_patterns.clone(),
        tx,
        args: args.clone(),
        working_dir: std::env::current_dir().expect("cwd"),
    };

    app::update_with_services(&mut app, Msg::ExpandOrEnter, Some(&services));
    let details_msg = rx
        .recv_timeout(StdDuration::from_secs(1))
        .expect("details ready message");
    app::update_with_services(&mut app, details_msg, Some(&services));

    let rendered = render_app(&mut app, &args, 140, 20);
    assert!(
        rendered.contains("golf: clean"),
        "missing makefile row\n{rendered}"
    );
    assert!(
        rendered.contains("Image: localhost/djf/m-golf-srvless:latest"),
        "missing inferred image details\n{rendered}"
    );
    assert!(
        rendered.contains("Target: clean"),
        "missing target\n{rendered}"
    );
    assert!(
        rendered.contains("Created:"),
        "missing created time\n{rendered}"
    );
    assert!(
        rendered.contains("single neighbor file"),
        "missing inference note\n{rendered}"
    );
}

fn makefile_app(root: &std::path::Path, scan: podman_compose_mgr::domain::ScanResult) -> App {
    let mut app = App::new();
    app.state = UiState::Ready;
    app.view_mode = ViewMode::ByMakefile;
    app.all_items = scan.images;
    app.makefile_items = scan.makefiles;
    app.set_root_path(root.to_path_buf());
    app.rebuild_rows_for_view();
    app
}
