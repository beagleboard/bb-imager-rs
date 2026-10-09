use iced::{Element, widget};

use crate::{
    Message,
    constants::{self, BB_ICON, BUG_REPORT_ICON, FORMAT_ICON, ISSUE_TRACKER, SETTINGS_ICON},
};

pub(crate) const VIEW_COL_PADDING: u16 = 16;
const LIST_COL_PADDING: iced::Padding = iced::Padding {
    right: 16.0,
    ..iced::Padding::ZERO
};
const SIDEBAR_MARKER_SIZE: f32 = 20.0;

/// The sidebar entry a page belongs to, highlighted while it is shown.
///
/// Declared in flow order, so earlier steps compare less.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SidebarEntry {
    Hardware,
    FormatMedia,
    Software,
    Storage,
    Modify,
    Review,
    Write,
    AppOptions,
}

impl SidebarEntry {
    /// Text shown for this entry in the sidebar.
    const fn label(self) -> &'static str {
        match self {
            Self::Hardware => "Hardware",
            Self::Software => "Software",
            Self::Storage => "Storage",
            Self::Modify => "Modify",
            Self::Review => "Review",
            Self::Write => "Write",
            Self::FormatMedia => "Format Media",
            Self::AppOptions => "App Options",
        }
    }

    const fn msg(self) -> Message {
        Message::Goto(self)
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
    current: SidebarEntry,
    col1: Element<'a, Message>,
    col2: Element<'a, Message>,
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

    let col2 = widget::column![
        card_box(col2)
            .height(iced::Length::Fill)
            .width(iced::Length::Fill),
        row2.width(iced::Length::Fill)
    ]
    .spacing(24)
    .width(iced::FillPortion(1));

    with_sidebar(
        current,
        None,
        // Modify never comes before these pages.
        false,
        false,
        widget::row![
            card_box(col1)
                .height(iced::Length::Fill)
                .width(iced::Length::FillPortion(1)),
            col2
        ]
        .padding(24)
        .spacing(24),
    )
}

/// |--------|
/// |        |
/// |  row1  |
/// |        |
/// |--------|
/// |  btns  |
/// |--------|
pub(crate) fn page_type2<'a>(
    current: SidebarEntry,
    has_customization: bool,
    is_flashing: bool,
    row1: Element<'a, Message>,
    btns: impl IntoIterator<Item = widget::Button<'a, Message>>,
) -> Element<'a, Message> {
    with_sidebar(
        current,
        None,
        has_customization,
        is_flashing,
        page_type3(row1, btns),
    )
}

/// Same as [`page_type2`], for pages shown over another page, such as App
/// Options. The sidebar follows `previous`, the step of the page beneath.
pub(crate) fn page_type2_overlay<'a>(
    previous: SidebarEntry,
    has_customization: bool,
    is_flashing: bool,
    row1: Element<'a, Message>,
    btns: impl IntoIterator<Item = widget::Button<'a, Message>>,
) -> Element<'a, Message> {
    with_sidebar(
        SidebarEntry::AppOptions,
        Some(previous),
        has_customization,
        is_flashing,
        page_type3(row1, btns),
    )
}

/// Same as [`page_type2`], but without the sidebar. For pages outside the
/// flashing flow.
///
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

/// |--------|
/// |        |
/// |  row1  |
/// |        |
/// |--------|
pub(crate) fn page_type4<'a>(
    current: SidebarEntry,
    is_flashing: bool,
    row1: Element<'a, Message>,
) -> Element<'a, Message> {
    with_sidebar(
        current,
        None,
        false,
        is_flashing,
        widget::container(card_box(row1).height(iced::Fill).width(iced::Fill)).padding(24),
    )
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

fn card_btn_style(
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
pub(crate) fn list_item<'a>(
    contents: impl IntoIterator<Item = Element<'a, Message>>,
    is_selected: bool,
    msg: Message,
) -> widget::Button<'a, Message> {
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

/// How a sidebar entry is shown, and what clicking it sends.
#[derive(Debug, Clone)]
enum SidebarEntryState {
    Disabled,
    Selected,
    Enabled(Message),
}

fn sidebar<'a>(
    current: SidebarEntry,
    // Step of the page beneath an overlay such as App Options.
    previous: Option<SidebarEntry>,
    has_customization: bool,
    is_flashing: bool,
) -> Element<'a, Message> {
    let state = |entry: SidebarEntry, enabled: bool| {
        if current == entry {
            SidebarEntryState::Selected
        } else if enabled {
            SidebarEntryState::Enabled(entry.msg())
        } else {
            SidebarEntryState::Disabled
        }
    };
    // Only steps before the current page can be revisited, and Modify only
    // when this run has it. Under an overlay, the page beneath counts as the
    // current one, and its own step leads back to it.
    let flow_step = previous.unwrap_or(current);
    let step = |n, entry: SidebarEntry| {
        let enabled = (entry < flow_step || previous == Some(entry))
            && (entry != SidebarEntry::Modify || has_customization)
            // A running flash can't be left; App Options can only return to it.
            && (!is_flashing || entry == SidebarEntry::Write);
        let s = state(entry, enabled);
        sidebar_btn(step_badge(n, &s), entry.label(), s)
    };
    // A running flash can't be left, but App Options can return to a running format.
    let format_media = state(
        SidebarEntry::FormatMedia,
        !is_flashing || previous == Some(SidebarEntry::FormatMedia),
    );
    let app_options = state(SidebarEntry::AppOptions, true);
    let issue_tracker = SidebarEntryState::Enabled(Message::OpenUrl(ISSUE_TRACKER.clone()));

    widget::column![
        step(1, SidebarEntry::Hardware),
        step(2, SidebarEntry::Software),
        step(3, SidebarEntry::Storage),
        step(4, SidebarEntry::Modify),
        step(5, SidebarEntry::Review),
        step(6, SidebarEntry::Write),
        widget::space::vertical(),
        widget::rule::horizontal(2),
        sidebar_btn(
            sidebar_icon(FORMAT_ICON.clone(), &format_media),
            SidebarEntry::FormatMedia.label(),
            format_media,
        ),
        sidebar_btn(
            sidebar_icon(SETTINGS_ICON.clone(), &app_options),
            SidebarEntry::AppOptions.label(),
            app_options,
        ),
        sidebar_btn(
            sidebar_icon(BUG_REPORT_ICON.clone(), &issue_tracker),
            "Issue Tracker",
            issue_tracker,
        ),
        widget::container(
            widget::svg(BB_ICON.clone())
                .width(iced::Fill)
                .height(iced::Shrink)
        )
        .padding(iced::Padding::ZERO.top(8))
    ]
    .spacing(8)
    .padding(8)
    .width(160)
    .height(iced::Fill)
    .into()
}

fn sidebar_btn<'a>(
    leading: impl Into<Element<'a, Message>>,
    label: &'a str,
    state: SidebarEntryState,
) -> widget::Button<'a, Message> {
    let is_selected = matches!(state, SidebarEntryState::Selected);
    let on_press = match &state {
        SidebarEntryState::Enabled(msg) => Some(msg.clone()),
        SidebarEntryState::Disabled | SidebarEntryState::Selected => None,
    };

    widget::button(
        widget::row![leading.into(), widget::text(label)]
            .spacing(8)
            .align_y(iced::Center),
    )
    .on_press_maybe(on_press)
    .width(iced::Fill)
    .padding(8)
    .style(move |t, s| {
        let mut style = card_btn_style(t, s, is_selected);
        style.text_color = sidebar_color(t, &state);
        style
    })
}

/// Colour of a sidebar entry's label and marker.
fn sidebar_color(t: &iced::Theme, state: &SidebarEntryState) -> iced::Color {
    match state {
        SidebarEntryState::Selected => t.palette().primary,
        SidebarEntryState::Enabled(_) => t.palette().text,
        SidebarEntryState::Disabled => t.extended_palette().secondary.strong.color,
    }
}

/// Filled circle with a cut-out step number, marking a [`sidebar_btn`] of the
/// flashing flow. Coloured by [`sidebar_color`].
fn step_badge<'a>(n: u8, state: &SidebarEntryState) -> widget::Container<'a, Message> {
    let state = state.clone();

    widget::container(widget::text(n).size(15).font(constants::FONT_BOLD).style(
        |t: &iced::Theme| widget::text::Style {
            color: Some(t.palette().background),
        },
    ))
    .center(SIDEBAR_MARKER_SIZE)
    .style(move |t: &iced::Theme| {
        widget::container::Style::default()
            .background(sidebar_color(t, &state))
            .border(iced::border::rounded(SIDEBAR_MARKER_SIZE / 2.0))
    })
}

fn sidebar_icon<'a>(handle: widget::svg::Handle, state: &SidebarEntryState) -> widget::Svg<'a> {
    let state = state.clone();

    widget::svg(handle)
        .width(SIDEBAR_MARKER_SIZE)
        .height(SIDEBAR_MARKER_SIZE)
        .style(move |t, _| widget::svg::Style {
            color: Some(sidebar_color(t, &state)),
        })
}

fn with_sidebar<'a>(
    current: SidebarEntry,
    previous: Option<SidebarEntry>,
    has_customization: bool,
    is_flashing: bool,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    widget::row![
        sidebar(current, previous, has_customization, is_flashing),
        widget::rule::vertical(2),
        content.into()
    ]
    .into()
}
