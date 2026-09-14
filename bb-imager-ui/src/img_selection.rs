use std::borrow::Cow;

use bb_config::config;
use bb_iced_widgets::cached_icon::Cache;
use iced::widget;

use crate::constants::ARROW_FORWARD_ICON;
use crate::helpers::{
    layout_with_search, list_item, network_image_or_default, page_layout, pretty_bytes, svg,
};
use crate::{Message, constants};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageId {
    Format,
    // points to parent
    Local(config::Flasher),
    // points to OsImage
    OsImage(i64),
    OsSublist((i64, config::Flasher)),
}

#[derive(Debug, Clone)]
pub struct ImageItem {
    pub id: ImageId,
    pub label: Cow<'static, str>,
    pub icon: Option<std::sync::Arc<url::Url>>,
    pub description: Cow<'static, str>,
    pub size: Option<u64>,
    pub release_date: Option<chrono::NaiveDate>,
}

impl<D> From<&ImageItem> for Message<D> {
    fn from(value: &ImageItem) -> Self {
        Self::SelectOs(value.id)
    }
}

impl ImageItem {
    fn icon<'a, D: 'a>(
        &'a self,
        cache: &'a Cache<std::sync::Arc<url::Url>>,
    ) -> iced::Element<'a, Message<D>> {
        match self.id {
            ImageId::Format => svg(constants::FORMAT_ICON.clone()),
            ImageId::Local(_) => svg(constants::FILE_ADD_ICON.clone()),
            _ => network_image_or_default(cache, self.icon.as_ref()),
        }
    }

    /// Sublists open another list instead of selecting an image, so they get a chevron.
    fn trailing<'a, D: 'a>(&self) -> Option<iced::Element<'a, Message<D>>> {
        const SIZE: u32 = 16;

        matches!(self.id, ImageId::OsSublist(_)).then(|| {
            widget::svg(ARROW_FORWARD_ICON.clone())
                .width(SIZE)
                .height(SIZE)
                .style(|t: &iced::Theme, _| widget::svg::Style {
                    color: Some(t.extended_palette().background.strong.color),
                })
                .into()
        })
    }
}

#[derive(Default, Debug)]
pub struct State {
    pub imgs: Box<[ImageItem]>,
    pub pos: Option<i64>,
    pub search: std::sync::Arc<str>,
}

pub fn view<'a, D: Clone + 'a>(
    cache: &'a Cache<std::sync::Arc<url::Url>>,
    s: &'a State,
    scroll_id: widget::Id,
) -> iced::Element<'a, Message<D>> {
    let columns = if s.pos.is_none() {
        widget::column([])
    } else {
        widget::column![
            list_item(
                svg(constants::ARROW_BACK_ICON.clone()),
                "Back",
                Vec::new(),
                None
            )
            .on_press(Message::GotoOsListParent)
        ]
    };

    let list = s.imgs.iter().map(|x| {
        let mut rows = Vec::new();

        if !x.description.is_empty() {
            rows.push(widget::text(&x.description).into());
        }

        if x.size.is_some() || x.release_date.is_some() {
            let mut last_row = widget::row([]);

            if let Some(size) = x.size {
                last_row = last_row.push(widget::rich_text![
                    widget::span::<'_, (), _>("Size: ").font(constants::FONT_BOLD),
                    widget::span(pretty_bytes(size))
                ]);
            }

            if let Some(r) = x.release_date {
                last_row = last_row.extend([
                    widget::space().width(iced::Fill).into(),
                    widget::rich_text![
                        widget::span::<'_, (), _>("Release Date: ").font(constants::FONT_BOLD),
                        widget::span(r.to_string())
                    ]
                    .into(),
                ]);
            }

            rows.push(last_row.into());
        }

        list_item(x.icon(cache), &x.label, rows, x.trailing())
            .on_press(x.into())
            .into()
    });

    page_layout(
        (
            [
                ("Device", false, Some(Message::GotoDevicePage)),
                ("Software", true, None),
            ],
            [("App Options", false, Some(Message::GotoAppOptions))],
        ),
        layout_with_search(&s.search, columns.extend(list).spacing(8), scroll_id),
    )
}
