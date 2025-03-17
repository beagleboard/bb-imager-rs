pub(crate) mod board_selection;
pub(crate) mod configuration;
pub(crate) mod destination_selection;
pub(crate) mod flash;
pub(crate) mod home;
pub(crate) mod image_selection;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub(crate) enum Screen {
    #[default]
    Home,
    BoardSelection,
    ImageSelection(image_selection::ImageSelectionPage),
    DestinationSelection,
    ExtraConfiguration,
    Flashing,
    FlashingConfirmation,
}
