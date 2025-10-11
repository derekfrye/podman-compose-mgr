use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::Receiver;
use podman_compose_mgr::Args;
use podman_compose_mgr::app::AppCore;
use podman_compose_mgr::infra::discovery_adapter::FsDiscovery;
use podman_compose_mgr::infra::podman_adapter::PodmanCli;
use podman_compose_mgr::tui::app::{self, App, Msg, RebuildStatus, Services, UiState};

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
    let ddns_output = ddns
        .output
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    eprintln!("first_ddns_line={:?}", ddns.output.front().map(|l| l.text.clone()));
    eprintln!("ddns_output_snippet={:?}", &ddns_output[..ddns_output.len().min(120)]);
    assert!(ddns_output.contains("STEP 1/10: FROM alpine:latest"));
    assert!(ddns_output.contains("Successfully tagged localhost/djf/ddns:latest"));

    let rclone = &rebuild.jobs[1];
    let rclone_output = rclone
        .output
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rclone_output.contains("STEP 1/8: FROM fedora:42"));
    assert!(rclone_output.contains("Successfully tagged localhost/djf/rclone:latest"));

    // Ensure scrolling to top resets the viewport to the first line
    {
        let rebuild_mut = app.rebuild.as_mut().expect("mutable rebuild state");
        rebuild_mut.active_idx = 0;
    }
    app::update_with_services(&mut app, Msg::ScrollOutputBottom, Some(&services));
    app::update_with_services(&mut app, Msg::ScrollOutputTop, Some(&services));

    let rebuild_after_scroll = app.rebuild.as_ref().expect("rebuild state after scroll");
    assert_eq!(
        rebuild_after_scroll.scroll_y, 0,
        "scroll should point to first line"
    );
    let first_line = rebuild_after_scroll.jobs[0]
        .output
        .front()
        .expect("ddns job should have output");
    assert_eq!(first_line.text, "STEP 1/10: FROM alpine:latest");

    unsafe { std::env::remove_var("PODMGR_PODMAN_BIN") };
}
