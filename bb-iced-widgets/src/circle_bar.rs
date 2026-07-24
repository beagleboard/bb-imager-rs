use iced::widget;

#[derive(Debug)]
pub struct CircleBar {
    label: &'static str,
    thickness: f32,
    color: iced::Color,
    cache: iced::widget::canvas::Cache,
    font: iced::Font,
}

impl CircleBar {
    pub(crate) fn new<T>(
        label: &'static str,
        thickness: impl Into<f32>,
        color: iced::Color,
        font: iced::Font,
    ) -> widget::Canvas<Self, T> {
        widget::canvas(Self {
            label,
            cache: widget::canvas::Cache::new(),
            thickness: thickness.into(),
            color,
            font,
        })
        .width(iced::Fill)
        .height(iced::Fill)
    }
}

impl<Message> widget::canvas::Program<Message> for CircleBar {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &iced::Renderer,
        theme: &iced::Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<widget::canvas::Geometry> {
        let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            let center = iced::Point::new(bounds.width / 2.0, bounds.height / 2.0);
            let radius = bounds.width.min(bounds.height) / 2.0 - self.thickness;

            // Background ring
            let bg = widget::canvas::Path::circle(center, radius);
            frame.stroke(
                &bg,
                widget::canvas::Stroke::default()
                    .with_width(self.thickness)
                    .with_color(self.color),
            );

            let frac = if self.label.len() > 4 { 3.0 } else { 2.0 };

            frame.fill_text(widget::canvas::Text {
                content: self.label.to_string(),
                position: center,
                align_x: iced::Center.into(),
                align_y: iced::Center.into(),
                size: (radius / frac).into(),
                color: theme.palette().text,
                font: self.font,
                ..Default::default()
            });
        });

        vec![geometry]
    }
}
