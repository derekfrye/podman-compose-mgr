use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::Receiver;
use podman_compose_mgr::Args;
use podman_compose_mgr::app::AppCore;
use podman_compose_mgr::infra::discovery_adapter::FsDiscovery;
use podman_compose_mgr::infra::podman_adapter::PodmanCli;
use podman_compose_mgr::tui::app::{self, App, Msg, RebuildStatus, Services, UiState};
use podman_compose_mgr::tui::ui;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;

fn build_mock_podman(manifest_dir: &Path) {
    let mock_manifest = manifest_dir.join("mock_podman").join("Cargo.toml");
    let status = std::process::Command::new("cargo")
        .current_dir(manifest_dir)
        .args([
            "build",
            "--manifest-path",
            mock_manifest.to_str().expect("utf8 path"),
        ])
        .status()
        .expect("failed to invoke cargo build for mock podman");
    assert!(status.success(), "mock podman binary failed to build");
}

fn mock_podman_binary(manifest_dir: &Path) -> PathBuf {
    let mut path = manifest_dir
        .join("mock_podman")
        .join("target")
        .join("debug")
        .join("podman");
    if cfg!(target_os = "windows") {
        path.set_extension("exe");
    }
    path
}

fn wait_for_rebuild(app: &mut App, services: &Services, rx: &Receiver<Msg>) {
    let timeout = Duration::from_secs(60);
    while let Ok(msg) = rx.recv_timeout(timeout) {
        app::update_with_services(app, msg, Some(services));
        if matches!(app.rebuild.as_ref(), Some(state) if state.finished) {
            break;
        }
    }

    // Drain any trailing messages emitted after completion
    while let Ok(msg) = rx.try_recv() {
        app::update_with_services(app, msg, Some(services));
    }

    let rebuild = app
        .rebuild
        .as_ref()
        .expect("rebuild state should be present after completion");
    assert!(rebuild.finished, "rebuild never reported completion");
}

#[test]
fn tui_rebuild_all_streams_output_and_scrolls_to_top() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    build_mock_podman(&manifest_dir);
    let podman_bin = mock_podman_binary(&manifest_dir);
    assert!(
        podman_bin.exists(),
        "mock podman binary missing at {podman_bin:?}"
    );
    unsafe { std::env::set_var("PODMGR_PODMAN_BIN", &podman_bin) };

    let test_path = manifest_dir.join("tests").join("test07");
    let args = Args {
        path: test_path,
        verbose: 0,
        exclude_path_patterns: vec![],
        include_path_patterns: vec![],
        build_args: vec![],
        temp_file_path: std::env::temp_dir(),
        tui: true,
        tui_rebuild_all: true,
    };

    let (tx, rx) = crossbeam_channel::unbounded();
    let discovery = Arc::new(FsDiscovery);
    let podman = Arc::new(PodmanCli);
    let core = Arc::new(AppCore::new(discovery, podman));

    let services = Services {
        core,
        root: args.path.clone(),
        include: args.include_path_patterns.clone(),
        exclude: args.exclude_path_patterns.clone(),
        tx: tx.clone(),
        args: args.clone(),
    };

    let mut app = App::new();
    app.set_root_path(args.path.clone());
    app.auto_rebuild_all = args.tui_rebuild_all;

    let discovered = services
        .core
        .scan_images(
            args.path.clone(),
            args.include_path_patterns.clone(),
            args.exclude_path_patterns.clone(),
        )
        .expect("scan images for test data");

    app::update_with_services(&mut app, Msg::ScanResults(discovered), Some(&services));

    wait_for_rebuild(&mut app, &services, &rx);

    assert_eq!(
        app.state,
        UiState::Rebuilding,
        "app should remain in rebuild view"
    );

    let rebuild = app.rebuild.as_ref().expect("rebuild state expected");
    assert_eq!(rebuild.jobs.len(), 2, "expected two rebuild jobs");
    assert!(
        rebuild
            .jobs
            .iter()
            .all(|job| job.status == RebuildStatus::Succeeded),
        "all jobs should succeed"
    );

    let ddns = &rebuild.jobs[0];
    assert!(
        !ddns.output.is_empty(),
        "ddns job should record at least the prompt output"
    );
    let ddns_prompt = ddns
        .output
        .front()
        .expect("ddns job output should include prompt");
    assert!(
        ddns_prompt.text.starts_with("Refresh djf/ddns"),
        "ddns prompt should be visible at start of output"
    );
    assert!(
        ddns_prompt.text.contains("p/N/d/b/s/?:"),
        "ddns prompt should include action shortcuts"
    );
    assert!(
        ddns.output
            .iter()
            .any(|line| line.text.contains("Auto-selecting 'b' (build)")),
        "ddns job should automatically select build"
    );

    let rclone = &rebuild.jobs[1];
    assert!(
        rclone
            .output
            .iter()
            .any(|line| line.text.starts_with("Refresh djf/rclone")),
        "rclone job should display rebuild prompt"
    );
    assert!(
        rclone
            .output
            .iter()
            .any(|line| line.text.contains("Auto-selecting 'b' (build)")),
        "rclone job should automatically select build"
    );
    assert!(
        rclone
            .output
            .iter()
            .any(|line| line.text.contains("Rebuild queue completed")),
        "final job should report queue completion"
    );

    // Ensure scrolling to top resets the viewport to the first line
    {
        let rebuild_mut = app.rebuild.as_mut().expect("mutable rebuild state");
        rebuild_mut.active_idx = 0;
    }
    app::update_with_services(&mut app, Msg::ScrollOutputBottom, Some(&services));
    app::update_with_services(&mut app, Msg::ScrollOutputTop, Some(&services));

    let (active_header, job_images) = {
        let rebuild_after_scroll = app.rebuild.as_ref().expect("rebuild state after scroll");
        assert_eq!(
            rebuild_after_scroll.scroll_y, 0,
            "scroll should point to first line"
        );
        let first_line = rebuild_after_scroll.jobs[0]
            .output
            .front()
            .expect("ddns job should have output");
        assert!(
            first_line.text.starts_with("Refresh djf/ddns"),
            "scroll to top should reveal prompt line"
        );

        let active_header = {
            let job = &rebuild_after_scroll.jobs[0];
            match &job.container {
                Some(container) => format!("{} ({container})", job.image),
                None => job.image.clone(),
            }
        };

        let job_images: Vec<String> = rebuild_after_scroll
            .jobs
            .iter()
            .map(|job| job.image.clone())
            .collect();

        (active_header, job_images)
    };

    let width: u16 = 120;
    let height: u16 = 32;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");

    terminal
        .draw(|f| ui::draw(f, &app, &args))
        .expect("draw rebuild view");
    let base_buffer = terminal.backend().buffer().clone();

    let buffer_to_string = |buffer: &Buffer| {
        let mut rendered = String::new();
        for y in 0..height {
            for x in 0..width {
                if let Some(cell) = buffer.cell((x, y)) {
                    rendered.push_str(cell.symbol());
                }
            }
            rendered.push('\n');
        }
        rendered
    };

    let base_view = buffer_to_string(&base_buffer);
    assert!(
        base_view.contains(&active_header),
        "output pane should render active job header"
    );
    assert!(base_view.contains("Legend"), "legend pane should render");
    assert!(
        base_view.contains("Job: 1/2"),
        "sidebar should show job position"
    );
    assert!(
        base_view.contains("Status: Done"),
        "sidebar should show completed status"
    );
    assert!(
        base_view.contains("Refresh djf/ddns"),
        "output pane should include prompt text"
    );

    app::update_with_services(&mut app, Msg::OpenWorkQueue, Some(&services));
    terminal
        .draw(|f| ui::draw(f, &app, &args))
        .expect("draw work queue modal");
    let modal_buffer = terminal.backend().buffer().clone();
    let modal_view = buffer_to_string(&modal_buffer);
    assert!(
        modal_view.contains("Work Queue (Esc=close)"),
        "work queue modal title should render"
    );
    assert!(
        modal_view.contains(&format!("▶ {}", job_images[0])),
        "work queue should highlight ddns job"
    );
    assert!(
        modal_view.contains(&job_images[1]),
        "work queue should list rclone job"
    );

    unsafe { std::env::remove_var("PODMGR_PODMAN_BIN") };
}
