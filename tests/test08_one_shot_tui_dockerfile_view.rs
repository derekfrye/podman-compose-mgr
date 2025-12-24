use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use podman_compose_mgr::args::types::{Args, OneShotArgs, SimulateViewMode, TuiArgs};
use podman_compose_mgr::infra::discovery_adapter::FsDiscovery;
use podman_compose_mgr::ports::DiscoveryPort;
use podman_compose_mgr::tui::app::{App, UiState, ViewMode};
use podman_compose_mgr::tui::simulate_view_with_ports;
use podman_compose_mgr::tui::ui;
use podman_compose_mgr::utils::log_utils::Logger;
use tempfile::tempdir;

#[test]
fn simulate_dockerfile_view_emits_registry_matches() {
    let tmp = tempdir().expect("temp dir");
    let root = tmp.path();
    let help_dir = root.join("help");
    fs::create_dir_all(&help_dir).unwrap();
    fs::write(help_dir.join("Dockerfile.ffmpeg"), "FROM scratch\n").unwrap();
    fs::write(help_dir.join("Dockerfile.ffmpeg_base"), "FROM scratch\n").unwrap();

    let include = format!("^{}", regex::escape(help_dir.to_str().unwrap()));

    let args = Args {
        config_toml: None,
        path: root.to_path_buf(),
        verbose: 0,
        exclude_path_patterns: vec![],
        include_path_patterns: vec![format!("{include}.*")],
        build_args: vec![],
        temp_file_path: std::env::temp_dir(),
        podman_bin: None,
        no_cache: false,
        one_shot: OneShotArgs {
            one_shot: true,
            dry_run: true,
        },
        tui: TuiArgs::default(),
        rebuild_view_line_buffer_max:
            podman_compose_mgr::args::types::REBUILD_VIEW_LINE_BUFFER_DEFAULT,
        tui_simulate: Some(SimulateViewMode::Dockerfile),
        tui_simulate_podman_input_json: Some(PathBuf::from("tests/test08/golden.json")),
    };

    let discovery: Arc<dyn DiscoveryPort> = Arc::new(FsDiscovery);
    let logger = Logger::new(0);

    let mut buf = Vec::new();
    simulate_view_with_ports(
        &args,
        SimulateViewMode::Dockerfile,
        &logger,
        discovery,
        podman_compose_mgr::tui::podman_from_json(
            PathBuf::from("tests/test08/golden.json").as_path(),
        )
        .expect("json podman"),
        &mut buf,
    )
    .expect("simulate dockerfile view");

    let out = String::from_utf8(buf).unwrap();
    assert!(
        out.contains("Dockerfile.ffmpeg -> registry matched (more than one Dockerfile in the dir) / registry name matched localhost/djf/ffmpeg:latest"),
        "ffmpeg line should include registry match\n{out}"
    );
    assert!(
        out.contains("Dockerfile.ffmpeg_base -> registry matched (more than one Dockerfile in the dir) / registry name matched localhost/djf/ffmpeg_base:latest"),
        "ffmpeg_base line should include registry match\n{out}"
    );
}

#[test]
fn tui_render_shows_inferred_images_from_json() {
    let tmp = tempdir().expect("temp dir");
    let root = tmp.path();
    let help_dir = root.join("help");
    fs::create_dir_all(&help_dir).unwrap();
    let dockerfiles = [
        "Dockerfile.ffmpeg",
        "Dockerfile.ffmpeg_base",
        "Dockerfile.helper_x",
        "Dockerfile.openssh",
    ];
    for name in dockerfiles {
        fs::write(help_dir.join(name), "FROM scratch\n").unwrap();
    }

    let include = format!("^{}", regex::escape(help_dir.to_str().unwrap()));

    let args = Args {
        config_toml: None,
        path: root.to_path_buf(),
        verbose: 0,
        exclude_path_patterns: vec![],
        include_path_patterns: vec![format!("{include}.*")],
        build_args: vec![],
        temp_file_path: std::env::temp_dir(),
        podman_bin: None,
        no_cache: false,
        one_shot: OneShotArgs::default(),
        tui: TuiArgs {
            enabled: true,
            ..TuiArgs::default()
        },
        rebuild_view_line_buffer_max:
            podman_compose_mgr::args::types::REBUILD_VIEW_LINE_BUFFER_DEFAULT,
        tui_simulate: None,
        tui_simulate_podman_input_json: None,
    };

    // Prepare state by running the same scan the TUI uses, but with podman JSON injected.
    let discovery: Arc<dyn DiscoveryPort> = Arc::new(FsDiscovery);
    let podman = podman_compose_mgr::tui::podman_from_json(
        PathBuf::from("tests/test08/golden.json").as_path(),
    )
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

    // Render a frame and capture buffer.
    let backend = ratatui::backend::TestBackend::new(120, 20);
    let mut terminal = ratatui::Terminal::new(backend).expect("terminal");
    terminal
        .draw(|f| ui::draw(f, &mut app, &args))
        .expect("draw");
    let buffer = terminal.backend_mut().buffer().clone();
    let mut rendered = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = buffer.cell((x, y)).unwrap();
            rendered.push_str(cell.symbol());
        }
        rendered.push('\n');
    }
    println!("{rendered}");

    assert!(
        rendered.contains("Dockerfile.ffmpeg") && rendered.contains("djf/ffmpeg:latest"),
        "rendered view should show inferred ffmpeg image\n{rendered}"
    );
    assert!(
        rendered.contains("Dockerfile.ffmpeg_base") && rendered.contains("djf/ffmpeg_base:latest"),
        "rendered view should show inferred ffmpeg_base image\n{rendered}"
    );
    assert!(
        rendered.contains("Dockerfile.helper_x") && rendered.contains("djf/helper_x:latest"),
        "rendered view should show inferred helper_x image\n{rendered}"
    );
    assert!(
        rendered.contains("Dockerfile.openssh") && rendered.contains("djf/openssh:latest"),
        "rendered view should show inferred openssh image\n{rendered}"
    );
}
