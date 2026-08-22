use iced::widget;

use crate::Message;
use crate::helpers::{card, layout_with_search, network_image_or_default, page_layout};

#[derive(Default, Debug, Clone)]
pub struct Board {
    pub id: i64,
    /// `Arc` so that cloning into the icon cache/fetch tasks is a refcount bump
    /// rather than a `Url` clone, and so the item stays small.
    pub icon: Option<std::sync::Arc<url::Url>>,
    pub name: Box<str>,
}

#[derive(Default, Debug)]
pub struct State {
    pub boards: Box<[Board]>,
    pub search: std::sync::Arc<str>,
}

pub fn view<'a, D: Clone + 'a>(
    cache: &'a bb_iced_widgets::cached_icon::Cache<std::sync::Arc<url::Url>>,
    s: &'a State,
    scroll_id: widget::Id,
) -> iced::Element<'a, Message<D>> {
    let grid = s.boards.iter().map(|x| {
        card(
            network_image_or_default(cache, x.icon.as_ref()),
            &x.name,
            Message::SelectBoardById(x.id),
        )
        .height(iced::Fill)
        .into()
    });

    page_layout(
        (
            [("Device", true, None)],
            [("App Options", false, Some(Message::GotoAppOptions))],
        ),
        layout_with_search(
            &s.search,
            widget::grid(grid)
                .height(widget::grid::aspect_ratio(4, 3))
                .fluid(300)
                .spacing(18),
            scroll_id,
        ),
    )
}
