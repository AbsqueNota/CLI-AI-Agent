pub mod config;
pub mod state;
pub mod commands;
pub mod theme;

use crate::ui::TerminalUi;

pub struct AppContext {
    pub state: state::AppState,
    pub ui: TerminalUi,
}

impl AppContext {
    pub fn new(state: state::AppState, ui: TerminalUi) -> Self {
        Self { state, ui }
    }
}