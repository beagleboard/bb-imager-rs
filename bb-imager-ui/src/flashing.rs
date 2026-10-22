use std::time::{Duration, Instant};

use iced::{Element, widget};

use crate::helpers::{SidebarEntry, VIEW_COL_PADDING, page_type2};
use crate::{Message, constants};

/// How far along the flash is.
///
/// Mirrors the flasher's own status enum; the host converts.
#[derive(Clone, Copy, Debug, Default)]
pub enum Progress {
    #[default]
    Preparing,
    Downloading(f32),
    Flashing(f32),
    Verifying,
    Customizing,
}

impl Progress {
    /// How far along this is, for comparing destinations.
    const fn fraction(self) -> f32 {
        match self {
            Progress::Preparing => 0.0,
            Progress::Downloading(x) | Progress::Flashing(x) => x,
            Progress::Verifying | Progress::Customizing => 0.99,
        }
    }
}

/// One destination being written.
#[derive(Debug)]
pub struct DestProgress {
    pub label: Box<str>,
    pub progress: Progress,
    /// `Some` once this destination has finished, with its error if it failed.
    pub result: Option<Result<(), Box<str>>>,
}

impl DestProgress {
    pub fn new(label: Box<str>) -> Self {
        Self {
            label,
            progress: Progress::default(),
            result: None,
        }
    }
}

#[derive(Debug)]
pub struct State {
    /// Never empty.
    pub dests: Box<[DestProgress]>,
    /// Stamped by the host on the first byte-moving update; the ETA is
    /// extrapolated from how long has elapsed since.
    pub start_timestamp: Option<Instant>,
}

impl State {
    /// The furthest-behind destination still being written, which the ETA is based on.
    fn slowest(&self) -> Option<Progress> {
        self.dests
            .iter()
            .filter(|x| x.result.is_none())
            .map(|x| x.progress)
            .min_by(|a, b| a.fraction().total_cmp(&b.fraction()))
    }
}

pub fn view<'a>(state: &'a State, scroll_id: widget::Id) -> Element<'a, Message> {
    page_type2(
        SidebarEntry::Write,
        // Every step is locked while flashing, Modify included.
        false,
        true,
        progress_view(state, scroll_id),
        [widget::button("Cancel")
            .style(widget::button::danger)
            .on_press(Message::FlashCancel)],
    )
}

fn progress_view(state: &State, scroll_id: widget::Id) -> Element<'_, Message> {
    let time_remaining = match state
        .slowest()
        .and_then(|p| time_remaining_from(p, state.start_timestamp.map(|t| t.elapsed())))
    {
        Some(x) => widget::span::<'_, (), _>(pretty_duration(x)),
        None => widget::span("Calculating"),
    };

    let (warning, progress): (_, Element<'_, Message>) = match state.dests.as_ref() {
        [dest] => {
            let (label, bar) = progress_row(dest.progress);
            (
                "Do not disconnect the storage device!",
                widget::column![
                    label
                        .font(constants::FONT_BOLD)
                        .style(widget::text::secondary),
                    bar
                ]
                .spacing(16)
                .into(),
            )
        }
        dests => (
            "Do not disconnect the storage devices!",
            widget::column(dests.iter().map(dest_view))
                .spacing(16)
                .into(),
        ),
    };

    widget::scrollable(
        widget::column![
            widget::text("Writing Image")
                .font(constants::FONT_BOLD)
                .size(26),
            widget::text(warning).style(widget::text::danger),
            widget::rule::horizontal(2),
            progress,
            widget::rich_text![
                widget::span("Time Remaining: ").font(constants::FONT_BOLD),
                time_remaining
            ]
        ]
        .padding(VIEW_COL_PADDING)
        .spacing(16),
    )
    .id(scroll_id)
    .into()
}

/// One destination's row when several are written at once.
fn dest_view(dest: &DestProgress) -> Element<'_, Message> {
    let (status, bar) = match &dest.result {
        None => progress_row(dest.progress),
        Some(Ok(())) => (
            widget::text("Done").style(widget::text::success),
            widget::progress_bar(0.0..=1.0, 1.0),
        ),
        Some(Err(e)) => (
            widget::text(format!("Failed: {e}")).style(widget::text::danger),
            widget::progress_bar(0.0..=1.0, dest.progress.fraction())
                .style(widget::progress_bar::danger),
        ),
    };

    widget::column![
        widget::text(dest.label.as_ref()).font(constants::FONT_BOLD),
        status.style(widget::text::secondary),
        bar
    ]
    .spacing(4)
    .into()
}

fn progress_row<'a>(progress: Progress) -> (widget::Text<'a>, widget::ProgressBar<'a>) {
    match progress {
        Progress::Preparing => (
            widget::text("Preparing..."),
            widget::progress_bar(0.0..=1.0, 0.0),
        ),
        Progress::Flashing(f) | Progress::Downloading(f) => (
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
    }
}

/// Estimate the remaining flashing time from the current `progress` and how
/// much time has `elapsed` since the first progress update.
///
/// Split out of [`progress_view`] so the ETA math is testable without an
/// `Instant` clock: a linear extrapolation `elapsed * (1 - x) / x`, suppressed
/// until progress clears a small threshold to avoid wild early estimates.
fn time_remaining_from(progress: Progress, elapsed: Option<Duration>) -> Option<Duration> {
    const THRESHOLD: f32 = 0.02;

    match progress {
        Progress::Flashing(x) | Progress::Downloading(x) => {
            if x < THRESHOLD {
                None
            } else {
                let t = elapsed?;
                let x = x.clamp(0.0, 1.0);
                let scale = (1.0 - x) / x;
                Some(t.mul_f32(scale))
            }
        }
        Progress::Customizing => Some(Duration::from_secs(1)),
        _ => None,
    }
}

fn pretty_duration(d: Duration) -> String {
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

    #[test]
    fn eta_scales_linearly_with_remaining_fraction() {
        // At 50% after 10s, the remaining half should take another ~10s.
        assert_eq!(
            time_remaining_from(Progress::Flashing(0.5), Some(Duration::from_secs(10))),
            Some(Duration::from_secs(10))
        );
        // At 25% after 10s, the remaining 75% extrapolates to 30s.
        assert_eq!(
            time_remaining_from(Progress::Flashing(0.25), Some(Duration::from_secs(10))),
            Some(Duration::from_secs(30))
        );
    }

    #[test]
    fn eta_uses_the_same_math_for_downloads() {
        assert_eq!(
            time_remaining_from(Progress::Downloading(0.5), Some(Duration::from_secs(4))),
            Some(Duration::from_secs(4))
        );
    }

    #[test]
    fn eta_suppressed_below_threshold() {
        // Below 2% the estimate is too noisy, so no ETA is reported.
        assert_eq!(
            time_remaining_from(Progress::Flashing(0.01), Some(Duration::from_secs(10))),
            None
        );
    }

    #[test]
    fn eta_requires_a_start_timestamp() {
        // Past the threshold but with no elapsed time recorded yet.
        assert_eq!(time_remaining_from(Progress::Flashing(0.5), None), None);
    }

    #[test]
    fn eta_clamps_progress_above_one() {
        // A progress value >1.0 clamps to 1.0, yielding a zero remainder.
        assert_eq!(
            time_remaining_from(Progress::Flashing(1.5), Some(Duration::from_secs(10))),
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
    fn non_progress_states_have_no_eta() {
        assert_eq!(
            time_remaining_from(Progress::Preparing, Some(Duration::from_secs(5))),
            None
        );
        assert_eq!(
            time_remaining_from(Progress::Verifying, Some(Duration::from_secs(5))),
            None
        );
    }

    fn dest(progress: Progress, result: Option<Result<(), Box<str>>>) -> DestProgress {
        DestProgress {
            label: "dest".into(),
            progress,
            result,
        }
    }

    #[test]
    fn slowest_ignores_finished_destinations() {
        let state = State {
            dests: Box::new([
                dest(Progress::Flashing(0.1), Some(Err("gone".into()))),
                dest(Progress::Flashing(0.6), None),
                dest(Progress::Customizing, None),
                dest(Progress::Flashing(0.2), Some(Ok(()))),
            ]),
            start_timestamp: None,
        };

        assert!(matches!(state.slowest(), Some(Progress::Flashing(x)) if x == 0.6));
    }

    #[test]
    fn slowest_is_none_once_all_finished() {
        let state = State {
            dests: Box::new([dest(Progress::Flashing(1.0), Some(Ok(())))]),
            start_timestamp: None,
        };

        assert!(state.slowest().is_none());
    }

    #[test]
    fn pretty_duration_formats_minutes_and_seconds() {
        assert_eq!(pretty_duration(Duration::from_secs(0)), "0s");
        assert_eq!(pretty_duration(Duration::from_secs(45)), "45s");
        assert_eq!(pretty_duration(Duration::from_secs(60)), "1:00");
        assert_eq!(pretty_duration(Duration::from_secs(125)), "2:05");
    }
}
