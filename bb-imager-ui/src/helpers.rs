use bb_iced_widgets::circle_bar;
use iced::{Element, widget};

use crate::{Message, constants};

pub(crate) const VIEW_COL_PADDING: u16 = 16;

/// |------|------|
/// |      |      |
/// |      | col2 |
/// | col1 |      |
/// |      |------|
/// |      | btns |
/// |------|------|
pub(crate) fn page_type1<'a>(
    col1: Element<'a, Message>,
    col2: Element<'a, Message>,
    btns: impl IntoIterator<Item = widget::Button<'a, Message>>,
) -> Element<'a, Message> {
    let row2 = widget::row(
        [
            info_btn(constants::INFO_ICON.clone()).into(),
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
    row1: Element<'a, Message>,
    btns: impl IntoIterator<Item = widget::Button<'a, Message>>,
) -> Element<'a, Message> {
    let row2 = widget::row(
        [
            info_btn(constants::INFO_ICON.clone()).into(),
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
    row1: Element<'a, Message>,
    btns: impl IntoIterator<Item = widget::Button<'a, Message>>,
) -> Element<'a, Message> {
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

/// Scrollable pane detailing whatever is currently selected in a [`list_pane`].
pub(crate) fn detail_pane<'a>(
    content: widget::Column<'a, Message>,
    scroll_id: &widget::Id,
) -> Element<'a, Message> {
    widget::scrollable(content.spacing(16).padding(VIEW_COL_PADDING))
        .id(scroll_id.clone())
        .into()
}

pub(crate) fn progress_finish_view<'a>(
    label: &'static str,
    color: iced::Color,
    details: impl widget::text::IntoFragment<'a>,
) -> Element<'a, Message> {
    widget::column![
        circle_bar(label, 10.0f32, color, constants::FONT_BOLD),
        widget::text(details)
    ]
    .align_x(iced::Center)
    .padding(VIEW_COL_PADDING)
    .into()
}

fn card_box<'a>(content: impl Into<Element<'a, Message>>) -> widget::Container<'a, Message> {
    widget::container(content).style(|_| {
        widget::container::Style::default()
            .background(constants::CARD)
            .border(iced::border::rounded(8))
    })
}

fn info_btn(handle: widget::svg::Handle) -> widget::Button<'static, Message> {
    widget::button(widget::svg(handle))
        .on_press(Message::GotoAppInfo)
        .width(iced::Shrink)
        .height(iced::Shrink)
}
