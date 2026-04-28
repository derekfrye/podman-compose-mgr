use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::Receiver;
use podman_compose_mgr::Args;
use podman_compose_mgr::app::AppCore;
use podman_compose_mgr::args::types::{OneShotArgs, REBUILD_VIEW_LINE_BUFFER_DEFAULT, TuiArgs};
use podman_compose_mgr::infra::discovery_adapter::FsDiscovery;
use podman_compose_mgr::infra::podman_adapter::PodmanCli;
use podman_compose_mgr::tui::app::{self, App, Msg, Services};
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

pub(crate) fn wait_for_rebuild(app: &mut App, services: &Services, rx: &Receiver<Msg>) {
    let timeout = Duration::from_mins(1);
    while let Ok(msg) = rx.recv_timeout(timeout) {
        app::update_with_services(app, msg, Some(services));
        if matches!(app.rebuild.as_ref(), Some(state) if state.finished) {
            break;
        }
    }

    while let Ok(msg) = rx.try_recv() {
        app::update_with_services(app, msg, Some(services));
    }

    let rebuild = app
        .rebuild
        .as_ref()
        .expect("rebuild state should be present after completion");
    assert!(rebuild.finished, "rebuild never reported completion");
}

pub(crate) struct TestContext {
    pub(crate) args: Args,
    pub(crate) services: Services,
    pub(crate) app: App,
    pub(crate) terminal: Terminal<TestBackend>,
    pub(crate) rx: Receiver<Msg>,
    pub(crate) width: u16,
    pub(crate) height: u16,
}

impl TestContext {
    pub(crate) fn new() -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        build_mock_podman(&manifest_dir);
        let podman_bin = mock_podman_binary(&manifest_dir);
        assert!(
            podman_bin.exists(),
            "mock podman binary missing at {}",
            podman_bin.display()
        );
        let args = test_args(&manifest_dir, &podman_bin);
        podman_compose_mgr::utils::podman_utils::set_podman_binary_override(
            podman_bin.into_os_string(),
        );

        let (tx, rx) = crossbeam_channel::unbounded();
        let discovery = Arc::new(FsDiscovery);
        let podman = Arc::new(PodmanCli);
        let core = Arc::new(AppCore::new(discovery, podman));
        let services = Services {
            core,
            root: args.path.clone(),
            include: args.include_path_patterns.clone(),
            exclude: args.exclude_path_patterns.clone(),
            tx,
            args: args.clone(),
            working_dir: args.path.clone(),
        };

        let mut app = App::new();
        app.set_root_path(args.path.clone());
        app.auto_rebuild_all = args.tui.rebuild_all();

        let width = 120;
        let height = 32;
        let backend = TestBackend::new(width, height);
        let terminal = Terminal::new(backend).expect("terminal");

        Self {
            args,
            services,
            app,
            terminal,
            rx,
            width,
            height,
        }
    }

    pub(crate) fn seed_scan_results(&mut self) {
        let discovered = self
            .services
            .core
            .scan_images(
                self.args.path.clone(),
                self.args.include_path_patterns.clone(),
                self.args.exclude_path_patterns.clone(),
            )
            .expect("scan images for test data");

        app::update_with_services(
            &mut self.app,
            Msg::ScanResults(discovered),
            Some(&self.services),
        );
    }

    pub(crate) fn capture_view(&mut self) -> (Buffer, String) {
        self.terminal
            .draw(|f| ui::draw(f, &mut self.app, &self.args))
            .expect("draw view");
        let buffer = self.terminal.backend().buffer().clone();
        let view = self.buffer_to_string(&buffer);
        (buffer, view)
    }

    pub(crate) fn buffer_to_string(&self, buffer: &Buffer) -> String {
        let mut rendered = String::new();
        for y in 0..self.height {
            for x in 0..self.width {
                if let Some(cell) = buffer.cell((x, y)) {
                    rendered.push_str(cell.symbol());
                }
            }
            rendered.push('\n');
        }
        rendered
    }
}

fn test_args(manifest_dir: &Path, podman_bin: &Path) -> Args {
    Args {
        config_toml: None,
        path: manifest_dir.join("tests").join("test07"),
        verbose: 0,
        exclude_path_patterns: vec![],
        include_path_patterns: vec![],
        build_args: vec![],
        temp_file_path: std::env::temp_dir(),
        podman_bin: Some(podman_bin.to_path_buf()),
        no_cache: false,
        one_shot: OneShotArgs::default(),
        tui: TuiArgs {
            enabled: true,
            rebuild_all: true,
        },
        rebuild_view_line_buffer_max: REBUILD_VIEW_LINE_BUFFER_DEFAULT,
        tui_simulate_podman_input_json: None,
        tui_simulate: None,
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        podman_compose_mgr::utils::podman_utils::clear_podman_binary_override();
    }
}
