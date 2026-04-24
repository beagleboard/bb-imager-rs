use std::sync::LazyLock;

use iced::{
    Element, Radians, Rectangle, Renderer, Theme,
    advanced::text::highlighter::PlainText,
    mouse,
    widget::{self, canvas, svg},
};

use crate::{constants, message::BBImagerMessage};

pub(crate) static WINDOW_ICON: LazyLock<widget::image::Handle> =
    LazyLock::new(|| widget::image::Handle::from_bytes(constants::WINDOW_ICON_BYTES));

pub(crate) static ARROW_BACK_ICON: LazyLock<svg::Handle> =
    LazyLock::new(|| svg::Handle::from_memory(constants::ARROW_BACK_ICON_BYTES));
pub(crate) static FILE_ADD_ICON: LazyLock<svg::Handle> =
    LazyLock::new(|| svg::Handle::from_memory(constants::FILE_ADD_ICON_BYTES));
pub(crate) static USB_ICON: LazyLock<svg::Handle> =
    LazyLock::new(|| svg::Handle::from_memory(constants::USB_ICON_BYTES));
pub(crate) static FORMAT_ICON: LazyLock<svg::Handle> =
    LazyLock::new(|| svg::Handle::from_memory(constants::FORMAT_ICON_BYTES));
pub(crate) static BOARD_ICON: LazyLock<svg::Handle> =
    LazyLock::new(|| svg::Handle::from_memory(constants::BOARD_ICON_BYTES));
pub(crate) static ARROW_FORWARD_IOS_ICON: LazyLock<svg::Handle> =
    LazyLock::new(|| svg::Handle::from_memory(constants::ARROW_FORWARD_IOS_ICON_BYTES));
pub(crate) static FILE_SAVE_ICON: LazyLock<svg::Handle> =
    LazyLock::new(|| svg::Handle::from_memory(constants::FILE_SAVE_ICON_BYTES));
pub(crate) static INFO_ICON: LazyLock<svg::Handle> =
    LazyLock::new(|| svg::Handle::from_memory(constants::INFO_ICON_BYTES));
pub(crate) static COPY_ICON: LazyLock<svg::Handle> =
    LazyLock::new(|| svg::Handle::from_memory(constants::COPY_ICON_BYTES));
pub(crate) static SEARCH_ICON: LazyLock<svg::Handle> =
    LazyLock::new(|| svg::Handle::from_memory(constants::SEARCH_ICON_BYTES));

pub(crate) const VIEW_COL_PADDING: u16 = 16;
pub(crate) const LIST_COL_PADDING: iced::Padding = iced::Padding {
    right: 16.0,
    ..iced::Padding::ZERO
};

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

pub(crate) fn svg_icon_style(theme: &iced::Theme, _: svg::Status) -> svg::Style {
    svg::Style {
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
    col1: Element<'a, BBImagerMessage>,
    col2: Element<'a, BBImagerMessage>,
    btns: impl IntoIterator<Item = widget::Button<'a, BBImagerMessage>>,
) -> Element<'a, BBImagerMessage> {
    let row2 = widget::row(
        [
            info_btn(INFO_ICON.clone()).into(),
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
    row1: Element<'a, BBImagerMessage>,
    btns: impl IntoIterator<Item = widget::Button<'a, BBImagerMessage>>,
) -> Element<'a, BBImagerMessage> {
    let row2 = widget::row(
        [
            info_btn(INFO_ICON.clone()).into(),
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
    dev: &'a crate::db::Board,
    state: &'a crate::BBImagerCommon,
) -> Element<'a, BBImagerMessage> {
    let img: Element<BBImagerMessage> = match &dev.icon {
        Some(u) => match state.img_handle_cache.get(u) {
            Some(x) => x.view(iced::Length::Fill, iced::Shrink),
            None => iced_aw::Spinner::new().width(iced::Length::Fill).into(),
        },
        None => svg(BOARD_ICON.clone())
            .width(iced::Length::Fill)
            .style(svg_icon_style)
            .into(),
    };

    let copy_btn = copy_btn(COPY_ICON.clone()).on_press_with(|| {
        let json = serde_json::to_string_pretty(dev).expect("Invalid image");
        BBImagerMessage::CopyToClipboard(json)
    });

    let cols = widget::column![
        img,
        widget::center(copy_btn),
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

    let mut btns = Vec::with_capacity(2);

    if let Some(x) = &dev.documentation {
        btns.push(
            widget::button(widget::text("DOCUMENTATION"))
                .on_press(BBImagerMessage::OpenUrl(x.clone().into()))
                .into(),
        );
    }

    if let Some(x) = &dev.oshw
        && let Ok(u) = url::Url::parse(&format!("{}/{}.html", constants::OSHW_BASE_URL, x))
    {
        btns.push(
            widget::button(widget::text("OSHW"))
                .on_press(BBImagerMessage::OpenUrl(u))
                .into(),
        );
    }

    let cols = cols.push(widget::center(widget::row(btns).spacing(16)));

    widget::scrollable(cols.padding(VIEW_COL_PADDING))
        .id(state.scroll_id.clone())
        .into()
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
                size: (radius / 2.0).into(),
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
    widget::container(content).style(|_| {
        widget::container::Style::default()
            .background(constants::CARD)
            .border(iced::border::rounded(8))
    })
}

fn info_btn(handle: svg::Handle) -> widget::Button<'static, BBImagerMessage> {
    widget::button(svg(handle))
        .on_press(BBImagerMessage::AppInfo)
        .width(iced::Shrink)
        .height(iced::Shrink)
}

pub(crate) fn copy_btn<'a>(handle: svg::Handle) -> widget::Button<'a, BBImagerMessage> {
    widget::button(svg(handle))
        .width(iced::Shrink)
        .style(widget::button::secondary)
}

pub(crate) fn search_box<'a>(inp: &'a str) -> widget::Container<'a, BBImagerMessage> {
    widget::container(
        widget::row![
            widget::svg(SEARCH_ICON.clone())
                .style(svg_icon_style)
                .width(iced::Length::Shrink)
                .height(18),
            widget::text_input("SEARCH", inp)
                .style(|theme, status| {
                    let mut temp = widget::text_input::default(theme, status);
                    temp.border.width = 0.0;
                    temp.background = iced::Background::Color(iced::Color::TRANSPARENT);
                    temp
                })
                .on_input(BBImagerMessage::UpdateSearchText),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding(iced::Padding {
        left: 16.0,
        top: 16.0,
        bottom: 8.0,
        ..Default::default()
    })
}
