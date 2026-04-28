use podman_compose_mgr::Args;
use podman_compose_mgr::args::types::{OneShotArgs, REBUILD_VIEW_LINE_BUFFER_DEFAULT, TuiArgs};
use podman_compose_mgr::tui::app::{App, ItemRow, UiState};
use podman_compose_mgr::tui::ui;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[test]
fn keys_overlay_is_drawn_with_labels() {
    let mut app = App::new();
    app.state = UiState::Ready;
    app.rows = vec![ItemRow {
        checked: false,
        image: "img".into(),
        container: Some("c".into()),
        source_dir: std::path::PathBuf::from("."),
        entry_path: Some(std::path::PathBuf::from("tests/test1/docker-compose.yml")),
        expanded: false,
        details: Vec::new(),
        is_dir: false,
        dir_name: None,
        dockerfile_extra: None,
        makefile_extra: None,
    }];

    let args = Args {
        config_toml: None,
        path: std::path::PathBuf::from("."),
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
        rebuild_view_line_buffer_max: REBUILD_VIEW_LINE_BUFFER_DEFAULT,
        tui_simulate_podman_input_json: None,
        tui_simulate: None,
    };

    let backend = TestBackend::new(100, 12);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|f| ui::draw(f, &mut app, &args))
        .expect("draw");

    let buf = terminal.backend_mut().buffer().clone();
    let mut all = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let cell = buf.cell((x, y)).expect("cell exists");
            all.push_str(cell.symbol());
        }
        all.push('\n');
    }

    assert!(all.contains("Keys"));
    assert!(all.contains("↑/↓"));
    assert!(all.contains("scroll"));
    assert!(all.contains("←/→"));
    assert!(all.contains("details"));
    assert!(all.contains("x/<space>"));
    assert!(all.contains("select"));
    assert!(all.contains("r rebuild selected images"));
    assert!(all.contains("quit"));
}
