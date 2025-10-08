use super::events::handle_message;
use crate::tui::app::state::{App, Msg, Services};

pub fn update_with_services(app: &mut App, msg: Msg, services: Option<&Services>) {
    handle_message(app, msg, services);
}
