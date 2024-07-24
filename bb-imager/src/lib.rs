pub mod bcf;
pub mod sd;
pub mod config;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Status {
    Preparing,
    Flashing,
    FlashingProgress(f32),
    Verifying,
    VerifyingProgress(f32),
    Finished,
}
