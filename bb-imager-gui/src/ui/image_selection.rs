use iced::{
    Element,
    widget::{self, button, text},
};

use crate::{
    constants,
    message::BBImagerMessage,
    ui::helpers::{card_btn_style, detail_entry, page_type1, svg_icon_style},
};

const ICON_WIDTH: u32 = 60;

pub(crate) fn view<'a>(state: &'a crate::ChooseOsState) -> Element<'a, BBImagerMessage> {
    page_type1(
        &state.common,
        os_list_pane(state),
        os_view_pane(state),
        [
            widget::button("BACK")
                .on_press(BBImagerMessage::Back)
                .style(widget::button::secondary),
            widget::button("NEXT")
                .on_press_maybe(state.selected_image().map(|_| BBImagerMessage::Next)),
        ],
    )
}

fn os_list_pane<'a>(state: &'a crate::ChooseOsState) -> Element<'a, BBImagerMessage> {
    match state.images() {
        Some(imgs) => {
            let items = imgs
                .map(|img| {
                    let is_selected = state
                        .selected_image
                        .as_ref()
                        .map(|(x, _)| *x == img.id)
                        .unwrap_or(false);

                    let icon: Element<BBImagerMessage> = match img.id {
                        crate::OsImageId::Format(_) => widget::svg(state.format_svg().clone())
                            .height(ICON_WIDTH)
                            .width(ICON_WIDTH)
                            .style(svg_icon_style)
                            .into(),
                        crate::OsImageId::Local(_) => widget::svg(state.file_add_svg().clone())
                            .height(ICON_WIDTH)
                            .width(ICON_WIDTH)
                            .style(svg_icon_style)
                            .into(),
                        crate::OsImageId::Remote(_) => {
                            match state
                                .image_handle_cache()
                                .get(img.icon.expect("Missing Os Image icon"))
                            {
                                Some(handle) => handle.view(ICON_WIDTH, ICON_WIDTH),
                                _ => widget::svg(state.downloading_svg().clone())
                                    .height(ICON_WIDTH)
                                    .width(ICON_WIDTH)
                                    .style(svg_icon_style)
                                    .into(),
                            }
                        }
                    };

                    let row =
                        widget::row![icon, text(img.label).size(18).width(iced::Length::Fill)];
                    let row = if img.is_sublist {
                        row.push(
                            widget::svg(state.arrow_forward_svg().clone())
                                .height(20)
                                .width(iced::Shrink)
                                .style(svg_icon_style),
                        )
                    } else {
                        row
                    };

                    button(
                        row.spacing(12)
                            .padding(8)
                            .align_y(iced::alignment::Vertical::Center),
                    )
                    .on_press(BBImagerMessage::SelectOs(img.id))
                    .style(move |theme, status| card_btn_style(theme, status, is_selected))
                })
                .map(Into::into);

            let col = if state.pos.is_empty() {
                widget::column(items)
            } else {
                let icon = widget::svg(state.arrow_back_svg().clone())
                    .height(ICON_WIDTH)
                    .width(ICON_WIDTH)
                    .style(svg_icon_style);
                let row = widget::row![icon, text("Back").size(18).width(iced::Length::Fill)]
                    .spacing(12)
                    .padding(8)
                    .align_y(iced::alignment::Vertical::Center);
                widget::column(
                    [button(row)
                        .on_press(BBImagerMessage::GotoOsListParent)
                        .style(move |theme, status| card_btn_style(theme, status, false))
                        .into()]
                    .into_iter()
                    .chain(items),
                )
            }
            .padding(iced::Padding::ZERO.right(12));

            widget::scrollable(col).into()
        }
        // TODO: Add loading circle
        None => text("Loading").into(),
    }
}

fn os_view_pane<'a>(state: &'a crate::ChooseOsState) -> Element<'a, BBImagerMessage> {
    match state.selected_image() {
        Some((_, img)) => {
            let icon = match img.icon() {
                crate::helpers::BoardImageIcon::Remote(url) => {
                    match state.image_handle_cache().get(url) {
                        Some(x) => x.view(iced::Length::Fill, 100),
                        None => widget::svg(state.downloading_svg().clone())
                            .width(iced::Length::Fill)
                            .into(),
                    }
                }
                crate::helpers::BoardImageIcon::Local => widget::svg(state.file_add_svg().clone())
                    .height(100)
                    .width(iced::Length::Fill)
                    .into(),
                crate::helpers::BoardImageIcon::Format => widget::svg(state.format_svg().clone())
                    .height(100)
                    .width(iced::Length::Fill)
                    .into(),
            };

            let col = widget::column![
                icon,
                text(img.to_string())
                    .size(24)
                    .align_x(iced::alignment::Alignment::Center)
                    .width(iced::Length::Fill),
            ]
            .spacing(16);

            // Add description if present
            let col = match img.description() {
                Some(x) => col
                    .push(
                        text(x)
                            .align_x(iced::alignment::Alignment::Center)
                            .width(iced::Length::Fill),
                    )
                    .width(iced::Length::Fill),
                None => col,
            };

            let col = col.extend(
                img.details()
                    .iter()
                    .map(|(k, v)| detail_entry(k, v))
                    .map(Into::into),
            );

            widget::scrollable(col).into()
        }
        None => {
            let col = widget::column![
                text("Please Select an OS")
                    .size(28)
                    .width(iced::Fill)
                    .align_x(iced::Center)
                    .font(constants::FONT_BOLD)
            ]
            .spacing(16);

            let col = match &state.selected_board().instructions {
                Some(x) => col.extend([
                    widget::rule::horizontal(2).into(),
                    text(format!(
                        "Special instructions for {}",
                        &state.selected_board().name
                    ))
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
