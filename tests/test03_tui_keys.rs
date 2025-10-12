use crossterm::event::KeyCode;
use podman_compose_mgr::Args;
use podman_compose_mgr::tui::app::{App, ItemRow, Msg, UiState};
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
        podman_bin: None,
        tui: true,
        tui_rebuild_all: false,
    };

    // Render at a reasonable size
    let backend = TestBackend::new(100, 12);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|f| ui::draw(f, &mut app, &args))
        .expect("draw");

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

#[test]
fn rebuild_view_q_quits_application() {
    let mut app = App::new();
    app.state = UiState::Rebuilding;
    assert!(matches!(
        podman_compose_mgr::tui::app::map_keycode_to_msg(&app, KeyCode::Char('q')),
        Some(Msg::Quit)
    ));
    assert!(matches!(
        podman_compose_mgr::tui::app::map_keycode_to_msg(&app, KeyCode::Char('Q')),
        Some(Msg::Quit)
    ));
    assert!(matches!(
        podman_compose_mgr::tui::app::map_keycode_to_msg(&app, KeyCode::Esc),
        Some(Msg::ExitRebuild)
    ));
}

#[test]
fn ready_view_escape_quits_application() {
    let mut app = App::new();
    app.state = UiState::Ready;
    assert!(matches!(
        podman_compose_mgr::tui::app::map_keycode_to_msg(&app, KeyCode::Esc),
        Some(Msg::Quit)
    ));
}

#[test]
fn page_navigation_moves_by_screenful() {
    let mut app = App::new();
    app.state = UiState::Ready;
    app.rows = (0..30)
        .map(|idx| ItemRow {
            checked: false,
            image: format!("img-{idx}"),
            container: Some(format!("container-{idx}")),
            source_dir: std::path::PathBuf::from("."),
            entry_path: Some(std::path::PathBuf::from(format!(
                "tests/test1/entry-{idx}.yml"
            ))),
            expanded: false,
            details: Vec::new(),
            is_dir: false,
            dir_name: None,
        })
        .collect();

    assert!(matches!(
        podman_compose_mgr::tui::app::map_keycode_to_msg(&app, KeyCode::PageDown),
        Some(Msg::MovePageDown)
    ));
    assert!(matches!(
        podman_compose_mgr::tui::app::map_keycode_to_msg(&app, KeyCode::PageUp),
        Some(Msg::MovePageUp)
    ));

    podman_compose_mgr::tui::app::update_with_services(&mut app, Msg::MovePageDown, None);
    assert_eq!(
        app.selected, 12,
        "page down should move roughly one screenful"
    );

    podman_compose_mgr::tui::app::update_with_services(&mut app, Msg::MovePageUp, None);
    assert_eq!(app.selected, 0, "page up should return toward the start");

    app.selected = app.rows.len() - 1;
    podman_compose_mgr::tui::app::update_with_services(&mut app, Msg::MovePageDown, None);
    assert_eq!(
        app.selected,
        app.rows.len() - 1,
        "page down at end clamps to final row"
    );
}
