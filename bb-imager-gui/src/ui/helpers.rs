use iced::{
    Element, Radians, Rectangle, Renderer, Theme,
    advanced::text::highlighter::PlainText,
    mouse,
    widget::{self, canvas},
};

use crate::{constants, message::BBImagerMessage};

pub(crate) fn card_btn_style(
    theme: &iced::Theme,
    status: widget::button::Status,
    is_selected: bool,
) -> widget::button::Style {
    let mut style = widget::button::Style {
        text_color: theme.palette().text,
        ..Default::default()
    };

    if is_selected || matches!(status, widget::button::Status::Hovered) {
        style.border = iced::Border::default()
            .color(theme.palette().primary)
            .width(3)
            .rounded(5);
    }

    style
}

pub(crate) fn svg_icon_style(theme: &iced::Theme, _: widget::svg::Status) -> widget::svg::Style {
    widget::svg::Style {
        color: Some(theme.palette().text),
    }
}

/// |------|------|
/// |      |      |
/// |      | col2 |
/// | col1 |      |
/// |      |------|
/// |      | btns |
/// |------|------|
pub(crate) fn page_type1<'a>(
    common: &crate::BBImagerCommon,
    col1: Element<'a, BBImagerMessage>,
    col2: Element<'a, BBImagerMessage>,
    btns: impl IntoIterator<Item = widget::Button<'a, BBImagerMessage>>,
) -> Element<'a, BBImagerMessage> {
    let row2 = widget::row(
        [
            info_btn(common.info_svg().clone()).into(),
            widget::space::horizontal().into(),
        ]
        .into_iter()
        .chain(btns.into_iter().map(Into::into)),
    )
    .align_y(iced::Center)
    .width(iced::Length::Fill)
    .spacing(24);

    let col2 = widget::column![
        card_box(col2)
            .height(iced::Length::Fill)
            .width(iced::Length::Fill),
        row2.width(iced::Length::Fill)
    ]
    .spacing(24)
    .width(iced::FillPortion(1));

    widget::row![
        card_box(col1)
            .height(iced::Length::Fill)
            .width(iced::Length::FillPortion(1)),
        col2
    ]
    .padding(24)
    .spacing(24)
    .into()
}

/// |--------|
/// |        |
/// |  row1  |
/// |        |
/// |--------|
/// |  btns  |
/// |--------|
pub(crate) fn page_type2<'a>(
    common: &crate::BBImagerCommon,
    row1: Element<'a, BBImagerMessage>,
    btns: impl IntoIterator<Item = widget::Button<'a, BBImagerMessage>>,
) -> Element<'a, BBImagerMessage> {
    let row2 = widget::row(
        [
            info_btn(common.info_svg().clone()).into(),
            widget::space::horizontal().into(),
        ]
        .into_iter()
        .chain(btns.into_iter().map(Into::into)),
    )
    .align_y(iced::Center)
    .width(iced::Length::Fill)
    .spacing(24);

    widget::column![card_box(row1).height(iced::Fill).width(iced::Fill), row2]
        .padding(24)
        .spacing(24)
        .into()
}

/// |--------|
/// |        |
/// |  row1  |
/// |        |
/// |--------|
/// |  btns  |
/// |--------|
pub(crate) fn page_type3<'a>(
    row1: Element<'a, BBImagerMessage>,
    btns: impl IntoIterator<Item = widget::Button<'a, BBImagerMessage>>,
) -> Element<'a, BBImagerMessage> {
    let row2 = widget::row(
        [widget::space::horizontal().into()]
            .into_iter()
            .chain(btns.into_iter().map(Into::into)),
    )
    .align_y(iced::Center)
    .width(iced::Length::Fill)
    .spacing(24);

    widget::column![card_box(row1).height(iced::Fill).width(iced::Fill), row2]
        .padding(24)
        .spacing(24)
        .into()
}

#[derive(Debug)]
pub(crate) struct ProgressCircle {
    progress: f32,
    thickness: f32,
    color: iced::Color,
    cache: canvas::Cache,
}

impl ProgressCircle {
    pub(crate) fn new(
        progress: f32,
        thickness: impl Into<f32>,
        color: iced::Color,
    ) -> widget::Canvas<Self, BBImagerMessage> {
        widget::canvas(Self {
            progress,
            cache: canvas::Cache::new(),
            thickness: thickness.into(),
            color,
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
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
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
            let angle = self.progress.clamp(0.0, 1.0) * 2.0 * Radians::PI;

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
                font: constants::FONT_BOLD,
                ..Default::default()
            });
        });

        vec![geometry]
    }
}

pub(crate) fn board_view_pane<'a>(
    dev: &'a bb_config::config::Device,
    state: &'a crate::BBImagerCommon,
) -> Element<'a, BBImagerMessage> {
    let img: Element<BBImagerMessage> = match &dev.icon {
        Some(u) => match state.image_handle_cache().get(u) {
            Some(x) => x.view(iced::Length::Fill, iced::Shrink),
            None => widget::svg(state.downloading_svg().clone())
                .width(iced::Length::Fill)
                .style(svg_icon_style)
                .into(),
        },
        None => widget::svg(state.board_svg().clone())
            .width(iced::Length::Fill)
            .style(svg_icon_style)
            .into(),
    };

    let cols = widget::column![
        img,
        widget::text(&dev.name)
            .size(24)
            .align_x(iced::alignment::Alignment::Center)
            .width(iced::Length::Fill),
        widget::text(&dev.description)
            .align_x(iced::alignment::Alignment::Center)
            .width(iced::Length::Fill),
    ]
    .spacing(16);

    let cols = cols.extend(
        dev.specification
            .iter()
            .map(|(k, v)| -> widget::text::Rich<'a, (), BBImagerMessage> { detail_entry(k, v) })
            .map(Into::into),
    );

    let cols = match dev.documentation.clone() {
        Some(x) => cols.push(widget::center_x(
            widget::button("DOCUMENTATION").on_press(BBImagerMessage::OpenUrl(x)),
        )),
        None => cols,
    };

    widget::scrollable(cols).into()
}

#[derive(Debug)]
pub(crate) struct CircleBar {
    label: &'static str,
    thickness: f32,
    color: iced::Color,
    cache: canvas::Cache,
}

impl CircleBar {
    pub(crate) fn new(
        label: &'static str,
        thickness: impl Into<f32>,
        color: iced::Color,
    ) -> widget::Canvas<Self, BBImagerMessage> {
        widget::canvas(Self {
            label,
            cache: canvas::Cache::new(),
            thickness: thickness.into(),
            color,
        })
        .width(iced::Fill)
        .height(iced::Fill)
    }
}

impl<Message> canvas::Program<Message> for CircleBar {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
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
                    .with_color(self.color),
            );

            frame.fill_text(canvas::Text {
                content: self.label.to_string(),
                position: center,
                align_x: iced::Center.into(),
                align_y: iced::Center.into(),
                size: (radius / 3.0).into(),
                color: theme.palette().text,
                font: constants::FONT_BOLD,
                ..Default::default()
            });
        });

        vec![geometry]
    }
}

pub(crate) fn detail_entry<'a>(
    key: &'a str,
    val: impl widget::text::IntoFragment<'a>,
) -> widget::text::Rich<'a, (), BBImagerMessage> {
    widget::rich_text![
        widget::span(format!("{key}:")).font(constants::FONT_BOLD),
        widget::span(" "),
        widget::span(val),
    ]
}

pub(crate) fn element_with_label<'a>(
    label: &'static str,
    el: Element<'a, BBImagerMessage>,
) -> widget::Row<'a, BBImagerMessage> {
    element_with_element(label.into(), el).padding(iced::Padding::ZERO.horizontal(16))
}

pub(crate) fn element_with_element<'a>(
    el1: Element<'a, BBImagerMessage>,
    el2: Element<'a, BBImagerMessage>,
) -> widget::Row<'a, BBImagerMessage> {
    widget::row![el1, widget::space::horizontal(), el2]
        .align_y(iced::Alignment::Center)
        .padding(iced::Padding::ZERO.right(16))
}

pub(crate) fn selectable_text(
    content: &widget::text_editor::Content,
) -> widget::text_editor::TextEditor<'_, PlainText, BBImagerMessage> {
    widget::text_editor(content).on_action(BBImagerMessage::EditorEvent)
}

fn card_box<'a>(
    content: impl Into<Element<'a, BBImagerMessage>>,
) -> widget::Container<'a, BBImagerMessage> {
    widget::container(content)
        .style(|_| {
            widget::container::Style::default()
                .background(constants::CARD)
                .border(iced::border::rounded(8))
        })
        .padding(16)
}

fn info_btn(handle: widget::svg::Handle) -> widget::Button<'static, BBImagerMessage> {
    widget::button(widget::svg(handle))
        .on_press(BBImagerMessage::AppInfo)
        .width(iced::Shrink)
        .height(iced::Shrink)
}
