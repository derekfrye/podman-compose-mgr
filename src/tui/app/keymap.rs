use super::state::{App, ModalState, Msg};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn map_key_event_to_msg(app: &App, ev: KeyEvent) -> Option<Msg> {
    if ev.modifiers.contains(KeyModifiers::CONTROL) && matches!(ev.code, KeyCode::Char('c' | 'C')) {
        return Some(Msg::Interrupt);
    }
    map_keycode_to_msg(app, ev.code)
}

pub fn map_keycode_to_msg(app: &App, key: KeyCode) -> Option<Msg> {
    if let Some(modal) = &app.modal {
        return match modal {
            ModalState::ViewPicker { .. } => Some(match key {
                KeyCode::Esc => Msg::ViewPickerCancel,
                KeyCode::Up => Msg::ViewPickerUp,
                KeyCode::Down => Msg::ViewPickerDown,
                KeyCode::Enter => Msg::ViewPickerAccept,
                _ => return None,
            }),
            ModalState::WorkQueue { .. } => Some(match key {
                KeyCode::Esc => Msg::CloseModal,
                KeyCode::Up => Msg::WorkQueueUp,
                KeyCode::Down => Msg::WorkQueueDown,
                KeyCode::Enter => Msg::WorkQueueSelect,
                _ => return None,
            }),
        };
    }

    if matches!(app.state, super::state::UiState::Rebuilding) {
        return Some(match key {
            KeyCode::Up => Msg::ScrollOutputUp,
            KeyCode::Down => Msg::ScrollOutputDown,
            KeyCode::PageUp => Msg::ScrollOutputPageUp,
            KeyCode::PageDown => Msg::ScrollOutputPageDown,
            KeyCode::Left => Msg::ScrollOutputLeft,
            KeyCode::Right => Msg::ScrollOutputRight,
            KeyCode::Home => Msg::ScrollOutputTop,
            KeyCode::End => Msg::ScrollOutputBottom,
            KeyCode::Char('w') => Msg::OpenWorkQueue,
            KeyCode::Esc => Msg::ExitRebuild,
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
        KeyCode::Char('r') => Msg::StartRebuild,
        KeyCode::Char('w') => Msg::OpenWorkQueue,
        KeyCode::Char('a') => Msg::ToggleCheckAll,
        KeyCode::Char('k') => Msg::ScrollOutputUp,
        KeyCode::Char('j') => Msg::ScrollOutputDown,
        KeyCode::Char('b') => Msg::ScrollOutputPageUp,
        KeyCode::Char('f') => Msg::ScrollOutputPageDown,
        KeyCode::Home => Msg::ScrollOutputTop,
        KeyCode::End => Msg::ScrollOutputBottom,
        KeyCode::Char('h') => Msg::ScrollOutputLeft,
        KeyCode::Char('l') => Msg::ScrollOutputRight,
        KeyCode::Esc => Msg::ExitRebuild,
        _ => return None,
    })
}
