use serde::{Deserialize, Serialize};
use colored::Color as ColoredColor;
use std::fmt;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreset {
    #[default]
    Neon,
    Pink,
}

impl fmt::Display for ThemePreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Neon => "Neon",
            Self::Pink => "Pink",
        };

        f.write_str(name)
    }
}

pub struct Palette {
    pub accent_pink:  ColoredColor,
    pub accent_mint:  ColoredColor,
    pub text_light:   ColoredColor,
    pub text_muted:   ColoredColor,
    pub dark_gray:    ColoredColor,
    pub text_warning: ColoredColor,
    pub text_error:   ColoredColor,
}

impl Palette {
    pub fn from_preset(preset: ThemePreset) -> Self {
        match preset {
            ThemePreset::Neon => Self {
                accent_pink: rgb(250, 6, 115),
                accent_mint: rgb(0, 245, 212),
                text_light: rgb(226, 212, 240),
                text_muted: rgb(158, 138, 152),
                dark_gray: rgb(128, 128, 128),
                text_warning: rgb(255, 196, 87),
                text_error: rgb(255, 75, 92),
            },

            ThemePreset::Pink => Self {
                accent_pink: rgb(255, 105, 180),
                accent_mint: rgb(220, 130, 220),
                text_light: rgb(255, 230, 245),
                text_muted: rgb(190, 150, 175),
                dark_gray: rgb(125, 105, 120),
                text_warning: rgb(255, 190, 110),
                text_error: rgb(255, 80, 120),
            },
        }
    }
}

fn rgb(r: u8, g: u8, b: u8) -> ColoredColor {
    ColoredColor::TrueColor { r, g, b }
}
