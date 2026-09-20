use std::borrow::Cow;
use std::sync::Arc;

use bb_config::config;
use iced::Element;
use iced::widget::{self, button, text};

use crate::helpers::{
    copy_btn, detail_entry, detail_pane, list_item, list_label, list_pane, page_type1,
    placeholder_pane, svg_icon_style,
};
use crate::{Message, constants};

const ICON_WIDTH: u32 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageId {
    Format,
    /// The flasher rides along so the host can pick a file filter without a
    /// second lookup; the page never reads it.
    Local(config::Flasher),
    OsImage(i64),
    OsSublist((i64, config::Flasher)),
}

impl ImageId {
    const fn is_sublist(&self) -> bool {
        matches!(self, Self::OsSublist(_))
    }

    /// Id of this image's config entry, if it has one. Local files and the
    /// format action are not in the config, so there is nothing to copy.
    const fn config_id(&self) -> Option<i64> {
        match self {
            Self::OsImage(id) => Some(*id),
            _ => None,
        }
    }
}

/// One row of the image list.
#[derive(Debug, Clone)]
pub struct ImageItem {
    pub id: ImageId,
    pub icon: Option<Arc<url::Url>>,
    pub label: Cow<'static, str>,
}

/// Which icon the detail pane shows; local files and the format action have no
/// remote icon to fetch.
#[derive(Debug, Clone)]
pub enum ImageIcon {
    Remote(Arc<url::Url>),
    Local,
    Format,
}

/// The selected image, as the detail pane renders it.
#[derive(Debug, Clone)]
pub struct ImageDetails {
    pub id: ImageId,
    pub icon: ImageIcon,
    pub title: Box<str>,
    pub description: Option<Box<str>>,
    pub details: Box<[(Box<str>, Box<str>)]>,
    /// More than one makes the picker appear; exactly one is shown as plain text.
    pub init_formats: &'static [config::InitFormat],
    pub init_format: config::InitFormat,
    pub support: Option<url::Url>,
    pub sbom: Option<url::Url>,
}

#[derive(Default, Debug)]
pub struct State {
    pub images: Box<[ImageItem]>,
    pub selected: Option<ImageDetails>,
    /// Current sublist; `None` is the root. The host owns the tree — the page
    /// only reports which way to move and is handed a fresh flat list.
    pub pos: Option<i64>,
    /// Only here so the search box can show what was typed; the host does the
    /// filtering and hands back a new [`State::images`].
    pub search: Arc<str>,
}

pub fn view<'a>(
    cache: &'a bb_iced_widgets::cached_icon::Cache<Arc<url::Url>>,
    state: &'a State,
    scroll_id: widget::Id,
) -> Element<'a, Message> {
    page_type1(
        os_list_pane(cache, state, &scroll_id),
        os_view_pane(cache, state, &scroll_id),
        [
            widget::button("BACK")
                .on_press(Message::Back)
                .style(widget::button::secondary),
            widget::button("NEXT").on_press_maybe(state.selected.as_ref().map(|_| Message::Next)),
        ],
    )
}

fn os_list_pane<'a>(
    cache: &'a bb_iced_widgets::cached_icon::Cache<Arc<url::Url>>,
    state: &'a State,
    scroll_id: &widget::Id,
) -> Element<'a, Message> {
    if state.images.is_empty() {
        return widget::center(
            iced_aw::Spinner::new()
                .width(50)
                .height(50)
                .circle_radius(3.0),
        )
        .into();
    }

    let items = state
        .images
        .iter()
        .map(|img| {
            let is_selected = state
                .selected
                .as_ref()
                .map(|x| x.id == img.id)
                .unwrap_or(false);

            let icon: Element<Message> = match img.id {
                ImageId::Format => svg_sized(constants::FORMAT_ICON.clone()),
                ImageId::Local(_) => svg_sized(constants::FILE_ADD_ICON.clone()),
                ImageId::OsImage(_) | ImageId::OsSublist(_) => bb_iced_widgets::cached_icon(
                    cache,
                    img.icon.as_ref().expect("Missing Os Image icon"),
                )
                .width(ICON_WIDTH)
                .height(ICON_WIDTH)
                .into(),
            };

            let mut contents = vec![icon, list_label(img.label.as_ref()).into()];
            if img.id.is_sublist() {
                contents.push(
                    widget::svg(constants::ARROW_FORWARD_IOS_ICON.clone())
                        .height(20)
                        .width(iced::Shrink)
                        .style(svg_icon_style)
                        .into(),
                );
            }

            list_item(contents, is_selected, Message::SelectOs(img.id))
        })
        .map(Into::into);

    // Nested sublists get a row to walk back up to their parent.
    let back: Vec<Element<Message>> = if state.pos.is_none() {
        Vec::new()
    } else {
        vec![
            list_item(
                [
                    svg_sized(constants::ARROW_BACK_ICON.clone()),
                    list_label("Back").into(),
                ],
                false,
                Message::GotoOsListParent,
            )
            .into(),
        ]
    };

    list_pane(&state.search, scroll_id, back, items)
}

fn os_view_pane<'a>(
    cache: &'a bb_iced_widgets::cached_icon::Cache<Arc<url::Url>>,
    state: &'a State,
    scroll_id: &widget::Id,
) -> Element<'a, Message> {
    let Some(img) = state.selected.as_ref() else {
        return placeholder_pane("Please Select an OS");
    };

    let icon: Element<'a, Message> = match &img.icon {
        ImageIcon::Remote(url) => bb_iced_widgets::cached_icon(cache, url)
            .width(iced::Length::Fill)
            .height(100)
            .into(),
        ImageIcon::Local => widget::svg(constants::FILE_ADD_ICON.clone())
            .height(100)
            .width(iced::Length::Fill)
            .into(),
        ImageIcon::Format => widget::svg(constants::FORMAT_ICON.clone())
            .height(100)
            .width(iced::Length::Fill)
            .into(),
    };

    let mut col = widget::column![icon];

    // Add button to copy image info when it makes sense.
    if let Some(id) = img.id.config_id() {
        col = col.push(widget::center(
            copy_btn(constants::COPY_ICON.clone()).on_press(Message::CopyImageConfig(id)),
        ));
    }

    col = col.push(
        text(img.title.as_ref())
            .size(24)
            .align_x(iced::alignment::Alignment::Center)
            .width(iced::Length::Fill),
    );

    // Add description if present
    let col = match img.description.as_ref() {
        Some(x) => col
            .push(
                text(x.as_ref())
                    .align_x(iced::alignment::Alignment::Center)
                    .width(iced::Length::Fill),
            )
            .width(iced::Length::Fill),
        None => col,
    };

    let mut col = col.extend(
        img.details
            .iter()
            .map(|(k, v)| detail_entry(k, v.as_ref()))
            .map(Into::into),
    );

    if img.init_formats.len() > 1 {
        let el = widget::pick_list(
            img.init_formats,
            if img.init_format == config::InitFormat::None {
                None
            } else {
                Some(img.init_format)
            },
            Message::UpdateInitFormat,
        );
        col = col.push(
            widget::row![text("Init Format: ").font(constants::FONT_BOLD), el]
                .align_y(iced::Alignment::Center)
                .padding(iced::Padding::ZERO.right(16)),
        )
    } else if img.init_formats.len() == 1 {
        col = col.push(detail_entry("Init Format", img.init_formats[0].to_string()))
    }

    if let Some(x) = img.support.as_ref() {
        let row = widget::row![button("SUPPORT").on_press(Message::OpenUrl(x.clone()))].spacing(16);
        col = col.push(widget::center(row));
    }

    if let Some(x) = img.sbom.as_ref() {
        let row = widget::row![button("SBOM").on_press(Message::OpenUrl(x.clone()))].spacing(16);
        col = col.push(widget::center(row));
    }

    detail_pane(col, scroll_id)
}

fn svg_sized<'a>(handle: widget::svg::Handle) -> Element<'a, Message> {
    widget::svg(handle)
        .height(ICON_WIDTH)
        .width(ICON_WIDTH)
        .style(svg_icon_style)
        .into()
}
