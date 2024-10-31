pub mod flash;
pub mod configuration;
pub mod destination_selection;
pub mod board_selection;

#[derive(Default, Debug, Clone)]
pub enum Screen {
    #[default]
    Home,
    BoardSelection,
    ImageSelection,
    DestinationSelection,
    ExtraConfiguration,
    Flashing(flash::FlashingScreen),
}
