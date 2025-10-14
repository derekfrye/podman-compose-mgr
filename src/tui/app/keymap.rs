use super::state::{App, ModalState, Msg};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[must_use]
pub fn map_key_event_to_msg(app: &App, ev: KeyEvent) -> Option<Msg> {
    if ev.modifiers.contains(KeyModifiers::CONTROL) && matches!(ev.code, KeyCode::Char('c' | 'C')) {
        return Some(Msg::Interrupt);
    }
    map_keycode_to_msg(app, ev.code)
}

#[must_use]
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
            KeyCode::Char(' ') => Msg::ScrollOutputPageDown,
            KeyCode::Left => Msg::ScrollOutputLeft,
            KeyCode::Right => Msg::ScrollOutputRight,
            KeyCode::Home => Msg::ScrollOutputTop,
            KeyCode::Char('g') => Msg::ScrollOutputTop,
            KeyCode::End => Msg::ScrollOutputBottom,
            KeyCode::Char('G') => Msg::ScrollOutputBottom,
            KeyCode::Char('w') => Msg::OpenWorkQueue,
            KeyCode::Char('q') | KeyCode::Char('Q') => Msg::Quit,
            KeyCode::Esc => Msg::ExitRebuild,
            _ => return None,
        });
    }

    Some(match key {
        KeyCode::Char('q') | KeyCode::Char('Q') => Msg::Quit,
        KeyCode::Up => Msg::MoveUp,
        KeyCode::Down => Msg::MoveDown,
        KeyCode::PageUp => Msg::MovePageUp,
        KeyCode::PageDown => Msg::MovePageDown,
        KeyCode::Char(' ') | KeyCode::Char('x') | KeyCode::Char('X') | KeyCode::Enter => {
            Msg::ToggleCheck
        }
        KeyCode::Right => Msg::ExpandOrEnter,
        KeyCode::Left => Msg::CollapseOrBack,
        KeyCode::Char('v') => Msg::OpenViewPicker,
        KeyCode::Char('r') => Msg::StartRebuild,
        KeyCode::Char('w') => Msg::OpenWorkQueue,
        KeyCode::Char('b') => Msg::MovePageUp,
        KeyCode::Char('f') => Msg::MovePageDown,
        KeyCode::Char('a') => Msg::ToggleCheckAll,
        KeyCode::Esc => Msg::Quit,
        _ => return None,
    })
}
