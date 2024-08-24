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
            Self::Background => theme.palette().background,
            Self::Text => theme.palette().text,
            Self::Primary => theme.palette().primary,
            Self::Success => theme.palette().success,
            Self::Danger => theme.palette().danger,
        }
    }
}
