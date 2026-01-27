use std::time::Duration;

use bb_config::config;
use iced::{
    Element,
    widget::{self, button, text},
};

use crate::{BBImagerMessage, constants, helpers, pages};

const ICON_WIDTH: u16 = 60;

pub(crate) struct ExtraImageEntry {
    label: &'static str,
    icon: &'static [u8],
    msg: BBImagerMessage,
}

impl ExtraImageEntry {
    pub(crate) const fn new(
        label: &'static str,
        icon: &'static [u8],
        msg: BBImagerMessage,
    ) -> Self {
        Self { label, icon, msg }
    }
}

pub(crate) fn view<'a>(
    state: &'a crate::pages::ImageSelectionState,
    images: Option<Vec<(usize, &'a config::OsListItem)>>,
    downloader: &'a bb_downloader::Downloader,
    // Allow optional format entry
    extra_entries: Vec<ExtraImageEntry>,
) -> Element<'a, BBImagerMessage> {
    let row3: Element<_> = if let Some(imgs) = images {
        let items = imgs
            .into_iter()
            .map(|(id, x)| entry(state, x, downloader, id))
            .chain(
                extra_entries
                    .into_iter()
                    .map(|x| custom_btn(x.label, x.icon, x.msg)),
            )
            .map(Into::into);

        widget::scrollable(widget::column(items).spacing(10)).into()
    } else {
        widget::center(
            iced_loading::circular::Circular::new()
                .size(80.0)
                .bar_height(6.0)
                .easing(&iced_loading::easing::STANDARD)
                .cycle_duration(Duration::from_secs(2)),
        )
        .into()
    };

    widget::column![
        super::helpers::nav_header(state.idx().is_empty()),
        widget::horizontal_rule(2),
        row3
    ]
    .spacing(10)
    .padding(10)
    .into()
}

fn entry_subitem<'a>(
    flasher: config::Flasher,
    image: &'a config::OsImage,
    downloader: &'a bb_downloader::Downloader,
) -> widget::Button<'a, BBImagerMessage> {
    let row3 = widget::row(
        [
            text(image.release_date.to_string()).into(),
            widget::horizontal_space().into(),
        ]
        .into_iter()
        .chain(image.tags.iter().map(|t| iced_aw::badge(t.as_str()).into())),
    )
    .align_y(iced::alignment::Vertical::Center)
    .spacing(5);

    let icon = match downloader.clone().check_cache_from_url(image.icon.clone()) {
        Some(y) => img_or_svg(y, ICON_WIDTH),
        None => widget::svg(widget::svg::Handle::from_memory(
            constants::DOWNLOADING_ICON,
        ))
        .width(ICON_WIDTH)
        .into(),
    };
    button(
        widget::row![
            icon,
            widget::column![
                text(image.name.as_str()).size(18),
                text(image.description.as_str()),
                row3
            ]
            .padding(5)
        ]
        .align_y(iced::Alignment::Center)
        .spacing(10),
    )
    .width(iced::Length::Fill)
    .on_press(BBImagerMessage::SelectImage(helpers::BoardImage::remote(
        image.clone(),
        flasher,
        downloader.clone(),
    )))
    .style(widget::button::secondary)
}

fn entry<'a>(
    state: &crate::pages::ImageSelectionState,
    item: &'a config::OsListItem,
    downloader: &'a bb_downloader::Downloader,
    id: usize,
) -> widget::Button<'a, BBImagerMessage> {
    fn internal<'a>(
        downloader: &'a bb_downloader::Downloader,
        icon: url::Url,
        name: &'a str,
        description: &'a str,
        msg: BBImagerMessage,
    ) -> widget::Button<'a, BBImagerMessage> {
        let icon = match downloader.clone().check_cache_from_url(icon) {
            Some(y) => img_or_svg(y, ICON_WIDTH),
            None => widget::svg(widget::svg::Handle::from_memory(
                constants::DOWNLOADING_ICON,
            ))
            .width(ICON_WIDTH)
            .into(),
        };
        let tail = widget::svg(widget::svg::Handle::from_memory(
            constants::ARROW_FORWARD_IOS_ICON,
        ))
        .width(20);
        button(
            widget::row![
                icon,
                widget::column![text(name).size(18), text(description)].padding(5),
                widget::horizontal_space(),
                tail
            ]
            .align_y(iced::Alignment::Center)
            .spacing(10),
        )
        .width(iced::Length::Fill)
        .on_press(msg)
        .style(widget::button::secondary)
    }

    match item {
        config::OsListItem::Image(image) => entry_subitem(state.flasher(), image, downloader),
        config::OsListItem::SubList(item) => internal(
            downloader,
            item.icon.clone(),
            &item.name,
            &item.description,
            push_screen(state, id, item.flasher),
        ),
        config::OsListItem::RemoteSubList(item) => internal(
            downloader,
            item.icon.clone(),
            &item.name,
            &item.description,
            push_screen(state, id, item.flasher),
        ),
    }
}

fn push_screen(
    state: &pages::ImageSelectionState,
    id: usize,
    flasher: config::Flasher,
) -> BBImagerMessage {
    BBImagerMessage::PushScreen(pages::Screen::ImageSelection(
        state.clone().with_added_id(id).with_flasher(flasher),
    ))
}

fn custom_btn<'a>(
    label: &'a str,
    icon: &'static [u8],
    msg: BBImagerMessage,
) -> widget::Button<'a, BBImagerMessage> {
    button(
        widget::row![
            widget::svg(widget::svg::Handle::from_memory(icon)).width(ICON_WIDTH),
            widget::container(text(label).size(18)).padding(5),
        ]
        .spacing(10),
    )
    .width(iced::Length::Fill)
    .on_press(msg)
    .style(widget::button::secondary)
}

pub(crate) fn img_or_svg<'a>(path: std::path::PathBuf, width: u16) -> Element<'a, BBImagerMessage> {
    let img = std::fs::read(path).expect("Failed to open image");

    match image::guess_format(&img) {
        Ok(_) => widget::image(widget::image::Handle::from_bytes(img))
            .width(width)
            .height(width)
            .into(),

        Err(_) => widget::svg(widget::svg::Handle::from_memory(img))
            .width(width)
            .height(width)
            .into(),
    }
}
