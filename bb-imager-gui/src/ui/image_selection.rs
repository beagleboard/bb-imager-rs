use iced::{
    Element,
    widget::{self, button, text},
};

use crate::{
    constants,
    message::BBImagerMessage,
    ui::helpers::{
        self, LIST_COL_PADDING, VIEW_COL_PADDING, card_btn_style, detail_entry, page_type1,
        svg_icon_style,
    },
};

const ICON_WIDTH: u32 = 60;

pub(crate) fn view<'a>(state: &'a crate::state::ChooseOsState) -> Element<'a, BBImagerMessage> {
    page_type1(
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

fn os_list_pane<'a>(state: &'a crate::state::ChooseOsState) -> Element<'a, BBImagerMessage> {
    if state.images.is_empty() {
        widget::center(
            iced_aw::Spinner::new()
                .width(50)
                .height(50)
                .circle_radius(3.0),
        )
        .into()
    } else {
        let items = state
            .images
            .iter()
            .map(|img| {
                let is_selected = state
                    .selected_image
                    .as_ref()
                    .map(|(x, _)| *x == img.id)
                    .unwrap_or(false);

                let icon: Element<BBImagerMessage> = match img.id {
                    crate::helpers::OsImageId::Format => widget::svg(helpers::FORMAT_ICON.clone())
                        .height(ICON_WIDTH)
                        .width(ICON_WIDTH)
                        .style(svg_icon_style)
                        .into(),
                    crate::helpers::OsImageId::Local(_) => {
                        widget::svg(helpers::FILE_ADD_ICON.clone())
                            .height(ICON_WIDTH)
                            .width(ICON_WIDTH)
                            .style(svg_icon_style)
                            .into()
                    }
                    crate::helpers::OsImageId::OsImage(_)
                    | crate::helpers::OsImageId::OsSublist(_) => {
                        match state
                            .image_handle_cache()
                            .get(img.icon.as_ref().expect("Missing Os Image icon"))
                        {
                            Some(handle) => handle.view(ICON_WIDTH, ICON_WIDTH),
                            _ => widget::svg(helpers::DOWNLOADING_ICON.clone())
                                .height(ICON_WIDTH)
                                .width(ICON_WIDTH)
                                .style(svg_icon_style)
                                .into(),
                        }
                    }
                };

                let row = widget::row![icon, text(img.label()).size(18).width(iced::Length::Fill)];
                let row = if img.is_sublist() {
                    row.push(
                        widget::svg(helpers::ARROW_FORWARD_IOS_ICON.clone())
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

        let col = widget::column![
            helpers::search_box(&state.search_text),
            widget::center(widget::rule::horizontal(2)).padding(iced::Padding {
                left: 16.0,
                ..Default::default()
            }),
        ];

        let col = if state.pos.is_none() {
            col.extend(items)
        } else {
            let icon = widget::svg(helpers::ARROW_BACK_ICON.clone())
                .height(ICON_WIDTH)
                .width(ICON_WIDTH)
                .style(svg_icon_style);
            let row = widget::row![icon, text("Back").size(18).width(iced::Length::Fill)]
                .spacing(12)
                .padding(8)
                .align_y(iced::alignment::Vertical::Center);
            col.extend(
                [button(row)
                    .on_press(BBImagerMessage::GotoOsListParent)
                    .style(move |theme, status| card_btn_style(theme, status, false))
                    .into()]
                .into_iter()
                .chain(items),
            )
        };

        widget::scrollable(col.padding(LIST_COL_PADDING))
            .id(state.common.scroll_id.clone())
            .into()
    }
}

fn os_view_pane<'a>(state: &'a crate::state::ChooseOsState) -> Element<'a, BBImagerMessage> {
    match state.selected_image() {
        Some((_, img)) => {
            let icon = match img.icon() {
                crate::helpers::BoardImageIcon::Remote(url) => {
                    match state.image_handle_cache().get(url) {
                        Some(x) => x.view(iced::Length::Fill, 100),
                        None => widget::svg(helpers::DOWNLOADING_ICON.clone())
                            .width(iced::Length::Fill)
                            .into(),
                    }
                }
                crate::helpers::BoardImageIcon::Local => {
                    widget::svg(helpers::FILE_ADD_ICON.clone())
                        .height(100)
                        .width(iced::Length::Fill)
                        .into()
                }
                crate::helpers::BoardImageIcon::Format => widget::svg(helpers::FORMAT_ICON.clone())
                    .height(100)
                    .width(iced::Length::Fill)
                    .into(),
            };

            let mut col = widget::column![icon];

            // Add button to copy image info when it makes sense.
            if let Some(json) = state.img_json() {
                col = col.push(widget::center(
                    helpers::copy_btn(helpers::COPY_ICON.clone())
                        .on_press(BBImagerMessage::CopyToClipboard(json)),
                ));
            }

            col = col.push(
                text(img.to_string())
                    .size(24)
                    .align_x(iced::alignment::Alignment::Center)
                    .width(iced::Length::Fill),
            );

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

            widget::scrollable(col.spacing(16).padding(VIEW_COL_PADDING))
                .id(state.common.scroll_id.clone())
                .into()
        }
        None => {
            let col = widget::column![
                text("Please Select an OS")
                    .size(28)
                    .width(iced::Fill)
                    .align_x(iced::Center)
                    .font(constants::FONT_BOLD)
            ];

            widget::center(col.padding(VIEW_COL_PADDING)).into()
        }
    }
}
