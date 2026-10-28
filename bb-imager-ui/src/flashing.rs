use std::cmp::Ordering;
use std::time::{Duration, Instant};

use iced::{Element, widget};

use crate::helpers::{SidebarEntry, VIEW_COL_PADDING, page_type2};
use crate::{Message, constants};

/// How far along the flash is.
///
/// Mirrors the flasher's own status enum; the host converts.
///
/// Ordered by how far along it is: variants in declaration order, then the
/// fraction written.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub enum Progress {
    #[default]
    Preparing,
    Writing(f32),
    Verifying,
    Customizing,
}

#[derive(Debug)]
pub struct State {
    /// Each destination's label and how far along its flash is.
    pub destinations: Box<[(Box<str>, Progress)]>,
    /// Stamped by the host on the first byte-moving update; the ETA is
    /// extrapolated from how long has elapsed since.
    pub start_timestamp: Option<Instant>,
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
    let time_remaining = match time_remaining(
        &state.destinations,
        state.start_timestamp.map(|t| t.elapsed()),
    ) {
        Some(x) => widget::span::<'_, (), _>(pretty_duration(x)),
        None => widget::span("Calculating"),
    };

    widget::scrollable(
        widget::column![
            widget::text("Writing Image")
                .font(constants::FONT_BOLD)
                .size(26),
            widget::text("Do not disconnect the storage device!").style(widget::text::danger),
            widget::rule::horizontal(2),
        ]
        .extend(
            state
                .destinations
                .iter()
                .map(|(dest, progress)| progress_row(dest, *progress)),
        )
        .push(widget::rich_text![
            widget::span("Time Remaining: ").font(constants::FONT_BOLD),
            time_remaining
        ])
        .padding(VIEW_COL_PADDING)
        .spacing(16),
    )
    .id(scroll_id)
    .into()
}

/// One destination's label, current phase and progress bar.
fn progress_row(dest: &str, progress: Progress) -> Element<'_, Message> {
    let (progress_label, progress_bar) = match progress {
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

    widget::column![
        widget::text(dest).font(constants::FONT_BOLD),
        progress_label.style(widget::text::secondary),
        progress_bar,
    ]
    .spacing(8)
    .into()
}

/// Estimate the remaining time for the whole run. Every destination shares
/// `elapsed`, so the least-progressed one is also the slowest.
fn time_remaining(
    destinations: &[(Box<str>, Progress)],
    elapsed: Option<Duration>,
) -> Option<Duration> {
    destinations
        .iter()
        .map(|(_, progress)| *progress)
        // `Progress` holds an `f32`, so no `Ord` for `min`; only NaN is unordered.
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
        .and_then(|least| time_remaining_from(least, elapsed))
}

/// Estimate one destination's remaining flashing time from its `progress` and
/// how much time has `elapsed` since the first progress update.
///
/// Split out of [`progress_view`] so the ETA math is testable without an
/// `Instant` clock: a linear extrapolation `elapsed * (1 - x) / x`, suppressed
/// until progress clears a small threshold to avoid wild early estimates.
fn time_remaining_from(progress: Progress, elapsed: Option<Duration>) -> Option<Duration> {
    const THRESHOLD: f32 = 0.02;

    match progress {
        Progress::Writing(x) => {
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
            time_remaining_from(Progress::Writing(0.5), Some(Duration::from_secs(10))),
            Some(Duration::from_secs(10))
        );
        // At 25% after 10s, the remaining 75% extrapolates to 30s.
        assert_eq!(
            time_remaining_from(Progress::Writing(0.25), Some(Duration::from_secs(10))),
            Some(Duration::from_secs(30))
        );
    }

    #[test]
    fn eta_suppressed_below_threshold() {
        // Below 2% the estimate is too noisy, so no ETA is reported.
        assert_eq!(
            time_remaining_from(Progress::Writing(0.01), Some(Duration::from_secs(10))),
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
            time_remaining_from(Progress::Writing(1.5), Some(Duration::from_secs(10))),
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
    fn eta_follows_slowest_destination() {
        let dests = [
            ("A".into(), Progress::Writing(0.5)),
            ("B".into(), Progress::Writing(0.25)),
        ];
        assert_eq!(
            time_remaining(&dests, Some(Duration::from_secs(10))),
            Some(Duration::from_secs(30))
        );
    }

    #[test]
    fn eta_waits_for_every_destination() {
        // One destination still preparing means the run has no estimate yet.
        let dests = [
            ("A".into(), Progress::Writing(0.5)),
            ("B".into(), Progress::Preparing),
        ];
        assert_eq!(time_remaining(&dests, Some(Duration::from_secs(10))), None);
    }

    #[test]
    fn eta_ignores_destinations_further_along() {
        // Verifying has no estimate of its own; the one still writing decides.
        let dests = [
            ("A".into(), Progress::Writing(0.5)),
            ("B".into(), Progress::Verifying),
        ];
        assert_eq!(
            time_remaining(&dests, Some(Duration::from_secs(10))),
            Some(Duration::from_secs(10))
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

    #[test]
    fn pretty_duration_formats_minutes_and_seconds() {
        assert_eq!(pretty_duration(Duration::from_secs(0)), "0s");
        assert_eq!(pretty_duration(Duration::from_secs(45)), "45s");
        assert_eq!(pretty_duration(Duration::from_secs(60)), "1:00");
        assert_eq!(pretty_duration(Duration::from_secs(125)), "2:05");
    }
}
