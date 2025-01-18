use iced::{
    widget::{self, button, text},
    Element,
};

use crate::{
    constants,
    helpers::{self, img_or_svg},
    BBImagerMessage,
};

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

#[derive(PartialEq, Eq, Clone, Debug)]
pub(crate) struct ImageSelectionPage {
    pub flasher: bb_imager::Flasher,
    pub idx: Vec<usize>,
}

impl ImageSelectionPage {
    pub fn new(flasher: bb_imager::Flasher) -> Self {
        Self {
            flasher,
            idx: Vec::with_capacity(3),
        }
    }

    pub fn view<'a, I, E>(
        &self,
        images: I,
        search_bar: &'a str,
        downloader: &'a bb_imager::download::Downloader,
        // Allow optional format entry
        extra_entries: E,
    ) -> Element<'a, BBImagerMessage>
    where
        I: Iterator<Item = (usize, &'a bb_imager::config::OsListItem)>,
        E: Iterator<Item = ExtraImageEntry>,
    {
        let items = images
            // TODO: Add search
            .filter(|(_, x)| {
                x.search_str()
                    .to_lowercase()
                    .contains(&search_bar.to_lowercase())
            })
            .map(|(idx, x)| self.entry(x, downloader, idx))
            .chain(extra_entries.map(|x| custom_btn(x.label, x.icon, x.msg)))
            .map(Into::into);

        widget::column![
            helpers::search_bar(search_bar),
            widget::horizontal_rule(2),
            widget::scrollable(widget::column(items).spacing(10))
        ]
        .spacing(10)
        .padding(10)
        .into()
    }

    fn entry_subitem<'a>(
        &self,
        image: &'a bb_imager::config::OsImage,
        downloader: &'a bb_imager::download::Downloader,
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

        let icon = match downloader.clone().check_image(&image.icon) {
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
            self.flasher,
        )))
        .style(widget::button::secondary)
    }

    fn entry<'a>(
        &self,
        item: &'a bb_imager::config::OsListItem,
        downloader: &'a bb_imager::download::Downloader,
        idx: usize,
    ) -> widget::Button<'a, BBImagerMessage> {
        match item {
            bb_imager::config::OsListItem::Image(image) => self.entry_subitem(image, downloader),
            bb_imager::config::OsListItem::SubList {
                name,
                description,
                icon,
                flasher,
                ..
            } => {
                let icon = match downloader.clone().check_image(icon) {
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
                        widget::column![text(name.as_str()).size(18), text(description.as_str())]
                            .padding(5)
                    ]
                    .align_y(iced::Alignment::Center)
                    .spacing(10),
                )
                .width(iced::Length::Fill)
                .on_press(self.push_screen(*flasher, idx))
                .style(widget::button::secondary)
            }
        }
    }

    fn push_screen(&self, flasher: bb_imager::Flasher, id: usize) -> BBImagerMessage {
        let mut idx = self.idx.clone();
        idx.push(id);
        BBImagerMessage::PushScreen(super::Screen::ImageSelection(Self { flasher, idx }))
    }
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
