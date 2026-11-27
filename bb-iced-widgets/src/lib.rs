pub mod cached_icon;
pub mod double_click;
pub mod icon;

pub fn icon<'a>(handle: impl Into<icon::Handle>) -> icon::Icon<'a> {
    icon::Icon::new(handle)
}

pub fn cached_icon<'a, M, K: Eq + std::hash::Hash>(
    cache: &cached_icon::Cache<K>,
    key: &K,
) -> cached_icon::CachedIcon<'a, M> {
    cached_icon::CachedIcon::new(cache, key)
}

pub fn double_click<'a, Message, Theme, Renderer>(
    content: impl Into<iced_widget::core::Element<'a, Message, Theme, Renderer>>,
    on_double_click: Message,
) -> double_click::DoubleClickArea<'a, Message, Theme, Renderer> {
    double_click::DoubleClickArea::new(content, on_double_click)
}
