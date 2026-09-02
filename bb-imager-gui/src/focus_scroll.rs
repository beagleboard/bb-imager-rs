//! Scroll ancestor [`iced::widget::scrollable`] panes so a target widget
//! stays visible.

use iced::advanced::widget::{self, Operation};
use iced::{Rectangle, Task, Vector};
use widget::operation::{Focusable, Outcome, Scrollable, scrollable::AbsoluteOffset};

use crate::message::BBImagerMessage;

enum Phase {
    Find,
    Apply,
}

enum Target {
    Focused,
    Id(widget::Id),
}

struct ScrollIntoView {
    target: Target,
    phase: Phase,
    found_bounds: Option<Rectangle>,
}

impl ScrollIntoView {
    const fn focused() -> Self {
        Self {
            target: Target::Focused,
            phase: Phase::Find,
            found_bounds: None,
        }
    }

    fn widget(id: widget::Id) -> Self {
        Self {
            target: Target::Id(id),
            phase: Phase::Find,
            found_bounds: None,
        }
    }
}

fn scroll_padding() -> f32 {
    crate::constants::FOCUS_RING_OUTSET + 8.0
}

/// Visible slice of `viewport` in the same coordinate space as content widgets.
///
/// iced reports child layout bounds in unscrolled content space and passes the
/// current scroll offset as `translation`. Adding it maps the clip rect into
/// that space.
fn visible_rect(viewport: Rectangle, translation: Vector) -> Rectangle {
    Rectangle {
        x: viewport.x + translation.x,
        y: viewport.y + translation.y,
        width: viewport.width,
        height: viewport.height,
    }
}

/// How far to scroll each axis so `focus` is visible inside `visible`.
fn scroll_delta(visible: Rectangle, focus: Rectangle, pad: f32) -> (f32, f32) {
    let mut delta_x = 0.0;
    let mut delta_y = 0.0;

    let focus_top = focus.y;
    let focus_bottom = focus.y + focus.height;
    let focus_left = focus.x;
    let focus_right = focus.x + focus.width;

    let view_top = visible.y + pad;
    let view_bottom = visible.y + visible.height - pad;
    let view_left = visible.x + pad;
    let view_right = visible.x + visible.width - pad;

    if focus_top < view_top {
        delta_y = focus_top - view_top;
    } else if focus_bottom > view_bottom {
        delta_y = focus_bottom - view_bottom;
    }

    if focus_left < view_left {
        delta_x = focus_left - view_left;
    } else if focus_right > view_right {
        delta_x = focus_right - view_right;
    }

    (delta_x, delta_y)
}

fn scroll_rect_into_view(
    state: &mut dyn Scrollable,
    viewport: Rectangle,
    content_bounds: Rectangle,
    translation: Vector,
    focus: Rectangle,
) {
    // Board/OS/destination screens have two scrollables. Only move the pane
    // that actually contains the target; otherwise the other pane jumps.
    if !content_bounds.intersects(&focus) {
        return;
    }

    let visible = visible_rect(viewport, translation);
    let (delta_x, delta_y) = scroll_delta(visible, focus, scroll_padding());

    if delta_x.abs() > f32::EPSILON || delta_y.abs() > f32::EPSILON {
        state.scroll_by(
            AbsoluteOffset {
                x: delta_x,
                y: delta_y,
            },
            viewport,
            content_bounds,
        );
    }
}

impl ScrollIntoView {
    fn consider(&mut self, id: Option<&widget::Id>, bounds: Rectangle, focused: bool) {
        if !matches!(self.phase, Phase::Find) {
            return;
        }

        match &self.target {
            Target::Focused if focused => self.found_bounds = Some(bounds),
            Target::Id(want) if id == Some(want) => self.found_bounds = Some(bounds),
            _ => {}
        }
    }
}

impl Operation<()> for ScrollIntoView {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<()>)) {
        operate(self);
    }

    fn container(&mut self, id: Option<&widget::Id>, bounds: Rectangle) {
        self.consider(id, bounds, false);
    }

    fn focusable(&mut self, id: Option<&widget::Id>, bounds: Rectangle, state: &mut dyn Focusable) {
        self.consider(id, bounds, state.is_focused());
    }

    fn scrollable(
        &mut self,
        _id: Option<&widget::Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        state: &mut dyn Scrollable,
    ) {
        if matches!(self.phase, Phase::Apply)
            && let Some(focus) = self.found_bounds
        {
            scroll_rect_into_view(state, bounds, content_bounds, translation, focus);
        }
    }

    fn finish(&self) -> Outcome<()> {
        if matches!(self.phase, Phase::Find) && self.found_bounds.is_some() {
            return Outcome::Chain(Box::new(Self {
                phase: Phase::Apply,
                found_bounds: self.found_bounds,
                target: match &self.target {
                    Target::Focused => Target::Focused,
                    Target::Id(id) => Target::Id(id.clone()),
                },
            }));
        }

        Outcome::None
    }
}

pub(crate) fn scroll_focused_into_view() -> Task<BBImagerMessage> {
    iced::advanced::widget::operate(ScrollIntoView::focused()).map(|()| BBImagerMessage::Null)
}

pub(crate) fn scroll_widget_into_view(id: widget::Id) -> Task<BBImagerMessage> {
    iced::advanced::widget::operate(ScrollIntoView::widget(id)).map(|()| BBImagerMessage::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrolls_up_when_focus_is_above_viewport() {
        let visible = Rectangle {
            x: 0.0,
            y: 100.0,
            width: 200.0,
            height: 300.0,
        };
        let focus = Rectangle {
            x: 10.0,
            y: 50.0,
            width: 80.0,
            height: 24.0,
        };

        let (_, delta_y) = scroll_delta(visible, focus, 8.0);
        assert!(delta_y < 0.0);
    }

    #[test]
    fn scrolls_down_when_focus_is_below_viewport() {
        let visible = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 100.0,
        };
        let focus = Rectangle {
            x: 10.0,
            y: 120.0,
            width: 80.0,
            height: 24.0,
        };

        let (_, delta_y) = scroll_delta(visible, focus, 8.0);
        assert!(delta_y > 0.0);
    }

    #[test]
    fn no_scroll_when_focus_is_fully_visible() {
        let visible = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 200.0,
        };
        let focus = Rectangle {
            x: 20.0,
            y: 50.0,
            width: 80.0,
            height: 24.0,
        };

        let (delta_x, delta_y) = scroll_delta(visible, focus, 8.0);
        assert_eq!(delta_x, 0.0);
        assert_eq!(delta_y, 0.0);
    }

    #[test]
    fn translation_maps_viewport_into_content_space() {
        let viewport = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 100.0,
        };
        let visible = visible_rect(viewport, Vector { x: 0.0, y: 50.0 });
        let focus = Rectangle {
            x: 10.0,
            y: 0.0,
            width: 80.0,
            height: 24.0,
        };

        let (_, delta_y) = scroll_delta(visible, focus, 8.0);
        assert!(delta_y < 0.0, "scrolled-away top widget should scroll up");
    }

    #[test]
    fn sibling_pane_content_does_not_intersect_focus() {
        let left_content = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 800.0,
        };
        let focus_in_right = Rectangle {
            x: 250.0,
            y: 40.0,
            width: 80.0,
            height: 24.0,
        };

        assert!(!left_content.intersects(&focus_in_right));
    }
}
