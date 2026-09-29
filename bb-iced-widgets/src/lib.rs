pub mod cached_icon;
pub mod icon;
pub mod progress_circle;

pub fn progress_circle<T>(
    progress: f32,
    thickness: impl Into<f32>,
    color: iced::Color,
    font: iced::Font,
) -> iced::widget::Canvas<progress_circle::ProgressCircle, T> {
    progress_circle::ProgressCircle::new(progress, thickness, color, font)
}

pub fn icon<'a>(handle: impl Into<icon::Handle>) -> icon::Icon<'a> {
    icon::Icon::new(handle)
}

pub fn cached_icon<'a, M, K: Eq + std::hash::Hash>(
    cache: &cached_icon::Cache<K>,
    key: &K,
) -> cached_icon::CachedIcon<'a, M> {
    cached_icon::CachedIcon::new(cache, key)
}
