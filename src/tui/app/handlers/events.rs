use super::expansion::{handle_collapse_or_back, handle_expand_or_enter};
use super::scan::{handle_details_ready, handle_scan_results, handle_tick};
use super::view_picker::{
    handle_open_view_picker, handle_view_picker_accept, handle_view_picker_cancel,
    handle_view_picker_down, handle_view_picker_up,
};
use crate::tui::app::keymap::map_key_event_to_msg;
use crate::tui::app::state::{App, Msg, Services};
use crossterm::event::KeyEvent;

pub fn handle_message(app: &mut App, msg: Msg, services: Option<&Services>) {
    match msg {
        Msg::Key(key) => handle_key(app, key, services),
        Msg::Init => handle_init(services),
        Msg::Quit | Msg::Interrupt => app.should_quit = true,
        Msg::MoveUp => handle_move_up(app),
        Msg::MoveDown => handle_move_down(app),
        Msg::ToggleCheck => handle_toggle_check(app),
        Msg::ExpandOrEnter => handle_expand_or_enter(app, services),
        Msg::CollapseOrBack => handle_collapse_or_back(app),
        Msg::OpenViewPicker => handle_open_view_picker(app),
        Msg::ViewPickerUp => handle_view_picker_up(app),
        Msg::ViewPickerDown => handle_view_picker_down(app),
        Msg::ViewPickerAccept => handle_view_picker_accept(app),
        Msg::ViewPickerCancel => handle_view_picker_cancel(app),
        Msg::Tick => handle_tick(app),
        Msg::ScanResults(discovered) => handle_scan_results(app, discovered),
        Msg::DetailsReady { row, details } => handle_details_ready(app, row, details),
    }
}

fn handle_key(app: &mut App, key: KeyEvent, services: Option<&Services>) {
    if let Some(mapped) = map_key_event_to_msg(app, key) {
        super::update::update_with_services(app, mapped, services);
    }
}

fn handle_init(services: Option<&Services>) {
    if let Some(svc) = services {
        let tx = svc.tx.clone();
        let root = svc.root.clone();
        let include = svc.include.clone();
        let exclude = svc.exclude.clone();
        let core = svc.core.clone();
        std::thread::spawn(move || {
            let result = core.scan_images(root, include, exclude).unwrap_or_default();
            let _ = tx.send(Msg::ScanResults(result));
        });
    }
}

fn handle_move_up(app: &mut App) {
    if app.selected > 0 {
        app.selected -= 1;
    }
}

fn handle_move_down(app: &mut App) {
    if app.selected + 1 < app.rows.len() {
        app.selected += 1;
    }
}

fn handle_toggle_check(app: &mut App) {
    if let Some(row) = app.rows.get_mut(app.selected) {
        row.checked = !row.checked;
    }
}
