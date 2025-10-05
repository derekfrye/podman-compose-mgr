use podman_compose_mgr::tui::app::{App, UiState, ViewMode};
use podman_compose_mgr::tui::discover::DiscoveredImage;
use podman_compose_mgr::Args;
use crossterm::event::KeyCode;

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
        },
        DiscoveredImage {
            image: "djf/rusty-golf".into(),
            container: Some("golf".into()),
            source_dir: app.root_path.join("a"),
        },
    ];

    // Open folder view via modal simulation: 'v', Down, Down, Enter
    app.on_key(KeyCode::Char('v'));
    app.on_key(KeyCode::Down); // ByImage
    app.on_key(KeyCode::Down); // ByFolderThenImage
    app.on_key(KeyCode::Enter);

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

