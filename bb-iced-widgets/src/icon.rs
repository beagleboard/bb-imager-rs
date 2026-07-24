use std::{io::Read, path::PathBuf};

use iced::{Length, widget};

#[derive(Debug, Clone)]
pub enum IconHandle {
    Svg(widget::svg::Handle),
    Img(widget::image::Handle),
}

impl From<PathBuf> for IconHandle {
    fn from(value: PathBuf) -> Self {
        let mut magic = [0u8; 32];
        {
            let mut f = std::fs::File::open(&value).expect("Failed to open image");
            f.read_exact(&mut magic).unwrap();
        };
        match image::guess_format(&magic) {
            Ok(_) => widget::image::Handle::from_path(value).into(),
            Err(_) => widget::svg::Handle::from_path(value).into(),
        }
    }
}

impl From<widget::svg::Handle> for IconHandle {
    fn from(value: widget::svg::Handle) -> Self {
        Self::Svg(value)
    }
}

impl From<widget::image::Handle> for IconHandle {
    fn from(value: widget::image::Handle) -> Self {
        Self::Img(value)
    }
}

pub enum Icon<'a> {
    Svg(widget::Svg<'a>),
    Img(widget::Image),
}

impl<'a> Icon<'a> {
    pub fn new(handle: impl Into<IconHandle>) -> Self {
        match handle.into() {
            IconHandle::Svg(handle) => Self::Svg(widget::svg(handle)),
            IconHandle::Img(handle) => Self::Img(widget::image(handle)),
        }
    }

    pub fn height(self, height: impl Into<Length>) -> Self {
        match self {
            Icon::Svg(svg) => Self::Svg(svg.height(height)),
            Icon::Img(image) => Self::Img(image.height(height)),
        }
    }

    pub fn width(self, width: impl Into<Length>) -> Self {
        match self {
            Icon::Svg(svg) => Self::Svg(svg.width(width)),
            Icon::Img(image) => Self::Img(image.width(width)),
        }
    }
}

impl<'a, M> From<Icon<'a>> for iced::Element<'a, M> {
    fn from(value: Icon<'a>) -> Self {
        match value {
            Icon::Svg(svg) => svg.into(),
            Icon::Img(image) => image.into(),
        }
    }
}
