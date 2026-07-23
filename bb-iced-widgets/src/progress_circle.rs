use iced::widget::{self, canvas};

#[derive(Debug)]
pub struct ProgressCircle {
    progress: f32,
    thickness: f32,
    color: iced::Color,
    cache: canvas::Cache,
    font: iced::Font,
}

impl ProgressCircle {
    pub fn new<T>(
        progress: f32,
        thickness: impl Into<f32>,
        color: iced::Color,
        font: iced::Font,
    ) -> widget::Canvas<Self, T> {
        widget::canvas(Self {
            progress,
            cache: canvas::Cache::new(),
            thickness: thickness.into(),
            color,
            font,
        })
        .width(iced::Fill)
        .height(iced::Fill)
    }
}

// Then, we implement the `Program` trait
impl<Message> canvas::Program<Message> for ProgressCircle {
    // No internal state
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &iced::Renderer,
        theme: &iced::Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            let center = iced::Point::new(bounds.width / 2.0, bounds.height / 2.0);
            let radius = bounds.width.min(bounds.height) / 2.0 - self.thickness;

            // Background ring
            let bg = canvas::Path::circle(center, radius);
            frame.stroke(
                &bg,
                canvas::Stroke::default()
                    .with_width(self.thickness)
                    .with_color(theme.palette().background),
            );

            // Foreground arc
            let angle = self.progress.clamp(0.0, 1.0) * 2.0 * iced::Radians::PI;

            let arc = canvas::path::Arc {
                center,
                radius,
                start_angle: iced::Radians::PI / 2.0,
                end_angle: iced::Radians::PI / 2.0 + angle,
            };
            let arc = canvas::Path::new(|b| b.arc(arc));

            frame.stroke(
                &arc,
                canvas::Stroke::default()
                    .with_line_cap(canvas::LineCap::Round)
                    .with_width(self.thickness)
                    .with_color(self.color),
            );

            // Progress Report
            let prog = (self.progress.clamp(0.0, 1.0) * 100.0).floor();
            let prog_pretty = format!("{}%", prog);
            frame.fill_text(canvas::Text {
                content: prog_pretty,
                position: center,
                align_x: iced::Center.into(),
                align_y: iced::Center.into(),
                size: (radius / 2.0).into(),
                color: theme.palette().text,
                font: self.font,
                ..Default::default()
            });
        });

        vec![geometry]
    }
}
