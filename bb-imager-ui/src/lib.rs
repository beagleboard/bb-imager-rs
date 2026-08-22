pub mod app_options;
pub mod board_selection;
pub(crate) mod constants;
pub mod customization;
pub mod dest_selection;
pub mod flash_cancel;
pub mod flash_fail;
pub mod flash_success;
pub mod flashing;
pub(crate) mod helpers;
pub mod image_selection;
pub mod review;

#[derive(Debug, Clone)]
pub enum Message<D> {
    // Messages to go to specific pages.
    GotoDevicePage,
    GotoSoftwarePage,
    GotoDestinationPage,
    GotoCustomizationPage,
    GotoReviewPage,
    GotoFlashingPage,
    GotoAppOptions,

    /// Select a board by its id. Only valid on the board selection page.
    SelectBoardById(i64),

    /// Select an OS image, or open a sublist. Only valid on the image page.
    SelectOs(image_selection::ImageId),
    /// Leave the current OS sublist for its parent.
    GotoOsListParent,

    SelectDestination(D),
    /// Open a save dialog for the image, with this suggested file name.
    SelectFileDest(std::sync::Arc<str>),
    /// Whether to hide destinations that are probably not removable media.
    DestinationFilter(bool),

    UpdateCustomizaton(customization::Customization),
    SelectInitFormat(bb_config::config::InitFormat),
    Next,
    /// Reset the customization back to its defaults.
    Reset,

    FlashStart,

    /// Cancel an in-progress flash.
    FlashCancel,

    // Retry flashing
    Retry,

    /// Text typed into a list pane's search box.
    UpdateSearchText(std::sync::Arc<str>),
    /// A remote icon finished downloading to the given path.
    ResolveImage(std::sync::Arc<url::Url>, std::path::PathBuf),
    /// Open a URL in the browser.
    OpenUrl(url::Url),
    CopyToClipboard,
    EditorEvent(iced::widget::text_editor::Action),
    Null,
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
            background: iced::Color::WHITE,
            text: iced::Color::BLACK,
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
