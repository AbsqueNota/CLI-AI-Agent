pub mod preset;

use dialoguer::{
    console::{style, Color as ConsoleColor, Style},
    theme::ColorfulTheme,
};
use std::{fmt, sync::Arc};
use colored::Color;
use crate::ui::theme::preset::{
    Palette,
    ThemePreset,
};

#[derive(Clone)]
pub struct AppTheme {
    pub palette: Palette,
    pub select_theme: Arc<ColorfulTheme>,
    pub input_theme: Arc<ColorfulTheme>,
}

impl AppTheme {
    pub fn from_preset(preset: ThemePreset) -> Self {
        let palette = Palette::from_preset(preset);

        let select_theme = build_select_theme(&palette);
        let input_theme = build_input_theme(&palette);

        Self {
            palette,
            select_theme: Arc::new(select_theme),
            input_theme: Arc::new(input_theme),
        }
    }

    pub fn error_color() -> colored::Color {
        Color::TrueColor { r: 255, g: 75, b: 92 }
    }

    pub fn warning_color() -> colored::Color {
        Color::TrueColor { r: 255, g: 196, b: 87 }
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
            .field("select_theme", &"ColorfulTheme(..)")
            .field("input_theme", &"ColorfulTheme(..)")
            .finish()
    }
}

fn build_select_theme(palette: &Palette) -> ColorfulTheme {
    let mut theme = ColorfulTheme::default();

    theme.prompt_prefix = style("✦".to_string())
        .for_stderr()
        .fg(to_console_color(palette.select.prompt))
        .bold();

    theme.prompt_style = Style::new()
        .for_stderr()
        .fg(to_console_color(palette.select.prompt))
        .bold();

    theme.active_item_prefix = style("▸".to_string())
        .for_stderr()
        .fg(to_console_color(palette.select.active))
        .bold();

    theme.inactive_item_prefix = style(" ".to_string())
        .for_stderr();

    theme.active_item_style = Style::new()
        .for_stderr()
        .fg(to_console_color(palette.select.active))
        .bold();

    theme.inactive_item_style = Style::new()
        .for_stderr()
        .fg(to_console_color(palette.select.inactive));

    theme.values_style = Style::new()
        .for_stderr()
        .fg(to_console_color(palette.select.value))
        .bold();

    theme
}

fn build_input_theme(palette: &Palette) -> ColorfulTheme {
    let mut theme = ColorfulTheme::default();

    theme.prompt_prefix = style("✦".to_string())
        .for_stderr()
        .fg(to_console_color(palette.input.prompt))
        .bold();

    theme.prompt_style = Style::new()
        .for_stderr()
        .fg(to_console_color(palette.input.prompt))
        .bold();

    theme

}

fn to_console_color(color: colored::Color) -> ConsoleColor {
    match color {
        colored::Color::TrueColor { r, g, b } => {
            ConsoleColor::TrueColor(r, g, b)
        }
        _ => ConsoleColor::White,
    }
}