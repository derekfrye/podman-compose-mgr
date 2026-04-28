use podman_compose_mgr::Args;
use podman_compose_mgr::tui::app::{
    self, App, Msg, OutputStream, RebuildJob, RebuildState, RebuildStatus, UiState,
};
use podman_compose_mgr::tui::ui;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use std::path::PathBuf;

#[test]
fn rebuild_home_and_end_keys_adjust_scroll_and_auto_follow() {
    let args = Args::default();
    let mut app = app_with_scrollable_output(&args);
    let mut terminal = Terminal::new(TestBackend::new(60, 20)).expect("terminal");
    terminal
        .draw(|f| ui::draw(f, &mut app, &args))
        .expect("initial draw");

    app::update_with_services(&mut app, Msg::ScrollOutputTop, None);
    let rebuild = app.rebuild.as_ref().expect("rebuild state");
    assert_eq!(rebuild.scroll_y, 0);
    assert!(!rebuild.auto_scroll, "home should disable auto-follow");

    app::update_with_services(&mut app, Msg::ScrollOutputBottom, None);
    let rebuild = app.rebuild.as_ref().expect("rebuild state");
    let expected = rebuild.jobs[0]
        .output
        .len()
        .saturating_sub(rebuild.viewport_height as usize);
    assert_eq!(rebuild.scroll_y as usize, expected);
    assert!(rebuild.auto_scroll, "end should re-enable auto-follow");

    terminal
        .draw(|f| ui::draw(f, &mut app, &args))
        .expect("draw after end");
    let lines = rendered_lines(&terminal);
    assert!(lines.iter().any(|line| line.contains("abcdefghij")));

    app::update_with_services(&mut app, Msg::ScrollOutputRight, None);
    assert!(lines.iter().any(|line| line.contains("abcdefghij")));

    app::update_with_services(&mut app, Msg::ScrollOutputRight, None);
    terminal
        .draw(|f| ui::draw(f, &mut app, &args))
        .expect("draw after right scroll");
    let lines_after = rendered_lines(&terminal);
    let content_line = lines_after
        .iter()
        .find(|line| line.contains("efghij"))
        .expect("scrolled line present");
    assert!(!content_line.contains("abcd"));
}

fn app_with_scrollable_output(args: &Args) -> App {
    let mut app = App::new();
    app.state = UiState::Rebuilding;
    let mut job = RebuildJob::new(
        "img".into(),
        Some("container".into()),
        PathBuf::from("."),
        PathBuf::from("."),
    );
    for _ in 0..20 {
        job.push_output(
            OutputStream::Stdout,
            "abcdefghij".into(),
            args.rebuild_view_line_buffer_max,
        );
    }
    job.status = RebuildStatus::Running;

    let mut rebuild = RebuildState::new(vec![job], args.rebuild_view_line_buffer_max);
    rebuild.auto_scroll = false;
    rebuild.scroll_y = 0;
    rebuild.viewport_height = 0;
    app.rebuild = Some(rebuild);
    app
}

fn rendered_lines(terminal: &Terminal<TestBackend>) -> Vec<String> {
    let buffer = terminal.backend().buffer();
    let mut lines = Vec::new();
    for y in 0..buffer.area.height {
        let mut line = String::new();
        for x in 0..buffer.area.width {
            if let Some(cell) = buffer.cell((x, y)) {
                line.push_str(cell.symbol());
            }
        }
        lines.push(line);
    }
    lines
}
