use iced::{
    Element,
    widget::{self, text},
};

use crate::{
    BBImagerMessage, constants,
    helpers::DestinationItem,
    state::ChooseDestState,
    ui::helpers::{self, detail_entry, page_type1, svg_icon_style},
};

const ICON_WIDTH: u32 = 60;

pub(crate) fn view<'a>(state: &'a ChooseDestState) -> Element<'a, BBImagerMessage> {
    page_type1(
        dest_list_pane(state),
        dest_view_pane(state),
        [
            widget::button("BACK")
                .on_press(BBImagerMessage::Back)
                .style(widget::button::secondary),
            widget::button("NEXT")
                .on_press_maybe(state.selected_dest.as_ref().map(|_| BBImagerMessage::Next)),
        ],
    )
}

fn dest_list_pane<'a>(state: &'a ChooseDestState) -> Element<'a, BBImagerMessage> {
    let items = state
        .destinations()
        .map(|dest| {
            let is_selected = state
                .selected_dest
                .as_ref()
                .map(|x| dest.is_selected(x))
                .unwrap_or(false);

            let icon: Element<BBImagerMessage> = match dest {
                DestinationItem::SaveToFile(_) => widget::svg(helpers::FILE_SAVE_ICON.clone()),
                DestinationItem::Destination(_) => widget::svg(helpers::USB_ICON.clone()),
            }
            .height(ICON_WIDTH)
            .width(ICON_WIDTH)
            .style(svg_icon_style)
            .into();

            let label: Element<'_, _> = match dest.subtitle() {
                Some(x) => widget::column![text(dest.to_string()).size(18), text(x)]
                    .width(iced::Length::Fill)
                    .into(),
                None => helpers::list_label(dest.to_string()).into(),
            };

            helpers::list_item([icon, label], is_selected, dest.msg())
        })
        .map(Into::into);

    let filter_toggle = widget::container(
        widget::toggler(!state.filter_destination)
            .label("Show all destinations")
            .on_toggle(|x| BBImagerMessage::DestinationFilter(!x)),
    )
    .padding(16);

    helpers::list_pane(
        &state.search_text,
        &state.common.scroll_id,
        [filter_toggle.into(), helpers::list_separator()],
        items,
    )
}

fn dest_view_pane<'a>(state: &'a crate::state::ChooseDestState) -> Element<'a, BBImagerMessage> {
    match state.selected_dest.as_ref() {
        Some(dest) => {
            let icon: Element<BBImagerMessage> = widget::svg(helpers::USB_ICON.clone())
                .height(100)
                .width(iced::Fill)
                .style(svg_icon_style)
                .into();

            let col = widget::column![
                icon,
                text(dest.to_string())
                    .size(24)
                    .align_x(iced::alignment::Alignment::Center)
                    .width(iced::Length::Fill),
            ];

            let col = col.extend(
                dest.details()
                    .into_iter()
                    .map(|(k, v)| detail_entry(k, v))
                    .map(Into::into),
            );

            helpers::detail_pane(col, &state.common.scroll_id)
        }
        None => {
            let col = widget::column![helpers::placeholder_heading("Please Select a Destination")];

            let col = match state.instruction() {
                Some(x) => col.extend([
                    widget::rule::horizontal(2).into(),
                    text("Special instructions")
                        .size(16)
                        .font(constants::FONT_BOLD)
                        .into(),
                    text(x).into(),
                ]),
                None => col,
            };

            widget::center(helpers::detail_pane(col, &state.common.scroll_id)).into()
        }
    }
}
