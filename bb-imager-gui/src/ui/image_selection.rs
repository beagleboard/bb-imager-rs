use iced::{
    Element,
    widget::{self, button, text},
};

use crate::{
    constants,
    message::BBImagerMessage,
    ui::helpers::{self, detail_entry, page_type1, svg_icon_style},
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
                .on_press_maybe(state.selected_image.as_ref().map(|_| BBImagerMessage::Next)),
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
                        state.common.img_handle_cache.view(
                            img.icon.as_ref().expect("Missing Os Image icon"),
                            ICON_WIDTH,
                            ICON_WIDTH,
                        )
                    }
                };

                let mut contents = vec![icon, helpers::list_label(img.label()).into()];
                if img.is_sublist() {
                    contents.push(
                        widget::svg(helpers::ARROW_FORWARD_IOS_ICON.clone())
                            .height(20)
                            .width(iced::Shrink)
                            .style(svg_icon_style)
                            .into(),
                    );
                }

                helpers::list_item(contents, is_selected, BBImagerMessage::SelectOs(img.id))
            })
            .map(Into::into);

        // Nested sublists get a row to walk back up to their parent.
        let back: Vec<Element<BBImagerMessage>> = if state.pos.is_none() {
            Vec::new()
        } else {
            let icon = widget::svg(helpers::ARROW_BACK_ICON.clone())
                .height(ICON_WIDTH)
                .width(ICON_WIDTH)
                .style(svg_icon_style);
            vec![
                helpers::list_item(
                    [icon.into(), helpers::list_label("Back").into()],
                    false,
                    BBImagerMessage::GotoOsListParent,
                )
                .into(),
            ]
        };

        helpers::list_pane(&state.search_text, &state.common.scroll_id, back, items)
    }
}

fn os_view_pane<'a>(state: &'a crate::state::ChooseOsState) -> Element<'a, BBImagerMessage> {
    match state.selected_image.as_ref() {
        Some((_, img)) => {
            let icon = match img.icon() {
                crate::helpers::BoardImageIcon::Remote(url) => {
                    state
                        .common
                        .img_handle_cache
                        .view(url, iced::Length::Fill, 100)
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

            let mut col = col.extend(
                img.details()
                    .iter()
                    .map(|(k, v)| detail_entry(k, v))
                    .map(Into::into),
            );

            let init_formats = img.supported_init_formats();
            if init_formats.len() > 1 {
                let init_format = img.init_format();
                let el = widget::pick_list(
                    init_formats,
                    if init_format == bb_config::config::InitFormat::None {
                        None
                    } else {
                        Some(init_format)
                    },
                    BBImagerMessage::UpdateInitFormat,
                );
                col = col.push(
                    widget::row![text("Init Format: ").font(constants::FONT_BOLD), el]
                        .align_y(iced::Alignment::Center)
                        .padding(iced::Padding::ZERO.right(16)),
                )
            } else if init_formats.len() == 1 {
                col = col.push(detail_entry("Init Format", init_formats[0].to_string()))
            }

            if let Some(x) = img.support() {
                let row =
                    widget::row![button("SUPPORT").on_press(BBImagerMessage::OpenUrl(x.clone()))]
                        .spacing(16);
                col = col.push(widget::center(row));
            }

            helpers::detail_pane(col, &state.common.scroll_id)
        }
        None => helpers::placeholder_pane("Please Select an OS"),
    }
}
