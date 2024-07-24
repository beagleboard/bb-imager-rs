pub mod bcf;
pub mod sd;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Status {
    Preparing,
    Flashing,
    FlashingProgress(f32),
    Verifying,
    VerifyingProgress(f32),
    Finished,
}
