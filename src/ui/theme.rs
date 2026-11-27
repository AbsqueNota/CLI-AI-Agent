pub mod preset;

use std::fmt;
use crate::ui::theme::preset::{
    Palette,
    ThemePreset,
};

#[derive(Clone)]
pub struct AppTheme {
    pub palette: Palette,
}

impl AppTheme {
    pub fn from_preset(preset: ThemePreset) -> Self {
        Self {
            palette: Palette::from_preset(preset),
        }
    }
}

impl Default for AppTheme {
    fn default() -> Self {
        Self::from_preset(ThemePreset::default())
    }
}

impl fmt::Debug for AppTheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppTheme")
            .field("palette", &self.palette)
            .finish()
    }
}