use crossterm::event::KeyCode;
use podman_compose_mgr::args::types::REBUILD_VIEW_LINE_BUFFER_DEFAULT;
use podman_compose_mgr::tui::app::{
    self, App, Msg, RebuildJob, RebuildJobSpec, RebuildState, SearchDirection, SearchState, UiState,
};
use std::path::PathBuf;

fn rebuild_app() -> App {
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
    app
}

#[test]
fn rebuild_view_q_quits_application() {
    let app = rebuild_app();
    assert!(matches!(
        app::map_keycode_to_msg(&app, KeyCode::Char('q')),
        Some(Msg::Quit)
    ));
    assert!(matches!(
        app::map_keycode_to_msg(&app, KeyCode::Char('Q')),
        Some(Msg::Quit)
    ));
    assert!(matches!(
        app::map_keycode_to_msg(&app, KeyCode::Esc),
        Some(Msg::ExitRebuild)
    ));
}

#[test]
fn rebuild_view_slash_starts_forward_search() {
    let app = rebuild_app();
    assert!(matches!(
        app::map_keycode_to_msg(&app, KeyCode::Char('/')),
        Some(Msg::StartSearchForward)
    ));
}

#[test]
fn rebuild_view_search_char_routes_to_input_when_editing() {
    let mut app = rebuild_app();
    let mut rebuild = app.rebuild.take().expect("rebuild state");
    rebuild.search = Some(SearchState::new(SearchDirection::Forward));
    app.rebuild = Some(rebuild);

    assert!(matches!(
        app::map_keycode_to_msg(&app, KeyCode::Char('a')),
        Some(Msg::SearchInput('a'))
    ));
}

#[test]
fn rebuild_view_navigates_matches_when_not_editing() {
    let mut app = rebuild_app();
    let mut rebuild = app.rebuild.take().expect("rebuild state");
    let mut search = SearchState::new(SearchDirection::Forward);
    search.query = "abc".into();
    search.editing = false;
    rebuild.search = Some(search);
    app.rebuild = Some(rebuild);

    assert!(matches!(
        app::map_keycode_to_msg(&app, KeyCode::Char('n')),
        Some(Msg::SearchNext)
    ));
    assert!(matches!(
        app::map_keycode_to_msg(&app, KeyCode::Char('N')),
        Some(Msg::SearchPrev)
    ));
}

#[test]
fn exit_rebuild_keeps_rebuild_state() {
    let mut app = rebuild_app();
    app::update_with_services(&mut app, Msg::ExitRebuild, None);

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
        app::map_keycode_to_msg(&app, KeyCode::Char('j')),
        Some(Msg::ShowRebuild)
    ));
    assert!(matches!(
        app::map_keycode_to_msg(&app, KeyCode::Char('J')),
        Some(Msg::ShowRebuild)
    ));

    app::update_with_services(&mut app, Msg::ShowRebuild, None);
    assert_eq!(app.state, UiState::Rebuilding);
}

#[test]
fn rebuild_session_created_appends_jobs() {
    let mut app = App::new();
    let job_a = rebuild_job_spec("img-a", "container-a");
    let job_b = rebuild_job_spec("img-b", "container-b");

    app::update_with_services(
        &mut app,
        Msg::RebuildSessionCreated {
            jobs: vec![job_a.clone()],
        },
        None,
    );
    assert_eq!(app.rebuild.as_ref().expect("rebuild state").jobs.len(), 1);

    app::update_with_services(
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

fn rebuild_job_spec(image: &str, container: &str) -> RebuildJobSpec {
    RebuildJobSpec {
        image: image.into(),
        container: Some(container.into()),
        entry_path: PathBuf::from("tests/test1/docker-compose.yml"),
        source_dir: PathBuf::from("tests/test1"),
        make_target: None,
    }
}
