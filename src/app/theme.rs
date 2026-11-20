use dialoguer::{
    console::{style, Color as ConsoleColor, Style},
    theme::ColorfulTheme,
};
use colored::Color as ColoredColor;
use std::{fmt, sync::Arc};

#[derive(Clone)]
pub struct AppTheme {
    pub accent_pink: ColoredColor,
    pub accent_mint: ColoredColor,
    pub text_light:  ColoredColor,
    pub text_muted:  ColoredColor,
    pub dark_gray:   ColoredColor,
    pub dialoguer_theme: Arc<ColorfulTheme>,
}

fn to_console_color(color: ColoredColor) -> ConsoleColor {
    match color {
        ColoredColor::TrueColor { r, g, b } => ConsoleColor::TrueColor(r, g, b),
        _ => ConsoleColor::White, // Запасной вариант для базовых цветов
    }
}
impl Default for AppTheme {
    fn default() -> Self {  
        let mut dialoguer_theme=  ColorfulTheme::default();
        let accent_pink = ColoredColor::TrueColor { r: 250, g:   6, b: 115 };  // #FA0673
        let accent_mint = ColoredColor::TrueColor { r:   0, g: 245, b: 212 };  // #00F5D4
        let text_light  = ColoredColor::TrueColor { r: 226, g: 212, b: 240 };  // #e2d4f0 
        let text_muted  = ColoredColor::TrueColor { r: 158, g: 138, b: 152 };  // #9E8A98
        let dark_gray   = ColoredColor::TrueColor { r: 128, g: 128, b: 128 };  // #808080
        dialoguer_theme.prompt_prefix = style("✦".to_string())
            .for_stderr()
            .fg(to_console_color(text_light))
            .bold();
        dialoguer_theme.prompt_style = Style::new()
            .for_stderr()
            .fg(to_console_color(text_light))
            .bold();
        dialoguer_theme.active_item_prefix = style("▸".to_string())
            .for_stderr()
            .fg(to_console_color(accent_pink))
            .bold();
        dialoguer_theme.inactive_item_prefix = style(" ".to_string()).for_stderr();
        dialoguer_theme.active_item_style = Style::new()
            .for_stderr()
            .fg(to_console_color(accent_pink))
            .bold();
        dialoguer_theme.inactive_item_style = Style::new()
            .for_stderr()
            .fg(to_console_color(text_muted));
        dialoguer_theme.values_style = Style::new()
            .for_stderr()
            .fg(to_console_color(accent_mint))
            .bold();
        Self {
            dialoguer_theme:Arc::new(dialoguer_theme),
            accent_pink,
            accent_mint,
            text_light,
            text_muted,
            dark_gray,
        }
    }
}
impl fmt::Debug for AppTheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppTheme")
            .field("dialog", &"ColorfulTheme(..)")
            .field("accent_pink", &self.accent_pink)
            .field("accent_mint", &self.accent_mint)
            .field("text_light",  &self.text_light)
            .field("text_muted",  &self.text_muted)
            .finish()
    }
}