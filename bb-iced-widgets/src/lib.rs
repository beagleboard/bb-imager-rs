pub mod cached_icon;
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
