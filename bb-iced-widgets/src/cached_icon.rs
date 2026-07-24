use std::{collections::HashMap, hash::Hash, path::PathBuf};

use iced::Length;

#[derive(Debug)]
pub struct Cache<K: Eq + Hash>(HashMap<K, Option<crate::icon::Handle>>);

impl<K: Eq + Hash> Default for Cache<K> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl<K: Eq + Hash> Cache<K> {
    pub fn get(&self, u: &K) -> Option<&crate::icon::Handle> {
        self.0.get(u)?.as_ref()
    }

    pub fn insert(&mut self, u: K, path: PathBuf) {
        self.0.insert(u, Some(path.into()));
    }

    pub fn mark_fetching(&mut self, u: K) {
        self.0.insert(u, None);
    }

    pub fn contains(&self, u: &K) -> bool {
        self.0.contains_key(u)
    }
}

pub enum CachedIcon<'a> {
    Icon(crate::icon::Icon<'a>),
    Spinner(iced_aw::Spinner),
}

impl<'a> CachedIcon<'a> {
    pub fn new<K: Eq + Hash>(cache: &Cache<K>, key: &K) -> Self {
        match cache.get(key) {
            Some(v) => Self::Icon(crate::icon(v.clone())),
            None => Self::Spinner(iced_aw::Spinner::new()),
        }
    }

    pub fn width(self, width: impl Into<Length>) -> Self {
        match self {
            CachedIcon::Icon(icon) => Self::Icon(icon.width(width)),
            CachedIcon::Spinner(spinner) => Self::Spinner(spinner.width(width)),
        }
    }

    pub fn height(self, height: impl Into<Length>) -> Self {
        match self {
            CachedIcon::Icon(icon) => Self::Icon(icon.height(height)),
            CachedIcon::Spinner(spinner) => Self::Spinner(spinner.height(height)),
        }
    }
}

impl<'a, M> From<CachedIcon<'a>> for iced::Element<'a, M> {
    fn from(value: CachedIcon<'a>) -> Self {
        match value {
            CachedIcon::Icon(icon) => icon.into(),
            CachedIcon::Spinner(spinner) => spinner.into(),
        }
    }
}
