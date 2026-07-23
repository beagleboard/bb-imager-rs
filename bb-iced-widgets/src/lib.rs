pub mod progress_circle;

pub fn progress_circle<T>(
    progress: f32,
    thickness: impl Into<f32>,
    color: iced::Color,
    font: iced::Font,
) -> iced::widget::Canvas<progress_circle::ProgressCircle, T> {
    progress_circle::ProgressCircle::new(progress, thickness, color, font)
}
