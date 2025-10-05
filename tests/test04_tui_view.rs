use std::collections::HashSet;

use crossterm::event::KeyCode;
use podman_compose_mgr::tui::app::{App, ViewMode};
use podman_compose_mgr::tui::discover::scan_images;
use podman_compose_mgr::utils::log_utils::Logger;
use podman_compose_mgr::Args;

#[test]
fn change_view_to_by_image_dedupes_images() {
    // Discover rows from fixtures
    let args = Args {
        path: std::path::PathBuf::from("tests/test1"),
        verbose: 0,
        exclude_path_patterns: vec![],
        include_path_patterns: vec![],
        build_args: vec![],
        temp_file_path: std::env::temp_dir(),
        tui: true,
    };
    let logger = Logger::new(0);
    let mut discovered = scan_images(&args, &logger);
    // Inject a duplicate image with different container if possible
    if let Some(first) = discovered.first().cloned() {
        let mut dup = first.clone();
        dup.container = Some("duplicate-container".to_string());
        discovered.push(dup);
    }
    let unique_images: HashSet<String> = discovered.iter().map(|d| d.image.clone()).collect();

    // Seed app and switch view via modal flow: 'v', Down, Enter
    let mut app = App::new();
    app.state = podman_compose_mgr::tui::app::UiState::Ready;
    app.all_items = discovered;

    // Open modal
    app.on_key(KeyCode::Char('v'));
    // Move to "List by image"
    app.on_key(KeyCode::Down);
    // Select
    app.on_key(KeyCode::Enter);

    assert_eq!(app.view_mode, ViewMode::ByImage);

    // After switch, rows should be unique by image and containers cleared
    let images_after: HashSet<String> = app.rows.iter().map(|r| r.image.clone()).collect();
    assert_eq!(app.rows.len(), images_after.len());
    assert_eq!(images_after.len(), unique_images.len());
    assert!(app.rows.iter().all(|r| r.container.is_none()));
}
