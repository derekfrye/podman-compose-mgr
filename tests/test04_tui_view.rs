use std::collections::HashSet;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use podman_compose_mgr::Args;
use podman_compose_mgr::tui::app::{self, App, Msg, ViewMode};
use podman_compose_mgr::tui::ui;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

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
        podman_bin: None,
        tui: true,
        tui_rebuild_all: false,
    };
    let discovery = std::sync::Arc::new(podman_compose_mgr::infra::discovery_adapter::FsDiscovery);
    let podman = std::sync::Arc::new(podman_compose_mgr::infra::podman_adapter::PodmanCli);
    let core = podman_compose_mgr::app::AppCore::new(discovery, podman);
    let mut discovered = core
        .scan_images(
            args.path.clone(),
            args.include_path_patterns.clone(),
            args.exclude_path_patterns.clone(),
        )
        .unwrap();
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
    app::update_with_services(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE)),
        None,
    );
    // Move to "List by image"
    app::update_with_services(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
        None,
    );
    // Select
    app::update_with_services(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        None,
    );

    assert_eq!(app.view_mode, ViewMode::ByImage);

    // After switch, rows should be unique by image and containers cleared
    let images_after: HashSet<String> = app.rows.iter().map(|r| r.image.clone()).collect();
    assert_eq!(app.rows.len(), images_after.len());
    assert_eq!(images_after.len(), unique_images.len());
    assert!(app.rows.iter().all(|r| r.container.is_none()));

    let backend = TestBackend::new(100, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|f| ui::draw(f, &mut app, &args))
        .expect("draw by-image view");
    let buffer = terminal.backend_mut().buffer().clone();
    let mut rendered = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = buffer.cell((x, y)).unwrap();
            rendered.push_str(cell.symbol());
        }
        rendered.push('\n');
    }
    assert!(
        rendered.contains("Container(s)"),
        "header should include container column"
    );
    assert!(
        rendered.contains("duplicate-container"),
        "aggregated container list should include all container names"
    );
}
