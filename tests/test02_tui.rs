use std::collections::HashSet;

use podman_compose_mgr::Args;
use podman_compose_mgr::args::types::{OneShotArgs, REBUILD_VIEW_LINE_BUFFER_DEFAULT, TuiArgs};

#[test]
fn discovery_finds_expected_images_in_test1() {
    let args = Args {
        path: std::path::PathBuf::from("tests/test1"),
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
    let discovery = std::sync::Arc::new(podman_compose_mgr::infra::discovery_adapter::FsDiscovery);
    let podman = std::sync::Arc::new(podman_compose_mgr::infra::podman_adapter::PodmanCli);
    let core = podman_compose_mgr::app::AppCore::new(discovery, podman);
    let found = core
        .scan_images(
            args.path.clone(),
            args.include_path_patterns.clone(),
            args.exclude_path_patterns.clone(),
        )
        .unwrap();

    let got: HashSet<(String, Option<String>)> = found
        .images
        .iter()
        .map(|d| (d.image.clone(), d.container.clone()))
        .collect();

    let expected: HashSet<(String, Option<String>)> = [
        ("djf/rusty-golf".to_string(), Some("golf".to_string())),
        ("djf/squid".to_string(), Some("squid".to_string())),
        (
            "pihole/pihole:latest".to_string(),
            Some("pihole".to_string()),
        ),
        ("djf/rusty-golf_unq".to_string(), Some("golf".to_string())),
        (
            "djf/rusty-golf-from-cont-file".to_string(),
            Some("golf".to_string()),
        ),
        (
            "pihole/pihole-from-cont:latest".to_string(),
            Some("pihole".to_string()),
        ),
        ("djf/squid-from-cont".to_string(), Some("squid".to_string())),
    ]
    .into_iter()
    .collect();

    assert_eq!(got, expected);
}

// Basic UI snapshot: render a small table and assert key content exists in buffer
#[test]
fn ui_snapshot_renders_table_with_rows() {
    use podman_compose_mgr::tui::app::{App, ItemRow, UiState};
    use podman_compose_mgr::tui::ui;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    // Build app state
    let mut app = App::new();
    app.state = UiState::Ready;
    app.rows = vec![
        ItemRow {
            checked: false,
            image: "djf/rusty-golf".into(),
            container: Some("golf".into()),
            source_dir: std::path::PathBuf::from("."),
            entry_path: Some(std::path::PathBuf::from("tests/test1/docker-compose.yml")),
            expanded: false,
            details: Vec::new(),
            is_dir: false,
            dir_name: None,
            dockerfile_extra: None,
        },
        ItemRow {
            checked: true,
            image: "djf/squid".into(),
            container: Some("squid".into()),
            source_dir: std::path::PathBuf::from("."),
            entry_path: Some(std::path::PathBuf::from("tests/test1/docker-compose.yml")),
            expanded: false,
            details: Vec::new(),
            is_dir: false,
            dir_name: None,
            dockerfile_extra: None,
        },
    ];

    // Minimal args for draw
    let args = Args {
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

    let width: u16 = 60;
    let height: u16 = 10;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|f| ui::draw(f, &mut app, &args))
        .expect("draw");

    // Access buffer
    let buf = terminal.backend_mut().buffer().clone();

    // Flatten lines
    let mut all = String::new();
    for y in 0..height {
        for x in 0..width {
            let cell = buf
                .cell((x, y))
                .expect("cell must exist in test backend buffer");
            all.push_str(cell.symbol());
        }
        all.push('\n');
    }

    // Check for key substrings
    assert!(all.contains("Images"));
}
