use crossterm::event::KeyCode;
use podman_compose_mgr::Args;
use podman_compose_mgr::args::types::{OneShotArgs, REBUILD_VIEW_LINE_BUFFER_DEFAULT, TuiArgs};
use podman_compose_mgr::tui::app::{
    self, App, ItemRow, Msg, OutputStream, RebuildJob, RebuildJobSpec, RebuildState, RebuildStatus,
    SearchDirection, SearchState, UiState,
};
use podman_compose_mgr::tui::ui;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use std::path::PathBuf;

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
        dockerfile_extra: None,
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
    assert!(all.contains("x/<space>"));
    assert!(all.contains("select"));
    assert!(all.contains("r rebuild selected images"));
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
            dockerfile_extra: None,
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

#[test]
fn rebuild_view_slash_starts_forward_search() {
    let mut app = App::new();
    app.state = UiState::Rebuilding;
    let job = RebuildJob::new(
        "img".into(),
        Some("container".into()),
        PathBuf::from("."),
        PathBuf::from("."),
    );
    app.rebuild = Some(RebuildState::new(
        vec![job],
        REBUILD_VIEW_LINE_BUFFER_DEFAULT,
    ));

    assert!(matches!(
        podman_compose_mgr::tui::app::map_keycode_to_msg(&app, KeyCode::Char('/')),
        Some(Msg::StartSearchForward)
    ));
}

#[test]
fn rebuild_view_search_char_routes_to_input_when_editing() {
    let mut app = App::new();
    app.state = UiState::Rebuilding;
    let job = RebuildJob::new(
        "img".into(),
        Some("container".into()),
        PathBuf::from("."),
        PathBuf::from("."),
    );
    let mut rebuild = RebuildState::new(vec![job], REBUILD_VIEW_LINE_BUFFER_DEFAULT);
    rebuild.search = Some(SearchState::new(SearchDirection::Forward));
    app.rebuild = Some(rebuild);

    assert!(matches!(
        podman_compose_mgr::tui::app::map_keycode_to_msg(&app, KeyCode::Char('a')),
        Some(Msg::SearchInput('a'))
    ));
}

#[test]
fn rebuild_view_navigates_matches_when_not_editing() {
    let mut app = App::new();
    app.state = UiState::Rebuilding;
    let job = RebuildJob::new(
        "img".into(),
        Some("container".into()),
        PathBuf::from("."),
        PathBuf::from("."),
    );
    let mut rebuild = RebuildState::new(vec![job], REBUILD_VIEW_LINE_BUFFER_DEFAULT);
    let mut search = SearchState::new(SearchDirection::Forward);
    search.query = "abc".into();
    search.editing = false;
    rebuild.search = Some(search);
    app.rebuild = Some(rebuild);

    assert!(matches!(
        podman_compose_mgr::tui::app::map_keycode_to_msg(&app, KeyCode::Char('n')),
        Some(Msg::SearchNext)
    ));
    assert!(matches!(
        podman_compose_mgr::tui::app::map_keycode_to_msg(&app, KeyCode::Char('N')),
        Some(Msg::SearchPrev)
    ));
}

#[test]
fn exit_rebuild_keeps_rebuild_state() {
    let mut app = App::new();
    app.state = UiState::Rebuilding;
    let job = RebuildJob::new(
        "img".into(),
        Some("container".into()),
        PathBuf::from("."),
        PathBuf::from("."),
    );
    app.rebuild = Some(RebuildState::new(
        vec![job],
        REBUILD_VIEW_LINE_BUFFER_DEFAULT,
    ));

    podman_compose_mgr::tui::app::update_with_services(&mut app, Msg::ExitRebuild, None);

    assert_eq!(app.state, UiState::Ready);
    assert!(
        app.rebuild.is_some(),
        "rebuild state should be retained after exiting the view"
    );
}

#[test]
fn ready_view_j_reopens_rebuild() {
    let mut app = App::new();
    app.state = UiState::Ready;
    let job = RebuildJob::new("img".into(), None, PathBuf::from("."), PathBuf::from("."));
    app.rebuild = Some(RebuildState::new(
        vec![job],
        REBUILD_VIEW_LINE_BUFFER_DEFAULT,
    ));

    assert!(matches!(
        podman_compose_mgr::tui::app::map_keycode_to_msg(&app, KeyCode::Char('j')),
        Some(Msg::ShowRebuild)
    ));
    assert!(matches!(
        podman_compose_mgr::tui::app::map_keycode_to_msg(&app, KeyCode::Char('J')),
        Some(Msg::ShowRebuild)
    ));

    podman_compose_mgr::tui::app::update_with_services(&mut app, Msg::ShowRebuild, None);

    assert_eq!(app.state, UiState::Rebuilding);
}

#[test]
fn rebuild_session_created_appends_jobs() {
    let mut app = App::new();
    let job_a = RebuildJobSpec {
        image: "img-a".into(),
        container: Some("container-a".into()),
        entry_path: PathBuf::from("tests/test1/docker-compose.yml"),
        source_dir: PathBuf::from("tests/test1"),
    };
    let job_b = RebuildJobSpec {
        image: "img-b".into(),
        container: Some("container-b".into()),
        entry_path: PathBuf::from("tests/test1/docker-compose.yml"),
        source_dir: PathBuf::from("tests/test1"),
    };

    podman_compose_mgr::tui::app::update_with_services(
        &mut app,
        Msg::RebuildSessionCreated {
            jobs: vec![job_a.clone()],
        },
        None,
    );

    let rebuild = app.rebuild.as_ref().expect("rebuild state exists");
    assert_eq!(rebuild.jobs.len(), 1);

    podman_compose_mgr::tui::app::update_with_services(
        &mut app,
        Msg::RebuildSessionCreated {
            jobs: vec![job_b.clone()],
        },
        None,
    );

    let rebuild = app.rebuild.as_ref().expect("rebuild state exists");
    assert_eq!(rebuild.jobs.len(), 2);
    assert_eq!(rebuild.jobs[0].image, job_a.image);
    assert_eq!(rebuild.jobs[1].image, job_b.image);
    assert_eq!(app.state, UiState::Rebuilding);
}

#[test]
fn rebuild_home_and_end_keys_adjust_scroll_and_auto_follow() {
    let args = Args::default();
    let mut app = App::new();
    app.state = UiState::Rebuilding;

    let mut job = RebuildJob::new(
        "img".into(),
        Some("container".into()),
        PathBuf::from("."),
        PathBuf::from("."),
    );
    for _ in 0..20 {
        job.push_output(
            OutputStream::Stdout,
            "abcdefghij".into(),
            args.rebuild_view_line_buffer_max,
        );
    }
    job.status = RebuildStatus::Running;

    let mut rebuild = RebuildState::new(vec![job], args.rebuild_view_line_buffer_max);
    rebuild.auto_scroll = false;
    rebuild.scroll_y = 0;
    rebuild.viewport_height = 0;
    app.rebuild = Some(rebuild);

    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|f| ui::draw(f, &mut app, &args))
        .expect("initial draw");

    let rebuild = app.rebuild.as_mut().expect("rebuild state");
    rebuild.auto_scroll = false;
    rebuild.scroll_y = 0;

    app::update_with_services(&mut app, Msg::ScrollOutputTop, None);
    let rebuild = app.rebuild.as_ref().expect("rebuild state");
    assert_eq!(rebuild.scroll_y, 0);
    assert!(!rebuild.auto_scroll, "home should disable auto-follow");

    app::update_with_services(&mut app, Msg::ScrollOutputBottom, None);
    let rebuild = app.rebuild.as_ref().expect("rebuild state");
    let viewport = rebuild.viewport_height as usize;
    let expected = rebuild.jobs[0].output.len().saturating_sub(viewport);
    assert_eq!(rebuild.scroll_y as usize, expected);
    assert!(rebuild.auto_scroll, "end should re-enable auto-follow");

    terminal
        .draw(|f| ui::draw(f, &mut app, &args))
        .expect("draw after end");

    let buffer = terminal.backend().buffer().clone();
    let mut lines: Vec<String> = Vec::new();
    for y in 0..buffer.area.height {
        let mut line = String::new();
        for x in 0..buffer.area.width {
            if let Some(cell) = buffer.cell((x, y)) {
                line.push_str(cell.symbol());
            }
        }
        lines.push(line);
    }
    assert!(lines.iter().any(|line| line.contains("abcdefghij")));

    app::update_with_services(&mut app, Msg::ScrollOutputRight, None);
    assert!(lines.iter().any(|line| line.contains("abcdefghij")));

    app::update_with_services(&mut app, Msg::ScrollOutputRight, None);
    terminal
        .draw(|f| ui::draw(f, &mut app, &args))
        .expect("draw after right scroll");

    let buffer = terminal.backend().buffer().clone();
    let mut lines_after = Vec::new();
    for y in 0..buffer.area.height {
        let mut line = String::new();
        for x in 0..buffer.area.width {
            if let Some(cell) = buffer.cell((x, y)) {
                line.push_str(cell.symbol());
            }
        }
        lines_after.push(line);
    }
    let content_line = lines_after
        .iter()
        .find(|line| line.contains("efghij"))
        .expect("scrolled line present");
    assert!(!content_line.contains("abcd"));
}
