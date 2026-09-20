pub mod app_info;
pub mod board_selection;
pub mod configuration;
mod constants;
pub mod destination_selection;
pub mod flash_cancel;
pub mod flash_fail;
pub mod flash_success;
pub mod flashing;
mod helpers;
pub mod image_selection;
pub mod review;
pub mod sandbox_notice;

#[derive(Clone, Debug)]
pub enum Message {
    GotoAppInfo,

    Null,
    Back,
    Next,
    Restart,
    /// Reset the customization back to its defaults.
    Reset,

    UpdateCustomization(configuration::Customization),

    /// Select a board by its id. Only valid on the board selection page.
    SelectBoardById(i64),
    /// Copy a board's config entry, looked up by id, to the clipboard.
    CopyBoardConfig(i64),
    /// Copy an OS image's config entry, looked up by id, to the clipboard.
    CopyImageConfig(i64),
    /// Select an OS image, or open a sublist. Only valid on the image page.
    SelectOs(image_selection::ImageId),
    /// Leave the current OS sublist for its parent.
    GotoOsListParent,
    /// Select an enumerated destination, by its device identifier.
    SelectDest(Box<str>),
    /// Open a save dialog for the image, with this suggested file name.
    SelectFileDest(std::sync::Arc<str>),
    /// Whether to hide destinations that are probably not removable media.
    DestinationFilter(bool),
    /// Pick which init format the selected image is customized with.
    UpdateInitFormat(bb_config::config::InitFormat),
    /// Text typed into a list pane's search box.
    UpdateSearchText(std::sync::Arc<str>),
    /// Open a URL in the browser.
    OpenUrl(url::Url),
    /// A remote icon finished downloading to the given path.
    ResolveImage(std::sync::Arc<url::Url>, std::path::PathBuf),

    EditorEvent(iced::widget::text_editor::Action),

    FlashStart,
    /// Cancel an in-progress flash.
    FlashCancel,
    // Retry flashing
    Retry,
    /// The user dismissed the first-run udev notice on a sandboxed install.
    DismissSandboxNotice,
}

pub fn application<A>(
    app: iced::Application<A>,
) -> iced::Application<
    impl iced::Program<State = A::State, Message = A::Message, Theme = iced::Theme>,
>
where
    A: iced::Program<Theme = iced::Theme>,
{
    let theme = iced::Theme::custom(
        "Beagle",
        iced::theme::Palette {
            background: constants::BACKGROUND,
            text: iced::Color::WHITE,
            primary: constants::TONGUE_ORANGE,
            success: constants::CHECK_MARK_GREEN,
            warning: constants::HAIR_LIGHT_BROWN,
            danger: constants::DANGER,
        },
    );

    let icon = iced::window::icon::from_file_data(constants::WINDOW_ICON_BYTES, None).ok();
    assert!(icon.is_some());
    let settings = iced::window::Settings {
        min_size: Some(iced::Size::new(680.0, 450.0)),
        size: iced::Size::new(680.0, 450.0),
        icon,
        ..Default::default()
    };

    app.theme(theme)
        .font(constants::FONT_NORMAL_BYTES)
        .font(constants::FONT_BOLD_BYTES)
        .default_font(constants::FONT_REGULAR)
        .window(settings)
}
