pub mod bcf;
pub mod sd;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Status {
    Preparing,
    Flashing(f32),
    Verifying,
    Finished,
}
