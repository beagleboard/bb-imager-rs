pub mod board_selection;
pub mod configuration;
pub mod destination_selection;
pub mod flash;
pub mod home;
pub mod image_selection;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Home,
    BoardSelection,
    ImageSelection,
    DestinationSelection,
    ExtraConfiguration,
    Flashing,
    FlashingConfirmation,
}
