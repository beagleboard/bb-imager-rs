//! Widgets that participate in Tab focus order (iced 0.14 [`widget::button`]
//! and [`widget::toggler`] do not).

use iced::advanced::Renderer as _;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::operation::Focusable as FocusableOp;
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::keyboard;
use iced::mouse;
use iced::widget::button as iced_button;
use iced::{Border, Color, Element, Event, Length, Rectangle, Size, Theme};

use crate::message::BBImagerMessage;

#[derive(Debug, Default)]
struct State {
    focused: bool,
}

impl FocusableOp for State {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self) {
        self.focused = true;
    }

    fn unfocus(&mut self) {
        self.focused = false;
    }
}

/// Builder for a focusable [`iced_button`].
pub(crate) struct FocusableButtonBuilder<'a> {
    content: Element<'a, BBImagerMessage>,
    on_press: Option<BBImagerMessage>,
    width: Length,
    height: Length,
    style: Option<iced_button::StyleFn<'a, Theme>>,
}

impl<'a> FocusableButtonBuilder<'a> {
    pub(crate) fn new(content: impl Into<Element<'a, BBImagerMessage>>) -> Self {
        Self {
            content: content.into(),
            on_press: None,
            width: Length::Shrink,
            height: Length::Shrink,
            style: None,
        }
    }

    pub(crate) fn on_press(mut self, message: BBImagerMessage) -> Self {
        self.on_press = Some(message);
        self
    }

    pub(crate) fn on_press_maybe(mut self, message: Option<BBImagerMessage>) -> Self {
        self.on_press = message;
        self
    }

    pub(crate) fn style(
        mut self,
        style: impl Fn(&Theme, iced_button::Status) -> iced_button::Style + 'a,
    ) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    pub(crate) fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub(crate) fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    fn into_focusable(self) -> Focusable<'a> {
        let mut button = iced_button(self.content)
            .width(self.width)
            .height(self.height);

        if let Some(style) = self.style {
            button = button.style(style);
        }

        button = button.on_press_maybe(self.on_press.clone());

        Focusable::new(button.into(), self.on_press)
    }
}

/// Wraps any widget so it joins iced's Tab cycle and activates on Enter or Space.
pub(crate) struct Focusable<'a> {
    inner: Element<'a, BBImagerMessage>,
    on_press: Option<BBImagerMessage>,
}

impl<'a> Focusable<'a> {
    fn new(inner: Element<'a, BBImagerMessage>, on_press: Option<BBImagerMessage>) -> Self {
        Self { inner, on_press }
    }
}

impl<'a> Widget<BBImagerMessage, Theme, iced::Renderer> for Focusable<'a> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.inner)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.inner));
    }

    fn size(&self) -> Size<Length> {
        self.inner.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.inner
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let state = tree.state.downcast_mut::<State>();
        if self.on_press.is_some() {
            // `None` keeps us in Tab order by tree position. A fresh
            // `Id::unique()` every `view()` would be wasted work and would
            // break `focus(id)` if anything ever targeted these widgets.
            operation.focusable(None, layout.bounds(), state);
        } else {
            state.unfocus();
        }

        self.inner
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, BBImagerMessage>,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let over = cursor.is_over(layout.bounds());

        if let Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event
            && state.focused
            && matches!(
                key,
                keyboard::Key::Named(keyboard::key::Named::Enter)
                    | keyboard::Key::Named(keyboard::key::Named::Space)
            )
            && let Some(on_press) = &self.on_press
        {
            shell.publish(on_press.clone());
            shell.capture_event();
            return;
        }

        // Same exclusive-focus pattern as iced's `text_input`: every focusable
        // sees the press, so the one under the cursor takes focus and the rest
        // drop it. Column/row still visit siblings after a capture.
        if matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                | Event::Touch(iced::touch::Event::FingerPressed { .. })
        ) {
            if over && self.on_press.is_some() {
                state.focus();
            } else {
                state.unfocus();
            }
        }

        self.inner.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.inner.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            defaults,
            layout,
            cursor,
            viewport,
        );

        let state = tree.state.downcast_ref::<State>();
        if state.focused {
            draw_focus_ring(renderer, layout.bounds(), theme);
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.inner.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }
}

fn draw_focus_ring(renderer: &mut iced::Renderer, bounds: Rectangle, theme: &Theme) {
    let high_contrast = crate::constants::is_high_contrast_palette(&theme.palette());
    let width = crate::constants::focus_ring_width(high_contrast);
    let radius = 5.0 + crate::constants::FOCUS_RING_OUTSET;
    let (inner_color, outer_color) = crate::constants::focus_ring_colors(high_contrast);

    let mut stroke = |expand: f32, color: Color| {
        renderer.fill_quad(
            renderer::Quad {
                bounds: bounds.expand(expand),
                border: Border {
                    width,
                    radius: (radius + expand - crate::constants::FOCUS_RING_OUTSET).into(),
                    color,
                },
                ..Default::default()
            },
            Color::TRANSPARENT,
        );
    };

    if high_contrast {
        // White outer + black inner so the ring stays visible on both yellow
        // primary buttons and dark secondary fills.
        stroke(crate::constants::FOCUS_RING_OUTSET + width, outer_color);
        stroke(crate::constants::FOCUS_RING_OUTSET, inner_color);
    } else {
        stroke(crate::constants::FOCUS_RING_OUTSET, inner_color);
    }
}

impl<'a> From<FocusableButtonBuilder<'a>> for Focusable<'a> {
    fn from(builder: FocusableButtonBuilder<'a>) -> Self {
        builder.into_focusable()
    }
}

impl<'a> From<FocusableButtonBuilder<'a>> for Element<'a, BBImagerMessage> {
    fn from(builder: FocusableButtonBuilder<'a>) -> Self {
        Focusable::from(builder).into()
    }
}

impl<'a> From<Focusable<'a>> for Element<'a, BBImagerMessage> {
    fn from(focusable: Focusable<'a>) -> Self {
        Element::new(focusable)
    }
}

pub(crate) fn button<'a>(
    content: impl Into<Element<'a, BBImagerMessage>>,
) -> FocusableButtonBuilder<'a> {
    FocusableButtonBuilder::new(content)
}

/// Wrap `content` so Tab can reach it and Enter/Space publishes `on_activate`.
pub(crate) fn activate<'a>(
    content: impl Into<Element<'a, BBImagerMessage>>,
    on_activate: BBImagerMessage,
) -> Element<'a, BBImagerMessage> {
    Focusable::new(content.into(), Some(on_activate)).into()
}
