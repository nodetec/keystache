mod button;
pub use button::*;

mod icon;
use iced::{Color, Theme};
pub use icon::*;

mod sidebar;
pub use sidebar::*;

mod util;

#[derive(PartialEq, Eq)]
pub enum PaletteColor {
    Background,
    Text,
    Primary,
    Success,
    Danger,
}

impl PaletteColor {
    pub fn to_color(&self, theme: &Theme) -> Color {
        match self {
            PaletteColor::Background => theme.palette().background,
            PaletteColor::Text => theme.palette().text,
            PaletteColor::Primary => theme.palette().primary,
            PaletteColor::Success => theme.palette().success,
            PaletteColor::Danger => theme.palette().danger,
        }
    }
}
