use iced::{
    widget::{
        svg::{self, Handle},
        Svg,
    },
    Color, Theme,
};

#[derive(Clone, Copy)]
pub enum SvgIcon {
    Add,
    ArrowBack,
    Casino,
    CurrencyBitcoin,
    Delete,
    Home,
    Hub,
    Key,
    LockOpen,
    Save,
    Settings,
    ThumbDown,
    ThumbUp,
}

macro_rules! icon_handle {
    ($icon:expr) => {
        Svg::new(Handle::from_memory(include_bytes!(concat!(
            "../../assets/icons/",
            $icon
        ))))
    };
}

impl SvgIcon {
    pub fn view<'a>(self, width: f32, height: f32, color: Color) -> Svg<'a, Theme> {
        match self {
            Self::Add => icon_handle!("add.svg"),
            Self::ArrowBack => icon_handle!("arrow_back.svg"),
            Self::Casino => icon_handle!("casino.svg"),
            Self::CurrencyBitcoin => icon_handle!("currency_bitcoin.svg"),
            Self::Delete => icon_handle!("delete.svg"),
            Self::Home => icon_handle!("home.svg"),
            Self::Hub => icon_handle!("hub.svg"),
            Self::Key => icon_handle!("key.svg"),
            Self::LockOpen => icon_handle!("lock_open.svg"),
            Self::Save => icon_handle!("save.svg"),
            Self::Settings => icon_handle!("settings.svg"),
            Self::ThumbDown => icon_handle!("thumb_down.svg"),
            Self::ThumbUp => icon_handle!("thumb_up.svg"),
        }
        .style(move |_, _| svg::Style { color: Some(color) })
        .width(width)
        .height(height)
    }
}
