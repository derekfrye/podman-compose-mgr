use crate::tui::app::search::{SearchDirection, SearchState};
use crate::tui::app::state::{App, RebuildState, UiState};

pub(super) fn handle_search_start(app: &mut App, direction: SearchDirection) {
    if app.state != UiState::Rebuilding {
        return;
    }
    let Some(rebuild) = app.rebuild.as_mut() else {
        return;
    };
    let search = rebuild
        .search
        .get_or_insert_with(|| SearchState::new(direction));
    search.set_direction(direction);
    search.editing = true;
    search.error = None;
    if search.has_query() {
        if let Some(job) = rebuild.jobs.get(rebuild.active_idx) {
            let baseline = usize::from(rebuild.scroll_y);
            search.recompute_matches(job, baseline);
        }
    } else {
        search.clear_results();
    }
}

pub(super) fn handle_search_input(app: &mut App, ch: char) {
    if app.state != UiState::Rebuilding || ch.is_control() {
        return;
    }
    let Some(rebuild) = app.rebuild.as_mut() else {
        return;
    };
    let Some(search) = rebuild.search.as_mut() else {
        return;
    };
    if !search.editing {
        return;
    }
    search.push_char(ch);
    search.error = None;
    if let Some(job) = rebuild.jobs.get(rebuild.active_idx) {
        let baseline = usize::from(rebuild.scroll_y);
        search.recompute_matches(job, baseline);
    }
}

pub(super) fn handle_search_backspace(app: &mut App) {
    if app.state != UiState::Rebuilding {
        return;
    }
    let Some(rebuild) = app.rebuild.as_mut() else {
        return;
    };
    let Some(search) = rebuild.search.as_mut() else {
        return;
    };
    if !search.editing {
        return;
    }
    search.pop_char();
    search.error = None;
    if let Some(job) = rebuild.jobs.get(rebuild.active_idx) {
        let baseline = usize::from(rebuild.scroll_y);
        search.recompute_matches(job, baseline);
    } else {
        search.clear_results();
    }
}

pub(super) fn handle_search_submit(app: &mut App) {
    if app.state != UiState::Rebuilding {
        return;
    }
    let Some(rebuild) = app.rebuild.as_mut() else {
        return;
    };
    let mut focus_result = false;
    let mut drop_search = false;

    if let Some(search) = rebuild.search.as_mut() {
        if !search.editing {
            focus_result = search.active.is_some();
        } else if search.query.is_empty() {
            drop_search = true;
        } else if let Some(job) = rebuild.jobs.get(rebuild.active_idx) {
            let baseline = usize::from(rebuild.scroll_y);
            search.recompute_matches(job, baseline);
            if search.error.is_none() {
                search.editing = false;
                if search.active.is_none() && !search.matches.is_empty() {
                    search.active = Some(0);
                }
                focus_result = true;
            }
        }
    }

    if drop_search {
        rebuild.search = None;
    } else if focus_result {
        focus_on_active_search_match(rebuild);
    }
}

pub(super) fn handle_search_cancel(app: &mut App) {
    if app.state != UiState::Rebuilding {
        return;
    }
    let Some(rebuild) = app.rebuild.as_mut() else {
        return;
    };
    let mut remove = false;
    if let Some(search) = rebuild.search.as_mut() {
        if search.editing {
            if search.has_query() && search.regex.is_some() {
                search.editing = false;
                search.error = None;
            } else {
                remove = true;
            }
        } else {
            remove = true;
        }
    }
    if remove {
        rebuild.search = None;
    }
}

pub(super) fn handle_search_next(app: &mut App) {
    if app.state != UiState::Rebuilding {
        return;
    }
    let Some(rebuild) = app.rebuild.as_mut() else {
        return;
    };
    let mut focus = false;
    if let Some(search) = rebuild.search.as_mut() {
        if search.editing || search.matches.is_empty() {
            return;
        }
        search.set_direction(SearchDirection::Forward);
        search.advance_next();
        focus = true;
    }
    if focus {
        focus_on_active_search_match(rebuild);
    }
}

pub(super) fn handle_search_prev(app: &mut App) {
    if app.state != UiState::Rebuilding {
        return;
    }
    let Some(rebuild) = app.rebuild.as_mut() else {
        return;
    };
    let mut focus = false;
    if let Some(search) = rebuild.search.as_mut() {
        if search.editing || search.matches.is_empty() {
            return;
        }
        search.set_direction(SearchDirection::Backward);
        search.advance_prev();
        focus = true;
    }
    if focus {
        focus_on_active_search_match(rebuild);
    }
}

pub(super) fn refresh_search_for_active_job(rebuild: &mut RebuildState) {
    if let Some(search) = rebuild.search.as_mut()
        && let Some(job) = rebuild.jobs.get(rebuild.active_idx)
    {
        if search.has_query() {
            let baseline = usize::from(rebuild.scroll_y);
            search.recompute_matches(job, baseline);
        } else {
            search.clear_results();
        }
    }
}

pub(super) fn focus_on_active_search_match(rebuild: &mut RebuildState) {
    let Some(search) = rebuild.search.as_ref() else {
        return;
    };
    let Some(active_idx) = search.active else {
        return;
    };
    let Some(hit) = search.matches.get(active_idx) else {
        return;
    };
    let Some(job) = rebuild.jobs.get(rebuild.active_idx) else {
        return;
    };

    let viewport = usize::from(rebuild.viewport_height.max(1));
    let max_start = job.output.len().saturating_sub(viewport);
    let target = hit.line.saturating_sub(viewport / 2);
    rebuild.scroll_y = super::clamp_usize_to_u16(target.min(max_start));
    rebuild.auto_scroll = false;
}
