use iced::{
    Border, Color, Element, Length, Rectangle, Shadow, Size,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, renderer,
        widget::{self, Operation, Tree},
    },
    mouse,
};
use std::marker::PhantomData;
use std::time::{Duration, Instant};

pub struct Linear<Message> {
    width: Length,
    height: f32,
    cycle_duration: Duration,
    bar_width_ratio: f32,
    color: Color,
    _phantom: PhantomData<Message>,
}

impl<Message> Linear<Message> {
    pub fn new() -> Self {
        Self {
            width: Length::Fill,
            height: 8.0, // Match ProgressBar
            cycle_duration: Duration::from_millis(1000),
            bar_width_ratio: 0.3,
            color: Color::from_rgb(0.0, 0.5, 1.0),
            _phantom: PhantomData,
        }
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn cycle_duration(mut self, duration: Duration) -> Self {
        self.cycle_duration = duration;
        self
    }

    pub fn bar_width_ratio(mut self, ratio: f32) -> Self {
        self.bar_width_ratio = ratio.clamp(0.1, 0.8);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<Message> Default for Linear<Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Linear<Message>
where
    Renderer: renderer::Renderer,
    Theme: 'static,
{
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State::new())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: Length::Fixed(self.height),
        }
    }

    fn layout(
        &self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(self.width).height(Length::Fixed(self.height));
        let size = limits.resolve(self.width, Length::Fixed(self.height), Size::ZERO);
        layout::Node::new(size)
    }

    fn operate(
        &self,
        tree: &mut Tree,
        _layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let state = tree.state.downcast_mut::<State>();
        operation.custom(state, None);
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let progress = state.progress(self.cycle_duration);

        let bar_width = bounds.width * self.bar_width_ratio;
        let max_offset = bounds.width - bar_width;
        let offset = max_offset * progress; // Linear easing for simplicity

        let bar = renderer::Quad {
            bounds: Rectangle {
                x: bounds.x + offset,
                y: bounds.y,
                width: bar_width,
                height: bounds.height,
            },
            border: Border {
                radius: 2.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            shadow: Shadow::default(),
        };

        renderer.fill_quad(bar, self.color);
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: iced::Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> iced::event::Status {
        let state = tree.state.downcast_mut::<State>();

        if let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            state.last_redraw = Some(now);
            if state.start.is_none() {
                state.start = Some(now);
            }
            shell.request_redraw(iced::window::RedrawRequest::NextFrame);
            iced::event::Status::Captured
        } else {
            iced::event::Status::Ignored
        }
    }
}

#[derive(Debug)]
struct State {
    start: Option<Instant>,
    last_redraw: Option<Instant>,
}

impl State {
    fn new() -> Self {
        Self {
            start: None,
            last_redraw: None,
        }
    }

    fn progress(&self, cycle_duration: Duration) -> f32 {
        match (self.start, self.last_redraw) {
            (Some(start), Some(last_redraw)) => {
                let elapsed = last_redraw.duration_since(start).as_secs_f32();
                let duration = cycle_duration.as_secs_f32();
                (elapsed % duration) / duration
            }
            _ => 0.0,
        }
    }
}

impl<Message, Theme, Renderer> From<Linear<Message>> for Element<'_, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Theme: 'static,
    Renderer: renderer::Renderer + 'static,
{
    fn from(widget: Linear<Message>) -> Self {
        Element::new(widget)
    }
}
