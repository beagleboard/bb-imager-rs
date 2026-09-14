use bb_iced_widgets::cached_icon::Cache;
use iced::{Element, widget};

use crate::{
    Message,
    constants::{
        BEAGLEBOARD_LOGO, BOARD_ICON, CYCLE_ICON, FONT_BOLD, ISSUE_TRACKER, QUICK_REFERENCE_ICON,
        REPORT_ICON, SEARCH_ICON, SETTINGS_ICON,
    },
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

pub(crate) fn page_layout<'a, D: Clone + 'a>(
    sidebar_items: (
        impl IntoIterator<Item = (&'static str, bool, Option<Message<D>>)>,
        impl IntoIterator<Item = (&'static str, bool, Option<Message<D>>)>,
    ),
    main: impl Into<Element<'a, Message<D>>>,
) -> Element<'a, Message<D>> {
    widget::row![sidebar(), main.into()]
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

fn sidebar_item<'a, D: 'a>(
    icon: impl Into<iced::Element<'a, Message<D>>>,
    label: &'static str,
    cb: Option<Message<D>>,
    style: impl Fn(&iced::Theme, widget::button::Status) -> widget::button::Style + 'a,
) -> widget::Button<'a, Message<D>> {
    widget::button(
        widget::row![icon.into(), widget::text(label).font(FONT_BOLD)]
            .spacing(4)
            .height(iced::Fill)
            .align_y(iced::Center),
    )
    .width(iced::Fill)
    .height(iced::Fill)
    .on_press_maybe(cb)
    .style(style)
}

pub(crate) fn sidebar<'a, D: 'a + Clone>() -> iced::Element<'a, Message<D>> {
    widget::container(widget::column![
        sidebar_item(circle(1, 10), "Hardware", None, widget::button::primary),
        widget::rule::horizontal(2),
        sidebar_item(circle(2, 10), "Software", None, widget::button::text),
        widget::rule::horizontal(2),
        sidebar_item(circle(3, 10), "Modify", None, widget::button::text),
        widget::rule::horizontal(2),
        sidebar_item(circle(4, 10), "Storage", None, widget::button::text),
        widget::rule::horizontal(2),
        sidebar_item(circle(5, 10), "Write", None, widget::button::text),
        widget::rule::horizontal(2),
        sidebar_item(
            widget::svg(CYCLE_ICON.clone()).width(20),
            "Format Media",
            Some(Message::GotoFormatPage),
            widget::button::warning
        ),
        widget::rule::horizontal(2),
        sidebar_item(
            widget::svg(SETTINGS_ICON.clone()).width(20),
            "App Options",
            Some(Message::GotoAppOptions),
            widget::button::warning
        ),
        widget::rule::horizontal(2),
        sidebar_item(
            widget::svg(QUICK_REFERENCE_ICON.clone()).width(20),
            "Usage Guide",
            Some(Message::GotoUsageGuide),
            widget::button::warning
        ),
        widget::rule::horizontal(2),
        sidebar_item(
            widget::svg(REPORT_ICON.clone()).width(20),
            "Issue Tracker",
            Some(Message::OpenUrl(ISSUE_TRACKER.clone())),
            widget::button::warning
        ),
        widget::rule::horizontal(2),
        widget::center(widget::svg(BEAGLEBOARD_LOGO.clone())).padding(8)
    ])
    .width(150)
    .style(|t| {
        let mut temp = widget::container::bordered_box(t);
        temp.border = temp.border.width(3);
        temp
    })
    .into()
}

fn circle<'a, M>(count: u8, radius: u8) -> widget::Container<'a, M> {
    let width = radius as u32 * 2;
    widget::center(widget::text(count).font(FONT_BOLD))
        .width(width)
        .height(width)
        .style(move |t: &iced::Theme| {
            let mut temp = widget::container::background(t.palette().background);
            temp.border = temp.border.rounded(radius);
            temp
        })
}
