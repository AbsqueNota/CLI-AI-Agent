pub mod preset;

use colored::Color as ColoredColor;
use dialoguer::{
    console::{style, Color as ConsoleColor, Style},
    theme::ColorfulTheme,
};
use std::{fmt, sync::Arc};
use crate::ui::theme::preset::{ThemePreset, Palette};

#[derive(Clone)]
pub struct AppTheme {
    pub accent_pink: ColoredColor,
    pub accent_mint: ColoredColor,
    pub text_light: ColoredColor,
    pub text_muted: ColoredColor,
    pub dark_gray: ColoredColor,
    pub text_warning: ColoredColor,
    pub text_error: ColoredColor,
    pub dialoguer_theme: Arc<ColorfulTheme>,
}

impl AppTheme {
    pub fn from_preset(preset: ThemePreset) -> Self {
        let palette = Palette::from_preset(preset);
        let mut dialoguer_theme = ColorfulTheme::default();

        dialoguer_theme.prompt_prefix = style("✦".to_string())
            .for_stderr()
            .fg(to_console_color(palette.text_light))
            .bold();

        dialoguer_theme.prompt_style = Style::new()
            .for_stderr()
            .fg(to_console_color(palette.text_light))
            .bold();

        dialoguer_theme.active_item_prefix = style("▸".to_string())
            .for_stderr()
            .fg(to_console_color(palette.accent_pink))
            .bold();

        dialoguer_theme.inactive_item_prefix =
            style(" ".to_string()).for_stderr();

        dialoguer_theme.active_item_style = Style::new()
            .for_stderr()
            .fg(to_console_color(palette.accent_pink))
            .bold();

        dialoguer_theme.inactive_item_style = Style::new()
            .for_stderr()
            .fg(to_console_color(palette.text_muted));

        dialoguer_theme.values_style = Style::new()
            .for_stderr()
            .fg(to_console_color(palette.accent_mint))
            .bold();

        Self {
            accent_pink: palette.accent_pink,
            accent_mint: palette.accent_mint,
            text_light: palette.text_light,
            text_muted: palette.text_muted,
            dark_gray: palette.dark_gray,
            text_warning: palette.text_warning,
            text_error: palette.text_error,
            dialoguer_theme: Arc::new(dialoguer_theme),
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
            .field("dialoguer_theme", &"ColorfulTheme(..)")
            .field("accent_pink", &self.accent_pink)
            .field("accent_mint", &self.accent_mint)
            .field("text_light", &self.text_light)
            .field("text_muted", &self.text_muted)
            .field("text_warning", &self.text_warning)
            .field("text_error", &self.text_error)
            .finish()
    }
}

fn to_console_color(color: ColoredColor) -> ConsoleColor {
    match color {
        ColoredColor::TrueColor { r, g, b } => {
            ConsoleColor::TrueColor(r, g, b)
        }
        _ => ConsoleColor::White,
    }
}