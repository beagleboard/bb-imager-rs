use std::sync::LazyLock;

use iced::{color, widget};

// Icons Bytes
pub(crate) const WINDOW_ICON_BYTES: &[u8] = include_bytes!("../assets/icons/icon.png");

// Icon Handles
pub(crate) static WINDOW_ICON: LazyLock<widget::image::Handle> =
    LazyLock::new(|| widget::image::Handle::from_bytes(WINDOW_ICON_BYTES));

// Fonts
pub(crate) const FONT_REGULAR: iced::Font = iced::Font::with_name("Nunito");
pub(crate) const FONT_NORMAL_BYTES: &[u8] =
    include_bytes!("../assets/fonts/Nunito-Regular-subset.ttf");
pub(crate) const FONT_BOLD_BYTES: &[u8] = include_bytes!("../assets/fonts/Nunito-Bold-subset.ttf");

// Theme
pub(crate) const TONGUE_ORANGE: iced::Color = color!(242, 105, 53);
pub(crate) const CHECK_MARK_GREEN: iced::Color = color!(142, 201, 105);
pub(crate) const HAIR_LIGHT_BROWN: iced::Color = color!(171, 131, 60);
pub(crate) const BACKGROUND: iced::Color = color!(30, 30, 30);
pub(crate) const DANGER: iced::Color = color!(255, 0, 0);
pub(crate) const CARD: iced::Color = color!(45, 45, 45);
