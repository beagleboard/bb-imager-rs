use iced::{Element, widget};

use crate::helpers::{VIEW_COL_PADDING, page_type3};
use crate::{Message, constants};

pub struct State {
    pub app_name: &'static str,
    pub app_release: &'static str,
    pub app_desc: &'static str,
    pub license: widget::text_editor::Content,
    pub cache_dir: std::sync::Arc<str>,
    pub log_path: std::sync::Arc<str>,
}

pub fn view<'a>(state: &'a State, scroll_id: widget::Id) -> Element<'a, Message> {
    page_type3(
        review_view(state, scroll_id),
        [widget::button("BACK")
            .on_press(Message::Back)
            .style(widget::button::secondary)],
    )
}

fn review_view<'a>(state: &'a State, scroll_id: widget::Id) -> Element<'a, Message> {
    let col = widget::column![
        widget::image(constants::WINDOW_ICON.clone()),
        state.app_name,
        state.app_release,
        state.app_desc,
        widget::rule::horizontal(2),
        input_with_label("Cache Directory", &state.cache_dir),
        widget::rule::horizontal(2),
        input_with_label("Log File", &state.log_path),
        widget::rule::horizontal(2),
        widget::container(widget::text_editor(&state.license).on_action(Message::EditorEvent))
            .padding(iced::Padding::ZERO.right(16))
    ]
    .spacing(8)
    .padding(VIEW_COL_PADDING)
    .width(iced::Fill)
    .align_x(iced::Center);

    widget::scrollable(col).id(scroll_id).into()
}

fn input_with_label<'a>(label: &'static str, value: &'a str) -> iced::Element<'a, Message> {
    widget::row![
        widget::text(label).width(150),
        widget::text_input(value, value).on_input(|_| Message::Null)
    ]
    .into()
}
