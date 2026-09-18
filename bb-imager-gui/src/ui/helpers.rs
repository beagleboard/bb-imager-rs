use std::sync::LazyLock;

use iced::Element;
use iced::widget::{self, svg};

use crate::{constants, message::BBImagerMessage};

pub(crate) static ARROW_BACK_ICON: LazyLock<svg::Handle> =
    LazyLock::new(|| svg::Handle::from_memory(constants::ARROW_BACK_ICON_BYTES));
pub(crate) static FILE_ADD_ICON: LazyLock<svg::Handle> =
    LazyLock::new(|| svg::Handle::from_memory(constants::FILE_ADD_ICON_BYTES));
pub(crate) static USB_ICON: LazyLock<svg::Handle> =
    LazyLock::new(|| svg::Handle::from_memory(constants::USB_ICON_BYTES));
pub(crate) static FORMAT_ICON: LazyLock<svg::Handle> =
    LazyLock::new(|| svg::Handle::from_memory(constants::FORMAT_ICON_BYTES));
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

/// Horizontal separator between rows of a list pane.
pub(crate) fn list_separator<'a>() -> Element<'a, BBImagerMessage> {
    widget::center(widget::rule::horizontal(2))
        .padding(iced::Padding::ZERO.left(16))
        .into()
}

/// Scrollable pane listing selectable items: a search box, a separator, any
/// extra `header` rows, then the `items` themselves.
pub(crate) fn list_pane<'a>(
    search_text: &'a str,
    scroll_id: &widget::Id,
    header: impl IntoIterator<Item = Element<'a, BBImagerMessage>>,
    items: impl IntoIterator<Item = Element<'a, BBImagerMessage>>,
) -> Element<'a, BBImagerMessage> {
    let top = [search_box(search_text).into(), list_separator()];

    widget::scrollable(
        widget::column(top.into_iter().chain(header).chain(items)).padding(LIST_COL_PADDING),
    )
    .id(scroll_id.clone())
    .into()
}

/// A selectable row of a [`list_pane`], laid out as a horizontal run of
/// `contents` (typically a leading icon followed by a label).
pub(crate) fn list_item<'a>(
    contents: impl IntoIterator<Item = Element<'a, BBImagerMessage>>,
    is_selected: bool,
    msg: BBImagerMessage,
) -> widget::Button<'a, BBImagerMessage> {
    widget::button(
        widget::row(contents)
            .spacing(12)
            .padding(8)
            .align_y(iced::alignment::Vertical::Center),
    )
    .on_press(msg)
    .style(move |theme, status| card_btn_style(theme, status, is_selected))
}

/// The primary label of a [`list_item`].
pub(crate) fn list_label<'a>(label: impl widget::text::IntoFragment<'a>) -> widget::Text<'a> {
    widget::text(label).size(18).width(iced::Length::Fill)
}

/// Scrollable pane detailing whatever is currently selected in a [`list_pane`].
pub(crate) fn detail_pane<'a>(
    content: widget::Column<'a, BBImagerMessage>,
    scroll_id: &widget::Id,
) -> Element<'a, BBImagerMessage> {
    widget::scrollable(content.spacing(16).padding(VIEW_COL_PADDING))
        .id(scroll_id.clone())
        .into()
}

/// Heading of a [`detail_pane`] with nothing selected yet.
pub(crate) fn placeholder_heading<'a>(label: &'a str) -> widget::Text<'a> {
    widget::text(label)
        .size(28)
        .width(iced::Fill)
        .align_x(iced::Center)
        .font(constants::FONT_BOLD)
}

/// A [`detail_pane`] with nothing selected yet.
pub(crate) fn placeholder_pane<'a>(label: &'a str) -> Element<'a, BBImagerMessage> {
    widget::center(placeholder_heading(label))
        .padding(VIEW_COL_PADDING)
        .into()
}

fn search_box<'a>(inp: &'a str) -> widget::Container<'a, BBImagerMessage> {
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
                .on_input(|x| BBImagerMessage::UpdateSearchText(x.into())),
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
