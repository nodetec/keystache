use iced::{
    widget::{
        button::{self, Status},
        row, text, Button,
    },
    Border, Color, Shadow, Theme,
};

use crate::Message;

use super::{
    util::{darken, lighten},
    PaletteColor, SvgIcon,
};

pub fn icon_button(
    text_str: &str,
    icon: SvgIcon,
    palette_color: PaletteColor,
) -> Button<'_, Message, Theme> {
    // TODO: Find a way to darken the icon color when the button is disabled.
    let svg = icon.view(24.0, 24.0, Color::WHITE);
    let content = row![svg, text(text_str).size(24.0)]
        .align_items(iced::Alignment::Center)
        .spacing(8)
        .padding(8);

    Button::new(content).style(move |theme, status| {
        let border = Border {
            color: iced::Color::WHITE,
            width: 0.0,
            radius: (8.0).into(),
        };

        let mut bg_color = palette_color.to_color(theme);

        if palette_color == PaletteColor::Background {
            bg_color = lighten(bg_color, 0.05);
        }

        bg_color = match status {
            Status::Hovered => lighten(bg_color, 0.05),
            Status::Pressed => lighten(bg_color, 0.1),
            Status::Disabled => darken(bg_color, 0.5),
            _ => bg_color,
        };

        let mut text_color = Color::WHITE;
        if status == Status::Disabled {
            text_color = darken(text_color, 0.5);
        }

        button::Style {
            background: Some(bg_color.into()),
            text_color,
            border,
            shadow: Shadow::default(),
        }
    })
}
