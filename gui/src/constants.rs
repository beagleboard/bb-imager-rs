pub const DEFAULT_CONFIG: &[u8] = include_bytes!("../../config.json");
pub const APP_NAME: &str = "BeagleBoard Imager";

pub const WINDOW_ICON: &[u8] = include_bytes!("../icon.png");
pub const BB_BANNER: &[u8] = include_bytes!("../../assets/icons/bb-banner.png");
pub const ARROW_BACK_ICON: &[u8] = include_bytes!("../../assets/icons/arrow-back.svg");
pub const DOWNLOADING_ICON: &[u8] = include_bytes!("../../assets/icons/downloading.svg");
pub const FILE_ADD_ICON: &[u8] = include_bytes!("../../assets/icons/file-add.svg");
pub const USB_ICON: &[u8] = include_bytes!("../../assets/icons/usb.svg");
pub const REFRESH_ICON: &[u8] = include_bytes!("../../assets/icons/refresh.svg");

pub const BEAGLE_BOARD_ABOUT: &str = "The BeagleBoard.org Foundation is a Michigan, USA-based 501(c)(3) non-profit corporation existing to provide education in and collaboration around the design and use of open-source software and hardware in embedded computing. BeagleBoard.org provides a forum for the owners and developers of open-source software and hardware to exchange ideas, knowledge and experience. The BeagleBoard.org community collaborates on the development of open source physical computing solutions including robotics, personal manufacturing tools like 3D printers and laser cutters, and other types of industrial and machine controls.";

pub const FONT_REGULAR: iced::Font = iced::Font::with_name("Roboto");
pub const FONT_BOLD: iced::Font = {
    let mut font = FONT_REGULAR;
    font.weight = iced::font::Weight::Bold;

    font
};
pub const FONT_REGULAR_BYTES: &[u8] = include_bytes!("../../assets/fonts/Roboto-Regular.ttf");
pub const FONT_BOLD_BYTES: &[u8] = include_bytes!("../../assets/fonts/Roboto-Bold.ttf");
