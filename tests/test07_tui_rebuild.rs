use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::Receiver;
use podman_compose_mgr::Args;
use podman_compose_mgr::app::AppCore;
use podman_compose_mgr::infra::discovery_adapter::FsDiscovery;
use podman_compose_mgr::infra::podman_adapter::PodmanCli;
use podman_compose_mgr::tui::app::{self, App, Msg, OutputStream, RebuildStatus, Services, UiState};
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
    // Flip on the same "rebuild everything" flag the real CLI uses so the scanner response
    // immediately queues the rebuild jobs below.
    app.auto_rebuild_all = args.tui_rebuild_all;

    let discovered = services
        .core
        .scan_images(
            args.path.clone(),
            args.include_path_patterns.clone(),
            args.exclude_path_patterns.clone(),
        )
        .expect("scan images for test data");

    // Feeding ScanResults into the state machine triggers the auto rebuild logic and enqueues
    // jobs on the worker thread via the channel wiring stored in `services`.
    app::update_with_services(&mut app, Msg::ScanResults(discovered), Some(&services));

    let width: u16 = 120;
    let height: u16 = 32;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");

    // Helpers below treat the TestBackend buffer like a bare terminal grid to keep assertions
    // readable.
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

    // Render the initial list view to populate the buffer with table text.
    terminal
        .draw(|f| ui::draw(f, &app, &args))
        .expect("draw list view");
    let list_buffer = terminal.backend().buffer().clone();
    let list_view = buffer_to_string(&list_buffer);
    // `Podman container for rclone` comes from the table view row; it should remain in the
    // buffer after we repaint only the rebuild pane unless we clear the area first.
    assert!(
        list_view.contains("Podman container for rclone"),
        "fixtures should include a second table row for rclone"
    );

    // Pump the message channel until the rebuild worker reports completion so the UI state is
    // identical to what a user would see after hitting `r` in the live TUI.
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
    assert!(
        ddns.output
            .iter()
            .any(|line| line.text.contains("STEP 1/10: FROM alpine:latest")),
        "ddns job should capture podman STEP output"
    );
    assert!(
        ddns.output
            .iter()
            .any(|line| line.text.contains("apk add jq curl bind-tools tini")),
        "ddns job should include representative build command output"
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
    assert!(
        rclone
            .output
            .iter()
            .any(|line| line.text.contains("STEP 1/8: FROM fedora:42")),
        "rclone job should capture podman STEP output"
    );
    assert!(
        rclone
            .output
            .iter()
            .any(|line| line.text.contains("Updating and loading repositories")),
        "rclone job should stream repository update output"
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

    let root_area = ratatui::layout::Rect::new(0, 0, width, height);
    // Recreate the same nested layout used by the production draw call so the manual rendering
    // below hits the identical rectangles as the real widget tree.
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .margin(1)
        .constraints([ratatui::layout::Constraint::Length(3), ratatui::layout::Constraint::Min(3)])
        .split(root_area);
    let rebuild_chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Min(40),
            ratatui::layout::Constraint::Length(24),
        ])
        .split(chunks[1]);

    let rebuild_state = app.rebuild.as_ref().expect("rebuild state for stale view");
    let job = &rebuild_state.jobs[rebuild_state.active_idx];
    let header = match &job.container {
        Some(container) => format!("{} ({container})", job.image),
        None => job.image.clone(),
    };
    let trimmed_lines: Vec<_> = job
        .output
        .iter()
        .cloned()
        .take(2)
        .map(|entry| match entry.stream {
            OutputStream::Stdout => ratatui::text::Line::from(vec![ratatui::text::Span::raw(
                entry.text,
            )]),
            OutputStream::Stderr => ratatui::text::Line::from(vec![ratatui::text::Span::styled(
                entry.text,
                ratatui::style::Style::default().fg(ratatui::style::Color::LightRed),
            )]),
        })
        .collect();

    // Build the rebuild output widget on demand; cloning lets us render it twice with and without
    // clearing just like the buggy vs fixed paint paths.
    let make_paragraph = || {
        ratatui::widgets::Paragraph::new(trimmed_lines.clone())
            .block(
                ratatui::widgets::Block::default()
                    .title(header.clone())
                    .borders(ratatui::widgets::Borders::ALL),
            )
            .wrap(ratatui::widgets::Wrap { trim: false })
            .scroll((rebuild_state.scroll_y, rebuild_state.scroll_x))
    };

    // First render without clearing – this reproduces the UI bug.
    let mut stale_buffer = list_buffer.clone();
    make_paragraph().render(rebuild_chunks[0], &mut stale_buffer);
    let stale_view = buffer_to_string(&stale_buffer);
    assert!(
        stale_view.contains("Podman container for rclone"),
        "stale table content should remain visible without clearing"
    );

    // Now render with a Clear widget to verify the intended fix.
    let mut cleared_buffer = list_buffer.clone();
    ratatui::widgets::Clear.render(rebuild_chunks[0], &mut cleared_buffer);
    make_paragraph().render(rebuild_chunks[0], &mut cleared_buffer);
    let cleared_view = buffer_to_string(&cleared_buffer);
    assert!(
        !cleared_view.contains("Podman container for rclone"),
        "clearing the pane before rendering should remove stale table text"
    );

    terminal
        .draw(|f| ui::draw(f, &app, &args))
        .expect("draw rebuild view");
    let base_buffer = terminal.backend().buffer().clone();

    let base_view = buffer_to_string(&base_buffer);
    println!("\n===== Rebuild View (Top) =====\n{base_view}");
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
    assert!(
        base_view.contains("STEP 1/10: FROM alpine:latest"),
        "output pane should render streamed podman output"
    );
    // Simulate scrolling to the bottom repeatedly
    for _ in 0..90 {
        app::update_with_services(&mut app, Msg::ScrollOutputDown, Some(&services));
    }

    terminal
        .draw(|f| ui::draw(f, &app, &args))
        .expect("draw rebuild view bottom");
    let bottom_buffer = terminal.backend().buffer().clone();
    let bottom_view = buffer_to_string(&bottom_buffer);
    println!("===== Rebuild View (90 Down Arrows) =====\n{bottom_view}");

    app::update_with_services(&mut app, Msg::OpenWorkQueue, Some(&services));
    terminal
        .draw(|f| ui::draw(f, &app, &args))
        .expect("draw work queue modal");
    let modal_buffer = terminal.backend().buffer().clone();
    let modal_view = buffer_to_string(&modal_buffer);
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

    unsafe { std::env::remove_var("PODMGR_PODMAN_BIN") };
}
