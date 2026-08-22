pub(crate) const LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/beagleboard/bb-imager-rs/releases/latest";

pub(crate) const PACKAGE_QUALIFIER: (&str, &str, &str) = ("org", "beagleboard", "imagingutility");

pub(crate) const DEFAULT_CONFIG: &[u8] = include_bytes!("../../config.json");
pub(crate) const APP_NAME: &str = "BeagleBoard Imager";
pub(crate) const APP_LINCESE: &str = include_str!("../../LICENSE");

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
/// The customization UI carries keymaps as `&'static str` borrowed from this
/// table, while the persisted config stores a plain `String`. Loading a saved
/// keymap back into the UI therefore has to come back through the table, or the
/// field silently reads as unset.
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
