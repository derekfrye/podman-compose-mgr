use super::state::{App, ModalState, Msg};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn map_key_event_to_msg(app: &App, ev: KeyEvent) -> Option<Msg> {
    if ev.modifiers.contains(KeyModifiers::CONTROL) && matches!(ev.code, KeyCode::Char('c' | 'C')) {
        return Some(Msg::Interrupt);
    }
    map_keycode_to_msg(app, ev.code)
}

pub fn map_keycode_to_msg(app: &App, key: KeyCode) -> Option<Msg> {
    if let Some(ModalState::ViewPicker { .. }) = app.modal {
        return Some(match key {
            KeyCode::Esc => Msg::ViewPickerCancel,
            KeyCode::Up => Msg::ViewPickerUp,
            KeyCode::Down => Msg::ViewPickerDown,
            KeyCode::Enter => Msg::ViewPickerAccept,
            _ => return None,
        });
    }

    Some(match key {
        KeyCode::Char('q') => Msg::Quit,
        KeyCode::Up => Msg::MoveUp,
        KeyCode::Down => Msg::MoveDown,
        KeyCode::Char(' ') | KeyCode::Enter => Msg::ToggleCheck,
        KeyCode::Right => Msg::ExpandOrEnter,
        KeyCode::Left => Msg::CollapseOrBack,
        KeyCode::Char('v') => Msg::OpenViewPicker,
        _ => return None,
    })
}
