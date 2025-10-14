use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::Receiver;
use podman_compose_mgr::Args;
use podman_compose_mgr::app::AppCore;
use podman_compose_mgr::args::types::REBUILD_VIEW_LINE_BUFFER_DEFAULT;
use podman_compose_mgr::infra::discovery_adapter::FsDiscovery;
use podman_compose_mgr::infra::podman_adapter::PodmanCli;
use podman_compose_mgr::tui::app::{
    self, App, Msg, OutputStream, RebuildJob, RebuildState, RebuildStatus, Services, UiState,
};
use podman_compose_mgr::tui::ui;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::widgets::Widget;

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

struct TestContext {
    args: Args,
    services: Services,
    app: App,
    terminal: Terminal<TestBackend>,
    rx: Receiver<Msg>,
    width: u16,
    height: u16,
}

impl TestContext {
    fn new() -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        build_mock_podman(&manifest_dir);
        let podman_bin = mock_podman_binary(&manifest_dir);
        assert!(
            podman_bin.exists(),
            "mock podman binary missing at {}",
            podman_bin.display()
        );
        let args = Args {
            path: manifest_dir.join("tests").join("test07"),
            verbose: 0,
            exclude_path_patterns: vec![],
            include_path_patterns: vec![],
            build_args: vec![],
            temp_file_path: std::env::temp_dir(),
            podman_bin: Some(podman_bin.clone()),
            no_cache: false,
            tui: true,
            tui_rebuild_all: true,
            rebuild_view_line_buffer_max: REBUILD_VIEW_LINE_BUFFER_DEFAULT,
        };

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
            tx: tx.clone(),
            args: args.clone(),
        };

        let mut app = App::new();
        app.set_root_path(args.path.clone());
        app.auto_rebuild_all = args.tui_rebuild_all;

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

    fn seed_scan_results(&mut self) {
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

    fn capture_view(&mut self) -> (Buffer, String) {
        self.terminal
            .draw(|f| ui::draw(f, &mut self.app, &self.args))
            .expect("draw view");
        let buffer = self.terminal.backend().buffer().clone();
        let view = self.buffer_to_string(&buffer);
        (buffer, view)
    }

    fn buffer_to_string(&self, buffer: &Buffer) -> String {
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

impl Drop for TestContext {
    fn drop(&mut self) {
        podman_compose_mgr::utils::podman_utils::clear_podman_binary_override();
    }
}

#[test]
fn tui_rebuild_all_streams_output_and_scrolls_to_top() {
    let mut ctx = TestContext::new();
    ctx.seed_scan_results();

    let (list_buffer, list_view) = ctx.capture_view();
    assert_initial_list_view(&list_view);

    wait_for_rebuild(&mut ctx.app, &ctx.services, &ctx.rx);
    assert_app_in_rebuild(&ctx.app);

    let rebuild = ctx.app.rebuild.as_ref().expect("rebuild state expected");
    assert_eq!(rebuild.jobs.len(), 2, "expected two rebuild jobs");
    assert!(
        rebuild
            .jobs
            .iter()
            .all(|job| job.status == RebuildStatus::Succeeded),
        "all jobs should succeed"
    );

    assert_ddns_job(&rebuild.jobs[0]);
    assert_rclone_job(&rebuild.jobs[1]);

    let (active_header, job_images) = reset_scroll_and_verify(&mut ctx.app, &ctx.services);
    verify_manual_rendering(&mut ctx, &list_buffer, &active_header);
    verify_rebuild_view(&mut ctx, &active_header);
    verify_scroll_down(&mut ctx);
    verify_work_queue_modal(&mut ctx, &job_images);
}

#[test]
fn rebuild_output_overwrites_trailing_cells() {
    let args = Args::default();
    let mut app = App::new();
    app.state = UiState::Rebuilding;
    let mut rebuild = RebuildState::new(
        vec![RebuildJob::new(
            "img".into(),
            Some("container".into()),
            std::path::PathBuf::from("."),
            std::path::PathBuf::from("."),
        )],
        args.rebuild_view_line_buffer_max,
    );
    rebuild.jobs[0].status = RebuildStatus::Running;
    app.rebuild = Some(rebuild);

    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");

    let long_chunk = "X".repeat(60);
    app::update_with_services(
        &mut app,
        Msg::RebuildJobOutput {
            job_idx: 0,
            chunk: long_chunk,
            stream: OutputStream::Stdout,
        },
        None,
    );
    terminal
        .draw(|f| ui::draw(f, &mut app, &args))
        .expect("first draw");

    app::update_with_services(
        &mut app,
        Msg::RebuildJobOutput {
            job_idx: 0,
            chunk: "short".to_string(),
            stream: OutputStream::Stdout,
        },
        None,
    );
    terminal
        .draw(|f| ui::draw(f, &mut app, &args))
        .expect("second draw");

    let buffer = terminal.backend().buffer().clone();
    let mut rendered_lines = Vec::new();
    for y in 0..buffer.area.height {
        let mut line = String::new();
        for x in 0..buffer.area.width {
            if let Some(cell) = buffer.cell((x, y)) {
                line.push_str(cell.symbol());
            }
        }
        rendered_lines.push(line);
    }

    let short_line = rendered_lines
        .into_iter()
        .find(|line| line.contains("short"))
        .expect("short line present");
    let short_pos = short_line.find("short").expect("short text located");
    let after_short = short_pos + "short".len();
    let right_border = short_line[after_short..]
        .find('│')
        .map(|idx| after_short + idx)
        .unwrap_or_else(|| short_line.len());
    let tail = &short_line[after_short..right_border];
    assert!(
        tail.chars().all(|c| c == ' '),
        "residual characters detected"
    );
}

fn assert_initial_list_view(list_view: &str) {
    assert!(
        list_view.contains("Podman container for rclone"),
        "fixtures should include a second table row for rclone"
    );
}

fn assert_app_in_rebuild(app: &App) {
    assert_eq!(
        app.state,
        UiState::Rebuilding,
        "app should remain in rebuild view"
    );
}

fn assert_ddns_job(job: &RebuildJob) {
    assert!(
        !job.output.is_empty(),
        "ddns job should record at least the prompt output"
    );
    let prompt = job
        .output
        .front()
        .expect("ddns job output should include prompt");
    assert!(
        prompt.text.starts_with("Refresh djf/ddns"),
        "ddns prompt should be visible at start of output"
    );
    assert!(
        prompt.text.contains("p/N/d/b/s/?:"),
        "ddns prompt should include action shortcuts"
    );
    assert!(
        job.output
            .iter()
            .any(|line| line.text.contains("Auto-selecting 'b' (build)")),
        "ddns job should automatically select build"
    );
    assert!(
        job.output
            .iter()
            .any(|line| line.text.contains("STEP 1/10: FROM alpine:latest")),
        "ddns job should capture podman STEP output"
    );
    assert!(
        job.output
            .iter()
            .any(|line| line.text.contains("apk add jq curl bind-tools tini")),
        "ddns job should include representative build command output"
    );
}

fn assert_rclone_job(job: &RebuildJob) {
    assert!(
        job.output
            .iter()
            .any(|line| line.text.starts_with("Refresh djf/rclone")),
        "rclone job should display rebuild prompt"
    );
    assert!(
        job.output
            .iter()
            .any(|line| line.text.contains("Auto-selecting 'b' (build)")),
        "rclone job should automatically select build"
    );
    assert!(
        job.output
            .iter()
            .any(|line| line.text.contains("Rebuild queue completed")),
        "final job should report queue completion"
    );
    assert!(
        job.output
            .iter()
            .any(|line| line.text.contains("STEP 1/8: FROM fedora:42")),
        "rclone job should capture podman STEP output"
    );
    assert!(
        job.output
            .iter()
            .any(|line| line.text.contains("Updating and loading repositories")),
        "rclone job should stream repository update output"
    );
}

fn reset_scroll_and_verify(app: &mut App, services: &Services) -> (String, Vec<String>) {
    if let Some(rebuild) = app.rebuild.as_mut() {
        rebuild.active_idx = 0;
    }
    app::update_with_services(app, Msg::ScrollOutputBottom, Some(services));
    app::update_with_services(app, Msg::ScrollOutputTop, Some(services));

    let rebuild_state = app.rebuild.as_ref().expect("rebuild state after scroll");
    assert_eq!(
        rebuild_state.scroll_y, 0,
        "scroll should point to first line"
    );
    let first_line = rebuild_state.jobs[0]
        .output
        .front()
        .expect("ddns job should have output");
    assert!(
        first_line.text.starts_with("Refresh djf/ddns"),
        "scroll to top should reveal prompt line"
    );

    let active_header = match (
        &rebuild_state.jobs[0].image,
        &rebuild_state.jobs[0].container,
    ) {
        (image, Some(container)) => format!("{image} ({container})"),
        (image, None) => image.clone(),
    };

    let job_images = rebuild_state
        .jobs
        .iter()
        .map(|job| job.image.clone())
        .collect();

    (active_header, job_images)
}

fn verify_manual_rendering(ctx: &mut TestContext, list_buffer: &Buffer, active_header: &str) {
    let root_area = ratatui::layout::Rect::new(0, 0, ctx.width, ctx.height);
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .margin(1)
        .constraints([
            ratatui::layout::Constraint::Length(3),
            ratatui::layout::Constraint::Min(3),
        ])
        .split(root_area);
    let rebuild_chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Min(40),
            ratatui::layout::Constraint::Length(24),
        ])
        .split(chunks[1]);

    let rebuild_state = ctx
        .app
        .rebuild
        .as_ref()
        .expect("rebuild state for manual render");
    let job = &rebuild_state.jobs[rebuild_state.active_idx];
    let header = active_header.to_string();
    let trimmed_lines: Vec<_> = job
        .output
        .iter()
        .take(2)
        .cloned()
        .map(|entry| match entry.stream {
            OutputStream::Stdout => {
                ratatui::text::Line::from(vec![ratatui::text::Span::raw(entry.text)])
            }
            OutputStream::Stderr => ratatui::text::Line::from(vec![ratatui::text::Span::styled(
                entry.text,
                ratatui::style::Style::default().fg(ratatui::style::Color::LightRed),
            )]),
        })
        .collect();

    let make_paragraph = || {
        ratatui::widgets::Paragraph::new(trimmed_lines.clone())
            .block(
                ratatui::widgets::Block::default()
                    .title(header.clone())
                    .borders(ratatui::widgets::Borders::ALL),
            )
            .scroll((rebuild_state.scroll_y, rebuild_state.scroll_x))
    };

    let mut stale_buffer = list_buffer.clone();
    make_paragraph().render(rebuild_chunks[0], &mut stale_buffer);
    let stale_view = ctx.buffer_to_string(&stale_buffer);
    assert!(
        !stale_view.contains("Podman container for rclone"),
        "manual render should overwrite stale list content"
    );

    let mut cleared_buffer = list_buffer.clone();
    ratatui::widgets::Clear.render(rebuild_chunks[0], &mut cleared_buffer);
    make_paragraph().render(rebuild_chunks[0], &mut cleared_buffer);
    let cleared_view = ctx.buffer_to_string(&cleared_buffer);
    assert!(
        !cleared_view.contains("Podman container for rclone"),
        "clearing the pane before rendering should remove stale table text"
    );
}

fn verify_rebuild_view(ctx: &mut TestContext, active_header: &str) {
    let (base_buffer, base_view) = ctx.capture_view();
    println!("\n===== Rebuild View (Top) =====\n{base_view}");
    assert!(
        base_view.contains(active_header),
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
    assert!(
        base_view.contains("STEP 1/10: FROM alpine:latest"),
        "output pane should render streamed podman output"
    );

    // Keep buffer in scope for potential debugging even though we only need the string.
    let _ = base_buffer;
}

fn verify_scroll_down(ctx: &mut TestContext) {
    for _ in 0..90 {
        app::update_with_services(&mut ctx.app, Msg::ScrollOutputDown, Some(&ctx.services));
    }

    let (_, bottom_view) = ctx.capture_view();
    println!("===== Rebuild View (90 Down Arrows) =====\n{bottom_view}");
}

fn verify_work_queue_modal(ctx: &mut TestContext, job_images: &[String]) {
    app::update_with_services(&mut ctx.app, Msg::OpenWorkQueue, Some(&ctx.services));
    let (_, modal_view) = ctx.capture_view();
    println!("===== Work Queue Modal =====\n{modal_view}");
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
}
