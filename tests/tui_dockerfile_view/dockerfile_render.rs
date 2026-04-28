use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use crate::render_support::{GOLDEN_JSON, render_app, tui_args};
use podman_compose_mgr::infra::discovery_adapter::FsDiscovery;
use podman_compose_mgr::ports::DiscoveryPort;
use podman_compose_mgr::tui::app::{App, UiState, ViewMode};
use tempfile::tempdir;

#[test]
fn tui_render_shows_inferred_images_from_json() {
    let tmp = tempdir().expect("temp dir");
    let root = tmp.path();
    let help_dir = root.join("help");
    fs::create_dir_all(&help_dir).expect("create help dir");
    for name in [
        "Dockerfile.ffmpeg",
        "Dockerfile.ffmpeg_base",
        "Dockerfile.helper_x",
        "Dockerfile.openssh",
    ] {
        fs::write(help_dir.join(name), "FROM scratch\n").expect("write dockerfile");
    }

    let include = format!("^{}", regex::escape(help_dir.to_str().expect("utf8 path")));
    let args = tui_args(root, vec![format!("{include}.*")]);
    let discovery: Arc<dyn DiscoveryPort> = Arc::new(FsDiscovery);
    let podman = podman_compose_mgr::tui::podman_from_json(PathBuf::from(GOLDEN_JSON).as_path())
        .expect("json podman");
    let core = podman_compose_mgr::app::AppCore::new(discovery, podman);
    let scan = core
        .scan_images(
            args.path.clone(),
            args.include_path_patterns.clone(),
            args.exclude_path_patterns.clone(),
        )
        .expect("scan");

    let mut app = App::new();
    app.state = UiState::Ready;
    app.view_mode = ViewMode::ByDockerfile;
    app.all_items = scan.images;
    app.dockerfile_items = scan.dockerfiles;
    app.rebuild_rows_for_view();

    let rendered = render_app(&mut app, &args, 120, 20);
    assert_inferred_image(&rendered, "Dockerfile.ffmpeg", "djf/ffmpeg:latest");
    assert_inferred_image(
        &rendered,
        "Dockerfile.ffmpeg_base",
        "djf/ffmpeg_base:latest",
    );
    assert_inferred_image(&rendered, "Dockerfile.helper_x", "djf/helper_x:latest");
    assert_inferred_image(&rendered, "Dockerfile.openssh", "djf/openssh:latest");
}

fn assert_inferred_image(rendered: &str, dockerfile: &str, image: &str) {
    assert!(
        rendered.contains(dockerfile) && rendered.contains(image),
        "rendered view should show inferred image for {dockerfile}\n{rendered}"
    );
}
