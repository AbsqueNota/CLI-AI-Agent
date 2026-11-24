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
            Self::Neon => "Neon",
            Self::Pink => "Pink",
            Self::Dark => "Dark",
            Self::Light => "Light",
        };

        f.write_str(name)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub welcome: WelcomePalette,
    pub info: ColoredColor,
    pub model: ModelPalette,
    pub select: SelectPalette,
    pub spinner: SpinnerPalette,
    pub input: InputPalette,
}

#[derive(Debug, Clone, Copy)]
pub struct WelcomePalette {
    pub title: ColoredColor,
    pub hint: ColoredColor,
}

#[derive(Debug, Clone, Copy)]
pub struct ModelPalette {
    pub thinking: ColoredColor,
    pub answer: ColoredColor,
}

#[derive(Debug, Clone, Copy)]
pub struct SelectPalette {
    pub prompt: ColoredColor,
    pub active: ColoredColor,
    pub inactive: ColoredColor,
    pub value: ColoredColor,
}

#[derive(Debug, Clone, Copy)]
pub struct SpinnerPalette {
    pub message: ColoredColor,
    pub indicator: ColoredColor,
}

#[derive(Debug, Clone, Copy)]
pub struct InputPalette {
    pub prompt: ColoredColor,
    pub value: ColoredColor,
}

impl Palette {
    pub fn from_preset(preset: ThemePreset) -> Self {
        match preset {
            ThemePreset::Neon => Self {
                welcome: WelcomePalette {
                    title: rgb(250, 6, 115),
                    hint: rgb(158, 138, 152),
                },
                info: rgb(0, 245, 212),
                model: ModelPalette {
                    thinking: rgb(128, 128, 128),
                    answer: rgb(226, 212, 240),
                },
                select: SelectPalette {
                    prompt: rgb(226, 212, 240),
                    active: rgb(250, 6, 115),
                    inactive: rgb(158, 138, 152),
                    value: rgb(0, 245, 212),
                },
                spinner: SpinnerPalette {
                    message: rgb(158, 138, 152),
                    indicator: rgb(0, 245, 212),
                },
                input: InputPalette {
                    prompt: rgb(250, 6, 115),
                    value: rgb(226, 212, 240),
                },
            },

            ThemePreset::Pink => Self {
                welcome: WelcomePalette {
                    title: rgb(255, 105, 180),
                    hint: rgb(190, 150, 175),
                },
                info: rgb(220, 130, 220),
                model: ModelPalette {
                    thinking: rgb(125, 105, 120),
                    answer: rgb(255, 230, 245),
                },
                select: SelectPalette {
                    prompt: rgb(255, 230, 245),
                    active: rgb(255, 105, 180),
                    inactive: rgb(190, 150, 175),
                    value: rgb(220, 130, 220),
                },
                spinner: SpinnerPalette {
                    message: rgb(190, 150, 175),
                    indicator: rgb(220, 130, 220),
                },
                input: InputPalette {
                    prompt: rgb(255, 105, 180),
                    value: rgb(255, 230, 245),
                },
            },

            ThemePreset::Dark => Self {
                welcome: WelcomePalette {
                    title: rgb(255, 90, 160),
                    hint: rgb(145, 155, 175),
                },
                info: rgb(70, 230, 190),
                model: ModelPalette {
                    thinking: rgb(110, 120, 135),
                    answer: rgb(230, 235, 245),
                },
                select: SelectPalette {
                    prompt: rgb(230, 235, 245),
                    active: rgb(255, 90, 160),
                    inactive: rgb(145, 155, 175),
                    value: rgb(70, 230, 190),
                },
                spinner: SpinnerPalette {
                    message: rgb(145, 155, 175),
                    indicator: rgb(70, 230, 190),
                },
                input: InputPalette {
                    prompt: rgb(255, 90, 160),
                    value: rgb(230, 235, 245),
                },
            },

            ThemePreset::Light => Self {
                welcome: WelcomePalette {
                    title: rgb(190, 35, 110),
                    hint: rgb(105, 110, 125),
                },
                info: rgb(0, 130, 115),
                model: ModelPalette {
                    thinking: rgb(115, 120, 130),
                    answer: rgb(35, 40, 50),
                },
                select: SelectPalette {
                    prompt: rgb(35, 40, 50),
                    active: rgb(190, 35, 110),
                    inactive: rgb(105, 110, 125),
                    value: rgb(0, 130, 115),
                },
                spinner: SpinnerPalette {
                    message: rgb(105, 110, 125),
                    indicator: rgb(0, 130, 115),
                },
                input: InputPalette {
                    prompt: rgb(190, 35, 110),
                    value: rgb(35, 40, 50),
                },
            },
        }
    }
}

fn rgb(r: u8, g: u8, b: u8) -> ColoredColor {
    ColoredColor::TrueColor { r, g, b }
}
