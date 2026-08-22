use bb_iced_widgets::cached_icon::Cache;
use iced::{Element, widget};

use crate::{
    Message,
    constants::{BEAGLEBOARD_LOGO, BOARD_ICON, FONT_BOLD, ISSUE_TRACKER, SEARCH_ICON},
};

pub(crate) fn card_btn_style(t: &iced::Theme, s: widget::button::Status) -> widget::button::Style {
    const BORDER_RADIUS: f32 = 10.0;
    const BORDER_WIDTH: f32 = 3.0;

    let mut style = widget::button::text(t, s);

    let border = match s {
        widget::button::Status::Hovered => t.palette().primary,
        _ => t.extended_palette().background.weak.color,
    };

    style.border = style
        .border
        .rounded(BORDER_RADIUS)
        .color(border)
        .width(BORDER_WIDTH);
    style
}

pub(crate) fn search_box<'a, D: Clone + 'a>(inp: &'a str) -> widget::Container<'a, Message<D>> {
    widget::container(
        widget::row![
            widget::svg(SEARCH_ICON.clone())
                .width(iced::Length::Shrink)
                .height(18),
            widget::text_input("SEARCH", inp)
                .style(|theme, status| {
                    let mut temp = widget::text_input::default(theme, status);
                    temp.border.width = 0.0;
                    temp.background = iced::Background::Color(iced::Color::TRANSPARENT);
                    temp
                })
                .on_input(|x| Message::UpdateSearchText(x.into())),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding(iced::Padding {
        left: 16.0,
        top: 16.0,
        bottom: 8.0,
        ..Default::default()
    })
}

pub(crate) fn network_image_or_default<'a, M: 'a>(
    cache: &'a Cache<std::sync::Arc<url::Url>>,
    u: Option<&'a std::sync::Arc<url::Url>>,
) -> Element<'a, M> {
    match u {
        Some(x) => bb_iced_widgets::cached_icon(cache, x)
            .width(iced::Fill)
            .height(iced::Fill)
            .into(),
        None => widget::svg(BOARD_ICON.clone()).height(iced::Fill).into(),
    }
}

fn sidebar<'a, D: Clone + 'a>(
    top_items: impl IntoIterator<Item = (&'static str, bool, Option<Message<D>>)>,
    bottom_items: impl IntoIterator<Item = (&'static str, bool, Option<Message<D>>)>,
) -> Element<'a, Message<D>> {
    let cb = |(label, is_active, msg)| {
        if is_active {
            widget::container(label)
                .width(iced::Fill)
                .height(iced::Shrink)
                .padding(8)
                .style(|t| {
                    let mut s = widget::container::primary(t);
                    s.border = s.border.rounded(6);
                    s
                })
                .into()
        } else {
            widget::button(label)
                .on_press_maybe(msg)
                .width(iced::Fill)
                .height(iced::Shrink)
                .padding(8)
                .style(widget::button::subtle)
                .into()
        }
    };

    widget::column![
        widget::column(top_items.into_iter().map(cb))
            .height(iced::Fill)
            .spacing(8)
            .padding(8),
        widget::rule::horizontal(2),
        widget::column(
            bottom_items.into_iter().map(cb).chain([
                widget::button("Issue Tracker")
                    .on_press(Message::OpenUrl(ISSUE_TRACKER.clone()))
                    .width(iced::Fill)
                    .height(iced::Shrink)
                    .padding(8)
                    .style(widget::button::subtle)
                    .into(),
                widget::svg(BEAGLEBOARD_LOGO.clone()).into()
            ])
        )
        .padding(8)
        .spacing(8)
    ]
    .width(150)
    .spacing(8)
    .into()
}

pub(crate) fn page_layout<'a, D: Clone + 'a>(
    sidebar_items: (
        impl IntoIterator<Item = (&'static str, bool, Option<Message<D>>)>,
        impl IntoIterator<Item = (&'static str, bool, Option<Message<D>>)>,
    ),
    main: impl Into<Element<'a, Message<D>>>,
) -> Element<'a, Message<D>> {
    widget::row![
        sidebar(sidebar_items.0, sidebar_items.1),
        widget::rule::vertical(2),
        main.into()
    ]
    .width(iced::Fill)
    .height(iced::Fill)
    .into()
}

pub(crate) fn layout_with_search<'a, D: Clone + 'a>(
    search: &'a str,
    main: impl Into<Element<'a, Message<D>>>,
    scroll_id: widget::Id,
) -> Element<'a, Message<D>> {
    widget::column![
        search_box(search),
        widget::rule::horizontal(2),
        widget::scrollable(widget::container(main).padding(iced::Padding::from(15).right(20)))
            .id(scroll_id)
    ]
    .width(iced::Fill)
    .into()
}

/// A row in one of the selection lists.
///
/// `trailing` is pinned to the right edge of the row; see [`chevron`] for the
/// cue used by entries that open another list instead of selecting something.
pub(crate) fn list_item<'a, M: 'a>(
    icon: impl Into<iced::Element<'a, M>>,
    label: impl widget::text::IntoFragment<'a>,
    rows: Vec<iced::Element<'a, M>>,
    trailing: Option<iced::Element<'a, M>>,
) -> widget::Button<'a, M> {
    const ICON_WIDTH: u32 = 60;

    let info = widget::column![widget::text(label).font(FONT_BOLD).size(16)]
        .width(iced::Fill)
        .extend(rows);

    widget::button(
        widget::row![
            widget::container(icon.into())
                .width(ICON_WIDTH)
                .height(ICON_WIDTH),
            info
        ]
        .extend(trailing)
        .spacing(12)
        .padding(8)
        .align_y(iced::alignment::Vertical::Center),
    )
    .style(card_btn_style)
}

pub(crate) fn pretty_bytes(bytes: u64) -> String {
    const UNITS: [&str; 7] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit = 0;

    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{:.2} {}", size, UNITS[unit])
    }
}

pub(crate) fn svg<'a, M>(h: widget::svg::Handle) -> iced::Element<'a, M> {
    widget::svg(h).width(iced::Fill).height(iced::Fill).into()
}
