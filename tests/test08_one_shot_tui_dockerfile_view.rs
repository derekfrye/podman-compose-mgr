use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use chrono::{Duration, Local};
use crossbeam_channel as xchan;
use podman_compose_mgr::args::types::{Args, OneShotArgs, SimulateViewMode, TuiArgs};
use podman_compose_mgr::domain::LocalImageSummary;
use podman_compose_mgr::errors::PodmanComposeMgrError;
use podman_compose_mgr::infra::discovery_adapter::FsDiscovery;
use podman_compose_mgr::ports::{DiscoveryPort, PodmanPort};
use podman_compose_mgr::tui::app::{self, App, Msg, Services, UiState, ViewMode};
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
    fs::create_dir_all(&app_dir).unwrap();
    fs::write(app_dir.join("Makefile"), "clean:\n\t@echo clean\n").unwrap();
    fs::write(
        app_dir.join("m-miniflare.container"),
        "[Container]\nImage=localhost/djf/m-golf-srvless:latest\n",
    )
    .unwrap();

    let args = Args {
        config_toml: None,
        path: root.to_path_buf(),
        verbose: 0,
        exclude_path_patterns: vec![],
        include_path_patterns: vec![],
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

    let discovery: Arc<dyn DiscoveryPort> = Arc::new(FsDiscovery);
    let image = "localhost/djf/m-golf-srvless:latest";
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

    let mut app = App::new();
    app.state = UiState::Ready;
    app.view_mode = ViewMode::ByMakefile;
    app.all_items = scan.images;
    app.makefile_items = scan.makefiles;
    app.set_root_path(root.to_path_buf());
    app.rebuild_rows_for_view();

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

    let backend = ratatui::backend::TestBackend::new(140, 20);
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
