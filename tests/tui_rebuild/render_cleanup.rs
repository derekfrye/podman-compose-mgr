use podman_compose_mgr::Args;
use podman_compose_mgr::tui::app::{
    self, App, Msg, OutputStream, RebuildJob, RebuildState, RebuildStatus, UiState,
};
use podman_compose_mgr::tui::ui;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

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

    let mut terminal = Terminal::new(TestBackend::new(80, 20)).expect("terminal");
    send_output(&mut app, "X".repeat(60));
    terminal
        .draw(|f| ui::draw(f, &mut app, &args))
        .expect("first draw");

    send_output(&mut app, "short".to_string());
    terminal
        .draw(|f| ui::draw(f, &mut app, &args))
        .expect("second draw");

    let short_line = rendered_lines(&terminal)
        .into_iter()
        .find(|line| line.contains("short"))
        .expect("short line present");
    let short_pos = short_line.find("short").expect("short text located");
    let after_short = short_pos + "short".len();
    let right_border = short_line[after_short..]
        .find('│')
        .map_or_else(|| short_line.len(), |idx| after_short + idx);
    let tail = &short_line[after_short..right_border];
    assert!(
        tail.chars().all(|c| c == ' '),
        "residual characters detected"
    );
}

fn send_output(app: &mut App, chunk: String) {
    app::update_with_services(
        app,
        Msg::RebuildJobOutput {
            job_idx: 0,
            chunk,
            stream: OutputStream::Stdout,
        },
        None,
    );
}

fn rendered_lines(terminal: &Terminal<TestBackend>) -> Vec<String> {
    let buffer = terminal.backend().buffer();
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
    rendered_lines
}
