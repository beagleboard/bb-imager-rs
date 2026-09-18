pub mod app_info;
pub mod board_selection;
pub mod configuration;
mod constants;
pub mod flash_fail;
pub mod flash_success;
mod helpers;
pub mod review;

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
    /// Text typed into a list pane's search box.
    UpdateSearchText(std::sync::Arc<str>),
    /// Open a URL in the browser.
    OpenUrl(url::Url),
    /// A remote icon finished downloading to the given path.
    ResolveImage(std::sync::Arc<url::Url>, std::path::PathBuf),

    EditorEvent(iced::widget::text_editor::Action),

    FlashStart,
    // Retry flashing
    Retry,
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
