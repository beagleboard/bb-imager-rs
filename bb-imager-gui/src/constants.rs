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
pub(crate) const FONT_NORMAL_BYTES: &[u8] =
    include_bytes!("../assets/fonts/Nunito-Regular-subset.ttf");
pub(crate) const FONT_BOLD_BYTES: &[u8] = include_bytes!("../assets/fonts/Nunito-Bold-subset.ttf");

// Theme
pub(crate) const TONGUE_ORANGE: iced::Color = color!(242, 105, 53);
pub(crate) const CHECK_MARK_GREEN: iced::Color = color!(142, 201, 105);
pub(crate) const HAIR_LIGHT_BROWN: iced::Color = color!(171, 131, 60);
pub(crate) const BACKGROUND: iced::Color = color!(30, 30, 30);
pub(crate) const CARD: iced::Color = color!(45, 45, 45);
pub(crate) const DANGER: iced::Color = color!(255, 0, 0);

const HC_BACKGROUND: iced::Color = color!(0, 0, 0);
const HC_TEXT: iced::Color = color!(255, 255, 255);

pub(crate) fn is_high_contrast_palette(palette: &iced::theme::Palette) -> bool {
    palette.background == HC_BACKGROUND
}

/// Inner and outer focus-ring colors.
///
/// High contrast uses a black inner stroke plus a white outer stroke so the
/// ring stays visible on both yellow primary buttons and dark fills. The
/// default theme is a single white ring (inner == outer).
pub(crate) fn focus_ring_colors(high_contrast: bool) -> (iced::Color, iced::Color) {
    if high_contrast {
        (HC_BACKGROUND, HC_TEXT)
    } else {
        (iced::Color::WHITE, iced::Color::WHITE)
    }
}

pub(crate) fn focus_ring_width(high_contrast: bool) -> f32 {
    if high_contrast { 4.0 } else { 3.0 }
}

/// How far the focus ring sits outside the button bounds.
pub(crate) const FOCUS_RING_OUTSET: f32 = 3.0;

pub(crate) const KEYMAP_LAYOUTS: &[&str] = &[
    "af", "al", "am", "ara", "at", "au", "az", "ba", "bd", "be", "bg", "br", "brai", "bt", "bw",
    "by", "ca", "cd", "ch", "cm", "cn", "cz", "de", "dk", "dz", "ee", "epo", "es", "et", "fi",
    "fo", "fr", "gb", "ge", "gh", "gn", "gr", "hr", "hu", "id", "ie", "il", "in", "iq", "ir", "is",
    "it", "jp", "jv", "ke", "kg", "kh", "kr", "kz", "la", "latam", "lk", "lt", "lv", "ma", "mao",
    "md", "me", "mk", "ml", "mm", "mn", "mt", "mv", "my", "ng", "nl", "no", "np", "ph", "pk", "pl",
    "pt", "ro", "rs", "ru", "se", "si", "sk", "sn", "sy", "tg", "th", "tj", "tm", "tr", "tw", "tz",
    "ua", "us", "uz", "vn", "za",
];

/// Resolve a keymap name to its entry in [`KEYMAP_LAYOUTS`].
///
/// Customization carries keymaps as `&'static str` borrowed from this table,
/// while the persisted config stores a plain `String`. Loading a saved keymap
/// back into the UI therefore has to come back through the table, or the field
/// silently reads as unset.
///
/// Returns `None` for a name not in the table, which is what a hand-edited
/// config file can contain.
pub(crate) fn keymap_layout(name: &str) -> Option<&'static str> {
    KEYMAP_LAYOUTS
        .binary_search(&name)
        .ok()
        .map(|i| KEYMAP_LAYOUTS[i])
}

#[cfg(test)]
mod tests {
    use super::{KEYMAP_LAYOUTS, keymap_layout};

    /// [`keymap_layout`] looks entries up with `binary_search`, so new entries
    /// need to be inserted in byte order.
    #[test]
    fn keymap_layouts_sorted() {
        assert!(KEYMAP_LAYOUTS.is_sorted());
    }

    #[test]
    fn keymap_layout_resolves_known_names() {
        assert_eq!(keymap_layout("us"), Some("us"));
        assert_eq!(keymap_layout("de"), Some("de"));
        // First and last entries, to pin the search bounds.
        assert_eq!(keymap_layout(KEYMAP_LAYOUTS[0]), Some(KEYMAP_LAYOUTS[0]));
        let last = KEYMAP_LAYOUTS[KEYMAP_LAYOUTS.len() - 1];
        assert_eq!(keymap_layout(last), Some(last));
    }

    /// A hand-edited config can name a layout that does not exist; it should
    /// read as unset rather than be forced into the picker.
    #[test]
    fn keymap_layout_rejects_unknown_names() {
        assert_eq!(keymap_layout("not-a-layout"), None);
        assert_eq!(keymap_layout(""), None);
        // Matching is exact: the table is lowercase, locale regions are not.
        assert_eq!(keymap_layout("US"), None);
    }
}
