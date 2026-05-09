use iced::{
    Element,
    widget::{self, button, text},
};

use crate::{
    BBImagerMessage, constants,
    helpers::DestinationItem,
    state::ChooseDestState,
    ui::helpers::{
        self, LIST_COL_PADDING, VIEW_COL_PADDING, card_btn_style, detail_entry, page_type1,
        svg_icon_style,
    },
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

            let row = widget::row![
                icon,
                text(dest.to_string()).size(18).width(iced::Length::Fill)
            ];
            button(
                row.spacing(12)
                    .padding(8)
                    .align_y(iced::alignment::Vertical::Center),
            )
            .on_press(dest.msg())
            .style(move |theme, status| card_btn_style(theme, status, is_selected))
        })
        .map(Into::into);

    let hrule_pad = iced::Padding {
        left: 16.0,
        ..Default::default()
    };

    widget::scrollable(
        widget::column(
            [
                helpers::search_box(&state.search_text).into(),
                widget::center(widget::rule::horizontal(2))
                    .padding(hrule_pad)
                    .into(),
                widget::container(
                    widget::toggler(!state.filter_destination)
                        .label("Show all destinations")
                        .on_toggle(|x| BBImagerMessage::DestinationFilter(!x)),
                )
                .padding(16)
                .into(),
                widget::center(widget::rule::horizontal(2))
                    .padding(hrule_pad)
                    .into(),
            ]
            .into_iter()
            .chain(items),
        )
        .padding(LIST_COL_PADDING),
    )
    .id(state.common.scroll_id.clone())
    .into()
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

            widget::scrollable(col.spacing(16).padding(VIEW_COL_PADDING))
                .id(state.common.scroll_id.clone())
                .into()
        }
        None => {
            let col = widget::column![
                text("Please Select a Destination")
                    .size(28)
                    .width(iced::Fill)
                    .align_x(iced::Center)
                    .font(constants::FONT_BOLD)
            ];

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

            widget::center(
                widget::scrollable(col.padding(VIEW_COL_PADDING).spacing(16))
                    .id(state.common.scroll_id.clone()),
            )
            .into()
        }
    }
}
