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
    use super::KEYMAP_LAYOUTS;

    /// The keymap combo box looks up its selection with `binary_search`, so new
    /// entries need to be inserted in byte order.
    #[test]
    fn keymap_layouts_sorted() {
        assert!(KEYMAP_LAYOUTS.is_sorted());
    }
}
