use iced::{
    widget::{
        svg::{self, Handle},
        Svg,
    },
    Color, Theme,
};

pub enum SvgIcon {
    ArrowBack,
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
    pub fn view<'a>(&self, width: f32, height: f32, color: Color) -> Svg<'a, Theme> {
        match self {
            SvgIcon::ArrowBack => icon_handle!("arrow_back.svg"),
            SvgIcon::CurrencyBitcoin => icon_handle!("currency_bitcoin.svg"),
            SvgIcon::Delete => icon_handle!("delete.svg"),
            SvgIcon::Home => icon_handle!("home.svg"),
            SvgIcon::Hub => icon_handle!("hub.svg"),
            SvgIcon::Key => icon_handle!("key.svg"),
            SvgIcon::LockOpen => icon_handle!("lock_open.svg"),
            SvgIcon::Save => icon_handle!("save.svg"),
            SvgIcon::Settings => icon_handle!("settings.svg"),
            SvgIcon::ThumbDown => icon_handle!("thumb_down.svg"),
            SvgIcon::ThumbUp => icon_handle!("thumb_up.svg"),
        }
        .style(move |_, _| svg::Style { color: Some(color) })
        .width(width)
        .height(height)
    }
}
