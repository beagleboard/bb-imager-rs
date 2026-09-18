use std::sync::Arc;
use std::time::{Duration, Instant};

use bb_iced_widgets::progress_circle;
use iced::{Element, widget};

use crate::board_selection::BoardDetails;
use crate::helpers::{VIEW_COL_PADDING, board_details_pane, detail_entry, page_type1};
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

#[derive(Debug)]
pub struct State {
    pub board: BoardDetails,
    pub progress: Progress,
    /// Stamped by the host on the first byte-moving update; the ETA is
    /// extrapolated from how long has elapsed since.
    pub start_timestamp: Option<Instant>,
}

pub fn view<'a>(
    cache: &'a bb_iced_widgets::cached_icon::Cache<Arc<url::Url>>,
    state: &'a State,
    scroll_id: widget::Id,
) -> Element<'a, Message> {
    page_type1(
        board_details_pane(cache, &state.board, &scroll_id),
        progress_view(state),
        [widget::button("Cancel")
            .style(widget::button::danger)
            .on_press(Message::FlashCancel)],
    )
}

fn progress_view(state: &State) -> Element<'_, Message> {
    let (prog, label) = match state.progress {
        Progress::Preparing => (0.0, "Preparing ..."),
        Progress::Downloading(x) => (x, "Downloading ..."),
        Progress::Flashing(x) => (x, "Flashing Image ..."),
        Progress::Verifying => (0.99, "Verifying ..."),
        Progress::Customizing => (0.99, "Customizing ..."),
    };

    let progress = progress_circle(
        prog,
        10.0f32,
        constants::TONGUE_ORANGE,
        constants::FONT_BOLD,
    );

    let mut col = widget::column![progress, widget::text(label)];

    // Recomputed every frame against a live clock, so the estimate keeps
    // shrinking between progress updates.
    if let Some(x) = time_remaining_from(state.progress, state.start_timestamp.map(|t| t.elapsed()))
    {
        col = col.push(detail_entry("Time Remaining", pretty_duration(x)));
    }

    col.align_x(iced::Center).padding(VIEW_COL_PADDING).into()
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

    #[test]
    fn pretty_duration_formats_minutes_and_seconds() {
        assert_eq!(pretty_duration(Duration::from_secs(0)), "0s");
        assert_eq!(pretty_duration(Duration::from_secs(45)), "45s");
        assert_eq!(pretty_duration(Duration::from_secs(60)), "1:00");
        assert_eq!(pretty_duration(Duration::from_secs(125)), "2:05");
    }
}
