use bb_iced_widgets::circle_bar;
use iced::{Element, widget};

use crate::{Message, constants};

pub(crate) const VIEW_COL_PADDING: u16 = 16;
pub(crate) const LIST_COL_PADDING: iced::Padding = iced::Padding {
    right: 16.0,
    ..iced::Padding::ZERO
};

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

fn card_style(
    theme: &iced::Theme,
    is_selected: bool,
    is_hovered: bool,
) -> widget::container::Style {
    let mut style = widget::container::Style {
        text_color: Some(theme.palette().text),
        ..Default::default()
    };

    if is_selected || is_hovered {
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

/// Horizontal separator between rows of a list pane.
pub(crate) fn list_separator<'a>() -> Element<'a, Message> {
    widget::center(widget::rule::horizontal(2))
        .padding(iced::Padding::ZERO.left(16))
        .into()
}

/// Scrollable pane listing selectable items: a search box, a separator, any
/// extra `header` rows, then the `items` themselves.
pub(crate) fn list_pane<'a>(
    search_text: &'a str,
    scroll_id: &widget::Id,
    header: impl IntoIterator<Item = Element<'a, Message>>,
    items: impl IntoIterator<Item = Element<'a, Message>>,
) -> Element<'a, Message> {
    let top = [search_box(search_text).into(), list_separator()];

    widget::scrollable(
        widget::column(top.into_iter().chain(header).chain(items)).padding(LIST_COL_PADDING),
    )
    .id(scroll_id.clone())
    .into()
}

/// A selectable row of a [`list_pane`], laid out as a horizontal run of
/// `contents` (typically a leading icon followed by a label).
///
/// This is a [`widget::MouseArea`] around a plain container: a single click
/// selects via `on_press`, and a double click also advances to the next step
/// (the press replays before the double click, so the row is selected first).
/// The container cannot observe hover itself, so the row reports enter/leave
/// via [`Message::HoverRow`] and the page draws the outline around `hovered`.
pub(crate) fn list_item<'a>(
    contents: impl IntoIterator<Item = Element<'a, Message>>,
    is_selected: bool,
    index: usize,
    hovered: Option<usize>,
    msg: Message,
) -> Element<'a, Message> {
    let is_hovered = hovered == Some(index);
    widget::mouse_area(
        widget::container(
            widget::row(contents)
                .spacing(12)
                .padding(8)
                .align_y(iced::alignment::Vertical::Center),
        )
        .style(move |theme| card_style(theme, is_selected, is_hovered)),
    )
    .on_press(msg)
    .on_double_click(Message::Next)
    .on_enter(Message::HoverRow(Some(index)))
    .on_exit(Message::HoverRow(None))
    .into()
}

/// The primary label of a [`list_item`].
pub(crate) fn list_label<'a>(label: impl widget::text::IntoFragment<'a>) -> widget::Text<'a> {
    widget::text(label).size(18).width(iced::Length::Fill)
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
pub(crate) fn placeholder_pane<'a>(label: &'a str) -> Element<'a, Message> {
    widget::center(placeholder_heading(label))
        .padding(VIEW_COL_PADDING)
        .into()
}

/// A `key: value` line of a [`detail_pane`].
pub(crate) fn detail_entry<'a>(
    key: &'a str,
    val: impl widget::text::IntoFragment<'a>,
) -> widget::text::Rich<'a, (), Message> {
    widget::rich_text![
        widget::span(format!("{key}:")).font(constants::FONT_BOLD),
        widget::span(" "),
        widget::span(val),
    ]
}

pub(crate) fn copy_btn<'a>(handle: widget::svg::Handle) -> widget::Button<'a, Message> {
    widget::button(widget::svg(handle))
        .width(iced::Shrink)
        .style(widget::button::secondary)
}

/// A remotely fetched icon, falling back to `def` for items that have none.
///
/// Entries still in flight render as a spinner; see
/// [`bb_iced_widgets::cached_icon::Cache`].
pub(crate) fn network_image_or_default<'a>(
    cache: &'a bb_iced_widgets::cached_icon::Cache<std::sync::Arc<url::Url>>,
    img: Option<&std::sync::Arc<url::Url>>,
    def: widget::svg::Handle,
    width: impl Into<iced::Length>,
    height: impl Into<iced::Length>,
) -> Element<'a, Message> {
    match img {
        Some(u) => bb_iced_widgets::cached_icon(cache, u)
            .width(width)
            .height(height)
            .into(),
        None => widget::svg(def)
            .width(width)
            .height(height)
            .style(svg_icon_style)
            .into(),
    }
}

/// The pane detailing one board: icon, name, description, specification table
/// and its documentation/OSHW links.
pub(crate) fn board_details_pane<'a>(
    cache: &'a bb_iced_widgets::cached_icon::Cache<std::sync::Arc<url::Url>>,
    dev: &'a crate::board_selection::BoardDetails,
    scroll_id: &widget::Id,
) -> Element<'a, Message> {
    let img = network_image_or_default(
        cache,
        dev.icon.as_ref(),
        constants::BOARD_ICON.clone(),
        iced::Fill,
        iced::Shrink,
    );

    let copy_btn =
        copy_btn(constants::COPY_ICON.clone()).on_press(Message::CopyBoardConfig(dev.id));

    let cols = widget::column![
        img,
        widget::center(copy_btn),
        widget::text(dev.name.as_ref())
            .size(24)
            .align_x(iced::alignment::Alignment::Center)
            .width(iced::Length::Fill),
        widget::text(dev.description.as_ref())
            .align_x(iced::alignment::Alignment::Center)
            .width(iced::Length::Fill),
    ];

    let cols = cols.extend(
        dev.specification
            .iter()
            .map(|(k, v)| -> widget::text::Rich<'a, (), Message> { detail_entry(k, v.as_ref()) })
            .map(Into::into),
    );

    let mut btns = Vec::with_capacity(2);

    if let Some(x) = &dev.documentation {
        btns.push(
            widget::button(widget::text("DOCUMENTATION"))
                .on_press(Message::OpenUrl(x.clone()))
                .into(),
        );
    }

    if let Some(x) = &dev.oshw {
        btns.push(
            widget::button(widget::text("OSHW"))
                .on_press(Message::OpenUrl(x.clone()))
                .into(),
        );
    }

    detail_pane(
        cols.push(widget::center(widget::row(btns).spacing(16))),
        scroll_id,
    )
}

fn search_box<'a>(inp: &'a str) -> widget::Container<'a, Message> {
    widget::container(
        widget::row![
            widget::svg(constants::SEARCH_ICON.clone())
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
                .on_input(|x| Message::UpdateSearchText(x.into())),
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
