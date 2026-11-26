use colored::Color as ColoredColor;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreset {
    #[default]
    Neon,
    Pink,
    Dark,
    Light,
}

impl fmt::Display for ThemePreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Neon  => "Neon",
            Self::Pink  => "Pink",
            Self::Dark  => "Dark",
            Self::Light => "Light",
        };

        f.write_str(name)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub main    : ColoredColor,
    pub default : ColoredColor,
    pub muted   : ColoredColor,
    pub info    : ColoredColor,
    pub hint    : ColoredColor,
}

impl Palette {
    pub fn from_preset(preset: ThemePreset) -> Self {
        match preset {
            ThemePreset::Neon => Self {
                main    : rgb(168,255,0),
                default : rgb(240,255,250),
                muted   : rgb(72, 102,92),
                hint    : rgb(0,  255,235),
                info    : rgb(185,75, 255),
            },

            ThemePreset::Pink => Self {
                main    : rgb(255, 45, 135),
                default : rgb(255, 232, 242),
                muted   : rgb(148, 96, 118),
                hint    : rgb(255, 130, 185),
                info    : rgb(230, 70, 165),
            },

            ThemePreset::Dark => Self {
                main    : rgb(210, 215, 228),
                default : rgb(165, 172, 188),
                muted   : rgb(68, 73, 88),
                hint    : rgb(120, 130, 152),
                info    : rgb(142, 150, 170),
            },

            ThemePreset::Light => Self {
                main    : rgb(20, 24, 33),
                default : rgb(42, 48, 62),
                muted   : rgb(150, 157, 173),
                hint    : rgb(32, 86, 150),
                info    : rgb(36, 104, 112),
            },
        }
    }
}

fn rgb(r: u8, g: u8, b: u8) -> ColoredColor {
    ColoredColor::TrueColor { r, g, b }
}
