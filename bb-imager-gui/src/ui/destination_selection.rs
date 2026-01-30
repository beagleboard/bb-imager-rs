use iced::{
    Element,
    widget::{self, button, text},
};

use crate::{
    BBImagerMessage, constants,
    ui::helpers::{card_btn_style, detail_entry, page_type1, svg_icon_style},
};

const ICON_WIDTH: u32 = 60;

pub(crate) fn view<'a>(state: &'a crate::ChooseDestState) -> Element<'a, BBImagerMessage> {
    page_type1(
        &state.common,
        dest_list_pane(state),
        dest_view_pane(state),
        [
            widget::button("BACK")
                .on_press(BBImagerMessage::Back)
                .style(widget::button::secondary),
            widget::button("NEXT")
                .on_press_maybe(state.selected_dest().map(|_| BBImagerMessage::Next)),
        ],
    )
}

fn dest_list_pane<'a>(state: &'a crate::ChooseDestState) -> Element<'a, BBImagerMessage> {
    let items = state
        .destinations()
        .map(|dest| {
            let is_selected = state
                .selected_dest
                .as_ref()
                .map(|x| dest.is_selected(x))
                .unwrap_or(false);

            let icon: Element<BBImagerMessage> = match dest {
                crate::DestinationItem::SaveToFile(_) => {
                    widget::svg(state.file_save_icon().clone())
                }
                crate::DestinationItem::Destination(_) => widget::svg(state.usb_svg().clone()),
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

    widget::scrollable(widget::column(items).padding(iced::Padding::ZERO.right(12))).into()
}

fn dest_view_pane<'a>(state: &'a crate::ChooseDestState) -> Element<'a, BBImagerMessage> {
    match state.selected_dest() {
        Some(dest) => {
            let icon: Element<BBImagerMessage> = widget::svg(state.usb_svg().clone())
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
            ]
            .spacing(16);

            let col = col.extend(
                dest.details()
                    .into_iter()
                    .map(|(k, v)| detail_entry(k, v))
                    .map(Into::into),
            );

            widget::scrollable(col).into()
        }
        None => {
            let col = widget::column![
                text("Please Select a Destination")
                    .size(28)
                    .width(iced::Fill)
                    .align_x(iced::Center)
                    .font(constants::FONT_BOLD)
            ]
            .spacing(16);

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

            widget::center(widget::scrollable(col)).into()
        }
    }
}
