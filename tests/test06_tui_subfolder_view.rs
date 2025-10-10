use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use podman_compose_mgr::domain::DiscoveredImage;
use podman_compose_mgr::tui::app::{self, App, Msg, UiState, ViewMode};

#[test]
fn folder_view_lists_subfolders_even_with_duplicate_images() {
    let mut app = App::new();
    app.state = UiState::Ready;
    app.root_path = std::path::PathBuf::from("tests/test1");

    // Two discovered entries with identical image/container but different folders
    app.all_items = vec![
        DiscoveredImage {
            image: "djf/rusty-golf".into(),
            container: Some("golf".into()),
            source_dir: app.root_path.join("image1"),
            entry_path: app.root_path.join("image1/docker-compose.yml"),
        },
        DiscoveredImage {
            image: "djf/rusty-golf".into(),
            container: Some("golf".into()),
            source_dir: app.root_path.join("a"),
            entry_path: app.root_path.join("a/docker-compose.yml"),
        },
    ];

    // Open folder view via modal simulation: 'v', Down, Down, Enter
    app::update_with_services(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE)),
        None,
    );
    app::update_with_services(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
        None,
    );
    app::update_with_services(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
        None,
    );
    app::update_with_services(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        None,
    );

    assert_eq!(app.view_mode, ViewMode::ByFolderThenImage);

    // We expect to see both subfolders 'a' and 'image1' listed
    let names: Vec<String> = app
        .rows
        .iter()
        .filter(|r| r.is_dir)
        .filter_map(|r| r.dir_name.clone())
        .collect();

    assert!(names.contains(&"a".to_string()));
    assert!(names.contains(&"image1".to_string()));
}
