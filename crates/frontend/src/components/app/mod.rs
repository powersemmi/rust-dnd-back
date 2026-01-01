mod mouse_handler;
mod navigation;
mod state;

pub use state::App;

pub(crate) use mouse_handler::create_mouse_move_handler;
pub(crate) use navigation::{create_login_success_callback, create_navigation_callbacks};

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum AppState {
    Login,
    Register,
    RoomSelection,
    Connected,
}
