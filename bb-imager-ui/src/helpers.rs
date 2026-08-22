use std::time::Duration;

use bb_iced_widgets::cached_icon::Cache;
use iced::{Element, widget};

use crate::{
    Message,
    constants::{BEAGLEBOARD_LOGO, BOARD_ICON, FONT_BOLD, ISSUE_TRACKER, SEARCH_ICON},
};

pub(crate) fn card<'a, M: 'a>(img: Element<'a, M>, label: &'a str, cb: M) -> widget::Button<'a, M> {
    const BORDER_WIDTH: f32 = 3.0;

    widget::button(widget::column![
        widget::center(img).padding(6),
        widget::rule::horizontal(BORDER_WIDTH),
        widget::center(widget::text(label).font(FONT_BOLD))
            .height(iced::Shrink)
            .style(widget::container::transparent)
            .padding(6)
    ])
    .on_press(cb)
    .padding(BORDER_WIDTH)
    .style(card_btn_style)
}

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

pub(crate) fn sidebar<'a, D: Clone + 'a>(
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

/// Estimate the remaining flashing time from the current `progress` and how
/// much time has `elapsed` since the first progress update.
///
/// Split out of [`FlashingState::time_remaining`] so the ETA math is testable
/// without an `Instant` clock: a linear extrapolation `elapsed * (1 - x) / x`,
/// suppressed until progress clears a small threshold to avoid wild early
/// estimates.
pub(crate) fn time_remaining_from(
    progress: crate::flashing::Progress,
    elapsed: Option<Duration>,
) -> Option<Duration> {
    const THRESHOLD: f32 = 0.02;

    match progress {
        crate::flashing::Progress::Writing(x) => {
            if x < THRESHOLD {
                None
            } else {
                let t = elapsed?;
                let x = x.clamp(0.0, 1.0);
                let scale = (1.0 - x) / x;
                Some(t.mul_f32(scale))
            }
        }
        crate::flashing::Progress::Verifying | crate::flashing::Progress::Customizing => {
            Some(Duration::from_secs(1))
        }
        _ => None,
    }
}

pub(crate) fn pretty_duration(d: Duration) -> String {
    let secs = d.as_secs();

    if secs >= 60 {
        format!("{}:{:02}", secs / 60, secs % 60)
    } else {
        format!("{}s", secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flashing::Progress;
    use std::time::Duration;

    #[test]
    fn eta_scales_linearly_with_remaining_fraction() {
        // At 50% after 10s, the remaining half should take another ~10s.
        assert_eq!(
            time_remaining_from(Progress::Writing(0.5), Some(Duration::from_secs(10)),),
            Some(Duration::from_secs(10))
        );
        // At 25% after 10s, the remaining 75% extrapolates to 30s.
        assert_eq!(
            time_remaining_from(Progress::Writing(0.25), Some(Duration::from_secs(10)),),
            Some(Duration::from_secs(30))
        );
    }

    #[test]
    fn eta_uses_the_same_math_for_downloads() {
        assert_eq!(
            time_remaining_from(Progress::Writing(0.5), Some(Duration::from_secs(4)),),
            Some(Duration::from_secs(4))
        );
    }

    #[test]
    fn eta_suppressed_below_threshold() {
        // Below 2% the estimate is too noisy, so no ETA is reported.
        assert_eq!(
            time_remaining_from(Progress::Writing(0.01), Some(Duration::from_secs(10)),),
            None
        );
    }

    #[test]
    fn eta_requires_a_start_timestamp() {
        // Past the threshold but with no elapsed time recorded yet.
        assert_eq!(time_remaining_from(Progress::Writing(0.5), None), None);
    }

    #[test]
    fn eta_clamps_progress_above_one() {
        // A progress value >1.0 clamps to 1.0, yielding a zero remainder.
        assert_eq!(
            time_remaining_from(Progress::Writing(1.5), Some(Duration::from_secs(10)),),
            Some(Duration::ZERO)
        );
    }

    #[test]
    fn customizing_reports_fixed_estimate() {
        assert_eq!(
            time_remaining_from(Progress::Customizing, None),
            Some(Duration::from_secs(1))
        );
    }

    #[test]
    fn pretty_duration_formats_minutes_and_seconds() {
        assert_eq!(pretty_duration(Duration::from_secs(0)), "0s");
        assert_eq!(pretty_duration(Duration::from_secs(45)), "45s");
        assert_eq!(pretty_duration(Duration::from_secs(60)), "1:00");
        assert_eq!(pretty_duration(Duration::from_secs(125)), "2:05");
    }
}
