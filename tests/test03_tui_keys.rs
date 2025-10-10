use podman_compose_mgr::Args;
use podman_compose_mgr::tui::app::{App, ItemRow, UiState};
use podman_compose_mgr::tui::ui;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[test]
fn keys_overlay_is_drawn_with_labels() {
    // Prepare minimal app state
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
    }];

    // Minimal args
    let args = Args {
        path: std::path::PathBuf::from("."),
        verbose: 0,
        exclude_path_patterns: vec![],
        include_path_patterns: vec![],
        build_args: vec![],
        temp_file_path: std::env::temp_dir(),
        tui: true,
    };

    // Render at a reasonable size
    let backend = TestBackend::new(100, 12);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| ui::draw(f, &app, &args)).expect("draw");

    // Flatten buffer
    let buf = terminal.backend_mut().buffer().clone();
    let mut all = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let cell = buf.cell((x, y)).expect("cell exists");
            all.push_str(cell.symbol());
        }
        all.push('\n');
    }

    // Assert overlay title and key hints exist. The overlay width is capped,
    // so check for tokens that appear in the left portion.
    assert!(all.contains("Keys"));
    assert!(all.contains("↑/↓"));
    assert!(all.contains("scroll"));
    assert!(all.contains("←/→"));
    assert!(all.contains("details"));
    assert!(all.contains("[space]"));
    assert!(all.contains("select"));
    assert!(all.contains("quit"));
}
