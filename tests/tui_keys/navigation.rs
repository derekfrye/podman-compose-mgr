use crossterm::event::KeyCode;
use podman_compose_mgr::tui::app::{self, App, ItemRow, Msg, UiState};

#[test]
fn ready_view_escape_quits_application() {
    let mut app = App::new();
    app.state = UiState::Ready;
    assert!(matches!(
        app::map_keycode_to_msg(&app, KeyCode::Esc),
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
            dockerfile_extra: None,
            makefile_extra: None,
        })
        .collect();

    assert!(matches!(
        app::map_keycode_to_msg(&app, KeyCode::PageDown),
        Some(Msg::MovePageDown)
    ));
    assert!(matches!(
        app::map_keycode_to_msg(&app, KeyCode::PageUp),
        Some(Msg::MovePageUp)
    ));

    app::update_with_services(&mut app, Msg::MovePageDown, None);
    assert_eq!(
        app.selected, 12,
        "page down should move roughly one screenful"
    );

    app::update_with_services(&mut app, Msg::MovePageUp, None);
    assert_eq!(app.selected, 0, "page up should return toward the start");

    app.selected = app.rows.len() - 1;
    app::update_with_services(&mut app, Msg::MovePageDown, None);
    assert_eq!(
        app.selected,
        app.rows.len() - 1,
        "page down at end clamps to final row"
    );
}
