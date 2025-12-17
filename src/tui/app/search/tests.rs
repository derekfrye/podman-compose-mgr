use super::*;
use crate::tui::app::OutputStream;
use crate::tui::app::state::{RebuildJob, RebuildOutputLine};
use std::collections::VecDeque;

fn job_with_lines(lines: &[&str]) -> RebuildJob {
    let mut output = VecDeque::new();
    for line in lines {
        output.push_back(RebuildOutputLine {
            stream: OutputStream::Stdout,
            text: (*line).to_string(),
        });
    }
    RebuildJob {
        image: "img".into(),
        container: None,
        entry_path: std::path::PathBuf::from("."),
        source_dir: std::path::PathBuf::from("."),
        status: crate::tui::app::RebuildStatus::Running,
        output,
        error: None,
    }
}

#[test]
fn recompute_matches_tracks_lines_and_active_forward() {
    let job = job_with_lines(&["alpha", "beta", "alphabet"]);
    let mut search = SearchState::new(SearchDirection::Forward);
    search.query = "alpha".into();
    search.editing = false;
    search.recompute_matches(&job, 0);

    assert_eq!(search.matches.len(), 2);
    assert_eq!(search.matches[0].line, 0);
    assert_eq!(search.matches[1].line, 2);
    assert_eq!(search.active, Some(0));
    assert!(search.error.is_none());
}

#[test]
fn recompute_matches_tracks_backward_direction() {
    let job = job_with_lines(&["a", "b", "a"]);
    let mut search = SearchState::new(SearchDirection::Backward);
    search.query = "a".into();
    search.editing = false;
    search.recompute_matches(&job, 1);

    assert_eq!(search.matches.len(), 2);
    assert_eq!(search.active, Some(0));
    assert_eq!(search.matches[0].line, 0);
}

#[test]
fn advance_wraps_over_matches() {
    let job = job_with_lines(&["foo", "foo"]);
    let mut search = SearchState::new(SearchDirection::Forward);
    search.query = "foo".into();
    search.editing = false;
    search.recompute_matches(&job, 0);

    search.advance_next();
    assert_eq!(search.active, Some(1));
    search.advance_next();
    assert_eq!(search.active, Some(0));

    search.advance_prev();
    assert_eq!(search.active, Some(1));
}
