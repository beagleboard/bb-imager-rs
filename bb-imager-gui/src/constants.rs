use iced::color;

pub(crate) const OSHW_BASE_URL: &str = "https://certification.oshwa.org";

pub(crate) const LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/beagleboard/bb-imager-rs/releases/latest";

pub(crate) const PACKAGE_QUALIFIER: (&str, &str, &str) = ("org", "beagleboard", "imagingutility");

pub(crate) const DEFAULT_CONFIG: &[u8] = include_bytes!("../../config.json");
pub(crate) const WINDOW_SIZE: iced::Size = iced::Size::new(680.0, 450.0);
pub(crate) const APP_NAME: &str = "BeagleBoard Imager";
pub(crate) const APP_RELEASE: &str = if cfg!(feature = "pre-release") {
    "pre-release"
} else {
    env!("CARGO_PKG_VERSION")
};
pub(crate) const APP_DESC: &str = env!("CARGO_PKG_DESCRIPTION");
pub(crate) const APP_LINCESE: &str = include_str!("../../LICENSE");

// Icons
pub(crate) const WINDOW_ICON_BYTES: &[u8] = include_bytes!("../assets/icons/icon.png");
pub(crate) const ARROW_BACK_ICON_BYTES: &[u8] = include_bytes!("../assets/icons/arrow-back.svg");
pub(crate) const FILE_ADD_ICON_BYTES: &[u8] = include_bytes!("../assets/icons/file-add.svg");
pub(crate) const USB_ICON_BYTES: &[u8] = include_bytes!("../assets/icons/usb.svg");
pub(crate) const FORMAT_ICON_BYTES: &[u8] = include_bytes!("../assets/icons/format.svg");
pub(crate) const BOARD_ICON_BYTES: &[u8] = include_bytes!("../assets/icons/board.svg");
pub(crate) const ARROW_FORWARD_IOS_ICON_BYTES: &[u8] =
    include_bytes!("../assets/icons/arrow-forward-ios.svg");
pub(crate) const FILE_SAVE_ICON_BYTES: &[u8] = include_bytes!("../assets/icons/file-save.svg");
pub(crate) const INFO_ICON_BYTES: &[u8] = include_bytes!("../assets/icons/info.svg");
pub(crate) const COPY_ICON_BYTES: &[u8] = include_bytes!("../assets/icons/content-copy.svg");
pub(crate) const SEARCH_ICON_BYTES: &[u8] = include_bytes!("../assets/icons/search.svg");

// Font
pub(crate) const FONT_REGULAR: iced::Font = iced::Font::with_name("Nunito");
pub(crate) const FONT_BOLD: iced::Font = {
    let mut font = FONT_REGULAR;
    font.weight = iced::font::Weight::Bold;

    font
};

// Base Fonts
pub(crate) const FONT_NORMAL_BYTES: &[u8] = include_bytes!("../assets/fonts/Nunito-Regular-subset.ttf");
pub(crate) const FONT_BOLD_BYTES: &[u8] = include_bytes!("../assets/fonts/Nunito-Bold-subset.ttf");

// Theme
pub(crate) const TONGUE_ORANGE: iced::Color = color!(242, 105, 53);
pub(crate) const CHECK_MARK_GREEN: iced::Color = color!(142, 201, 105);
pub(crate) const HAIR_LIGHT_BROWN: iced::Color = color!(171, 131, 60);
pub(crate) const BACKGROUND: iced::Color = color!(30, 30, 30);
pub(crate) const CARD: iced::Color = color!(45, 45, 45);
pub(crate) const DANGER: iced::Color = color!(255, 0, 0);

pub(crate) const KEYMAP_LAYOUTS: &[&str] = &[
    "af", "al", "am", "ara", "at", "au", "az", "ba", "bd", "be", "bg", "br", "brai", "bt", "bw",
    "by", "ca", "cd", "ch", "cm", "cn", "cz", "de", "dk", "dz", "ee", "epo", "es", "et", "fi",
    "fo", "fr", "gb", "ge", "gh", "gn", "gr", "hr", "hu", "id", "ie", "il", "in", "iq", "ir", "is",
    "it", "jp", "jv", "ke", "kg", "kh", "kr", "kz", "la", "latam", "lk", "lt", "lv", "ma", "mao",
    "md", "me", "mk", "ml", "mm", "mn", "mt", "mv", "my", "ng", "nl", "no", "np", "ph", "pk", "pl",
    "pt", "ro", "rs", "ru", "se", "si", "sk", "sn", "sy", "tg", "th", "tj", "tm", "tr", "tw", "tz",
    "ua", "us", "uz", "vn", "za",
];

#[cfg(test)]
mod tests {
    use super::KEYMAP_LAYOUTS;

    /// The keymap combo box looks up its selection with `binary_search`, so new
    /// entries need to be inserted in byte order.
    #[test]
    fn keymap_layouts_sorted() {
        assert!(KEYMAP_LAYOUTS.is_sorted());
    }
}
