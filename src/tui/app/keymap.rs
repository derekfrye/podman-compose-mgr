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
            ModalState::ExportLog { .. } => Some(match key {
                KeyCode::Esc => Msg::ExportCancel,
                KeyCode::Enter => Msg::ExportSubmit,
                KeyCode::Backspace => Msg::ExportBackspace,
                KeyCode::Char(ch) if !ch.is_control() => Msg::ExportInput(ch),
                _ => return None,
            }),
        };
    }

    if matches!(app.state, super::state::UiState::Rebuilding) {
        if let Some(rebuild) = app.rebuild.as_ref()
            && let Some(search) = rebuild.search.as_ref()
            && search.editing
        {
            return Some(match key {
                KeyCode::Esc => Msg::SearchCancel,
                KeyCode::Enter => Msg::SearchSubmit,
                KeyCode::Backspace => Msg::SearchBackspace,
                KeyCode::Char(ch) => Msg::SearchInput(ch),
                _ => return None,
            });
        }
        return Some(match key {
            KeyCode::Up => Msg::ScrollOutputUp,
            KeyCode::Down => Msg::ScrollOutputDown,
            KeyCode::PageUp => Msg::ScrollOutputPageUp,
            KeyCode::PageDown | KeyCode::Char(' ') => Msg::ScrollOutputPageDown,
            KeyCode::Left => Msg::ScrollOutputLeft,
            KeyCode::Right => Msg::ScrollOutputRight,
            KeyCode::Home | KeyCode::Char('g') => Msg::ScrollOutputTop,
            KeyCode::End | KeyCode::Char('G') => Msg::ScrollOutputBottom,
            KeyCode::Char('/') => Msg::StartSearchForward,
            KeyCode::Char('?') => Msg::StartSearchBackward,
            KeyCode::Char('n') => Msg::SearchNext,
            KeyCode::Char('N') => Msg::SearchPrev,
            KeyCode::Char('w') => Msg::OpenWorkQueue,
            KeyCode::Char('e' | 'E') => Msg::OpenExportLog,
            KeyCode::Char('q' | 'Q') => Msg::Quit,
            KeyCode::Esc => Msg::ExitRebuild,
            _ => return None,
        });
    }

    Some(match key {
        KeyCode::Char('q' | 'Q') | KeyCode::Esc => Msg::Quit,
        KeyCode::Up => Msg::MoveUp,
        KeyCode::Down => Msg::MoveDown,
        KeyCode::PageUp | KeyCode::Char('b') => Msg::MovePageUp,
        KeyCode::PageDown | KeyCode::Char('f') => Msg::MovePageDown,
        KeyCode::Char(' ' | 'x' | 'X') | KeyCode::Enter => Msg::ToggleCheck,
        KeyCode::Right => Msg::ExpandOrEnter,
        KeyCode::Left => Msg::CollapseOrBack,
        KeyCode::Char('j' | 'J') if app.rebuild.is_some() => Msg::ShowRebuild,
        KeyCode::Char('v') => Msg::OpenViewPicker,
        KeyCode::Char('r') => Msg::StartRebuild,
        KeyCode::Char('w') => Msg::OpenWorkQueue,
        KeyCode::Char('a') => Msg::ToggleCheckAll,
        _ => return None,
    })
}
