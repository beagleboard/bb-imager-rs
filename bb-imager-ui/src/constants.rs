use std::sync::LazyLock;

use iced::{color, widget::svg};

pub(crate) static BOARD_ICON: LazyLock<svg::Handle> =
    LazyLock::new(|| svg::Handle::from_memory(include_bytes!("../assets/icons/board.svg")));
pub(crate) static SEARCH_ICON: LazyLock<svg::Handle> =
    LazyLock::new(|| svg::Handle::from_memory(include_bytes!("../assets/icons/search.svg")));
pub(crate) static FORMAT_ICON: LazyLock<svg::Handle> =
    LazyLock::new(|| svg::Handle::from_memory(include_bytes!("../assets/icons/format.svg")));
pub(crate) static FILE_ADD_ICON: LazyLock<svg::Handle> =
    LazyLock::new(|| svg::Handle::from_memory(include_bytes!("../assets/icons/file-add.svg")));
pub(crate) static ARROW_BACK_ICON: LazyLock<svg::Handle> =
    LazyLock::new(|| svg::Handle::from_memory(include_bytes!("../assets/icons/arrow-back.svg")));
pub(crate) static ARROW_FORWARD_ICON: LazyLock<svg::Handle> = LazyLock::new(|| {
    svg::Handle::from_memory(include_bytes!("../assets/icons/arrow-forward-ios.svg"))
});
pub(crate) static USB_ICON: LazyLock<svg::Handle> =
    LazyLock::new(|| svg::Handle::from_memory(include_bytes!("../assets/icons/usb.svg")));
pub(crate) static FILE_SAVE_ICON: LazyLock<svg::Handle> =
    LazyLock::new(|| svg::Handle::from_memory(include_bytes!("../assets/icons/file-save.svg")));
pub(crate) static COPY_ICON: LazyLock<svg::Handle> =
    LazyLock::new(|| svg::Handle::from_memory(include_bytes!("../assets/icons/content-copy.svg")));
pub(crate) static BEAGLEBOARD_LOGO: LazyLock<svg::Handle> = LazyLock::new(|| {
    svg::Handle::from_memory(include_bytes!("../assets/icons/beagleboard-logo.svg"))
});

// Theme
pub(crate) const TONGUE_ORANGE: iced::Color = color!(242, 105, 53);
pub(crate) const CHECK_MARK_GREEN: iced::Color = color!(142, 201, 105);
pub(crate) const HAIR_LIGHT_BROWN: iced::Color = color!(171, 131, 60);
pub(crate) const DANGER: iced::Color = color!(255, 0, 0);

pub(crate) static ISSUE_TRACKER: LazyLock<url::Url> = LazyLock::new(|| {
    url::Url::parse("https://github.com/beagleboard/bb-imager-rs/issues").unwrap()
});

// Font
pub(crate) const FONT_REGULAR: iced::Font = iced::Font::with_name("Nunito");
pub(crate) const FONT_BOLD: iced::Font = {
    let mut font = FONT_REGULAR;
    font.weight = iced::font::Weight::Bold;

    font
};

// Base Fonts
pub(crate) const FONT_NORMAL_BYTES: &[u8] =
    include_bytes!("../assets/fonts/Nunito-Regular-subset.ttf");
pub(crate) const FONT_BOLD_BYTES: &[u8] = include_bytes!("../assets/fonts/Nunito-Bold-subset.ttf");

pub(crate) const WINDOW_ICON_BYTES: &[u8] = include_bytes!("../assets/icons/icon.png");
