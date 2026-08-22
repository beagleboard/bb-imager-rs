use std::time::{Duration, Instant};

use iced::widget;

use crate::Message;
use crate::constants::FONT_BOLD;
use crate::helpers::page_layout;

#[derive(Clone, Copy, Debug, Default)]
pub enum Progress {
    #[default]
    Preparing,
    Writing(f32),
    Verifying,
    Customizing,
}

#[derive(Debug, Default)]
pub struct State {
    pub has_customization: bool,
    pub progress: Progress,
    pub start_timestamp: Option<Instant>,
}

pub fn view<'a, D: Clone + 'a>(s: &'a State) -> iced::Element<'a, Message<D>> {
    let mut sidebar: Vec<(_, _, Option<Message<D>>)> = vec![
        ("Device", false, None),
        ("Software", false, None),
        ("Destination", false, None),
    ];

    if s.has_customization {
        sidebar.push(("Customization", false, None));
    }

    sidebar.extend([("Review", false, None), ("Flashing", true, None)]);

    let (progress_label, progress_bar) = match s.progress {
        Progress::Preparing => (
            widget::text("Preparing..."),
            widget::progress_bar(0.0..=1.0, 0.0),
        ),
        Progress::Writing(f) => (
            widget::text(format!("Writing... ({}%)", (f * 100.0) as u8)),
            widget::progress_bar(0.0..=1.0, f),
        ),
        Progress::Verifying => (
            widget::text("Verifying..."),
            widget::progress_bar(0.0..=1.0, 0.99),
        ),
        Progress::Customizing => (
            widget::text("Customizing..."),
            widget::progress_bar(0.0..=1.0, 0.99),
        ),
    };

    let time_remaining =
        match time_remaining_from(s.progress, s.start_timestamp.map(|t| t.elapsed())) {
            Some(x) => widget::span::<'_, (), _>(pretty_duration(x)),
            None => widget::span("Calculating"),
        };

    page_layout(
        (
            sidebar,
            [("App Options", false, Some(Message::GotoAppOptions))],
        ),
        widget::column![
            widget::column![
                widget::text("Write Image").font(FONT_BOLD).size(26),
                widget::text("Do not disconnect the storage device!").style(widget::text::danger),
                progress_label
                    .font(FONT_BOLD)
                    .style(widget::text::secondary),
                progress_bar,
                widget::rich_text![
                    widget::span("Time Remaining: ").font(FONT_BOLD),
                    time_remaining
                ]
            ]
            .height(iced::Fill)
            .spacing(16)
            .padding(iced::Padding::ZERO.horizontal(16)),
            widget::rule::horizontal(2),
            widget::right(
                widget::button("CANCEL")
                    .style(widget::button::danger)
                    .on_press(Message::FlashCancel)
            )
            .padding(iced::Padding::ZERO.horizontal(16))
        ]
        .spacing(16)
        .padding(iced::Padding::ZERO.vertical(16)),
    )
}

fn pretty_duration(d: Duration) -> String {
    let secs = d.as_secs();

    if secs >= 60 {
        format!("{}:{:02}", secs / 60, secs % 60)
    } else {
        format!("{}s", secs)
    }
}

/// Estimate the remaining flashing time from the current `progress` and how
/// much time has `elapsed` since the first progress update.
fn time_remaining_from(
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
