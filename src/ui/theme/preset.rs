use ratatui::style::Color;
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
    pub main    : Color,
    pub default : Color,
    pub muted   : Color,
    pub info    : Color,
    pub hint    : Color,
    pub error   : Color,
    pub warning : Color, 
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            main    : Color::Rgb(210, 215, 228),
            default : Color::Rgb(165, 172, 188),
            muted   : Color::Rgb( 68,  73,  88),
            hint    : Color::Rgb(120, 130, 152),
            info    : Color::Rgb(142, 150, 170),
            error   : Color::Rgb(255,  75,  92),
            warning : Color::Rgb(255, 196,  87),
        }
    }
}


impl Palette {
    pub fn from_preset(preset: ThemePreset) -> Self {
        match preset {
            ThemePreset::Neon => Self {
                main    : Color::Rgb(168,255,  0),
                default : Color::Rgb(240,255,250),
                muted   : Color::Rgb( 72,102, 92),
                hint    : Color::Rgb(  0,255,235),
                info    : Color::Rgb(185, 75,255),
                ..Self::default()
            },

            ThemePreset::Pink => Self {
                main    : Color::Rgb(255, 45,135),
                default : Color::Rgb(255,232,242),
                muted   : Color::Rgb(148, 96,118),
                hint    : Color::Rgb(255,130,185),
                info    : Color::Rgb(230, 70,165),
                ..Self::default()
            },

            ThemePreset::Dark => Self {
                main    : Color::Rgb(210,215,228),
                default : Color::Rgb(165,172,188),
                muted   : Color::Rgb( 68, 73, 88),
                hint    : Color::Rgb(120,130,152),
                info    : Color::Rgb(142,150,170),
                ..Self::default()
            },

            ThemePreset::Light => Self {
                main    : Color::Rgb( 20, 24, 33),
                default : Color::Rgb( 42, 48, 62),
                muted   : Color::Rgb(150,157,173),
                hint    : Color::Rgb( 32, 86,150),
                info    : Color::Rgb( 36,104,112),
                ..Self::default()
            },
        }
    }
}