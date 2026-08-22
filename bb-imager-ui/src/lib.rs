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
pub mod img_selection;
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

    // Select board. Only valid in DevicePage
    SelectBoardById(i64),

    SelectOs(img_selection::ImageId),
    GotoOsListParent,

    SelectDestination(D),
    SelectFileDest(std::sync::Arc<str>),
    DestinationFilter(bool),

    UpdateCustomizaton(customization::Customization),
    SelectInitFormat(bb_config::config::InitFormat),
    Next,

    FlashStart,

    FlashCancel,

    Retry,

    Save,

    UpdateSearchText(std::sync::Arc<str>),
    ResolveImage(std::sync::Arc<url::Url>, std::path::PathBuf),
    /// Open URL in browser
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
