use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use podman_compose_mgr::Args;
use podman_compose_mgr::domain::DiscoveredImage;
use podman_compose_mgr::tui::app::{self, App, Msg, UiState, ViewMode};
use podman_compose_mgr::tui::ui;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[test]
fn view_modal_shows_three_options_and_selects_folder_view() {
    let mut app = App::new();
    app.state = UiState::Ready;
    app.title = "Podman Compose Manager".into();
    app.root_path = std::path::PathBuf::from("tests/test1");
    app.all_items = vec![
        DiscoveredImage {
            image: "imgA".into(),
            container: Some("cA".into()),
            source_dir: app.root_path.join("image1"),
            entry_path: app.root_path.join("image1/docker-compose.yml"),
        },
        DiscoveredImage {
            image: "imgB".into(),
            container: Some("cB".into()),
            source_dir: app.root_path.join("image2"),
            entry_path: app.root_path.join("image2/docker-compose.yml"),
        },
    ];
    // rows get built after selection; no-op here

    let args = Args {
        path: app.root_path.clone(),
        verbose: 0,
        exclude_path_patterns: vec![],
        include_path_patterns: vec![],
        build_args: vec![],
        temp_file_path: std::env::temp_dir(),
        tui: true,
    };

    let backend = TestBackend::new(80, 18);
    let mut terminal = Terminal::new(backend).expect("terminal");

    // Open modal (v), then move to 3rd item (Down, Down), then Enter
    app::update_with_services(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE)),
        None,
    );
    terminal.draw(|f| ui::draw(f, &app, &args)).unwrap();

    // Buffer should include the third option line
    let buf = terminal.backend_mut().buffer().clone();
    let mut all = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let cell = buf.cell((x, y)).unwrap();
            all.push_str(cell.symbol());
        }
        all.push('\n');
    }
    assert!(all.contains("List by folder, then image"));

    // Navigate to third option and select it
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

    // Draw and ensure the header uses Name and title shows Folder:
    terminal.draw(|f| ui::draw(f, &app, &args)).unwrap();
    let buf2 = terminal.backend_mut().buffer().clone();
    let mut all2 = String::new();
    for y in 0..buf2.area.height {
        for x in 0..buf2.area.width {
            let cell = buf2.cell((x, y)).unwrap();
            all2.push_str(cell.symbol());
        }
        all2.push('\n');
    }
    assert!(all2.contains("Name"));
    assert!(all2.contains("Folder:"));
}
