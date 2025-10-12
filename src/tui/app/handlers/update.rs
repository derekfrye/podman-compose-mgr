use super::events::handle_message;
use crate::tui::app::state::{App, Msg, Services};

/// Dispatch a message to the TUI update handler with optional environment services.
///
/// # Panics
/// Panics if the message handling path assumes services are present but `services` is `None`.
pub fn update_with_services(app: &mut App, msg: Msg, services: Option<&Services>) {
    handle_message(app, msg, services);
}
