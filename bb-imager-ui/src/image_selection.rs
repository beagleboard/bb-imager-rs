use std::borrow::Cow;
use std::sync::Arc;

use bb_config::config;
use iced::Element;
use iced::widget::{self, button, text};

use crate::helpers::{
    SidebarEntry, copy_btn, detail_entry, detail_pane, list_item, list_label, list_pane,
    page_type1, placeholder_pane, svg_icon_style,
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
}

/// One row of the image list.
#[derive(Debug, Clone)]
pub struct ImageItem {
    pub id: ImageId,
    pub icon: Option<Arc<url::Url>>,
    pub label: Cow<'static, str>,
}

/// The selected image, as the detail pane renders it.
#[derive(Debug, Clone)]
pub enum ImageDetails {
    Format,
    Local {
        flasher: config::Flasher,
        path: Box<std::path::Path>,
        size: u64,
        init_format: config::InitFormat,
    },
    Remote {
        id: i64,
        icon: Arc<url::Url>,
        title: Box<str>,
        description: Box<str>,
        details: Box<[(&'static str, Box<str>)]>,
        init_format: config::InitFormat,
        buttons: Box<[(&'static str, url::Url)]>,
        /// Rides along for the host; the page never reads it.
        flasher: config::Flasher,
        /// Rides along for the host; the page never reads it.
        file_name: Box<str>,
        /// Rides along for the host; the page never reads it.
        info_text: Option<Arc<str>>,
    },
}

impl ImageDetails {
    pub const fn id(&self) -> ImageId {
        match self {
            Self::Format => ImageId::Format,
            Self::Local { flasher, .. } => ImageId::Local(*flasher),
            Self::Remote { id, .. } => ImageId::OsImage(*id),
        }
    }
}

/// Init formats a local image can be customized with. More than one makes the
/// picker appear.
const fn init_formats(flasher: config::Flasher) -> &'static [config::InitFormat] {
    match flasher {
        config::Flasher::SdCard | config::Flasher::SdCardNoBootloader => {
            &[config::InitFormat::Sysconf, config::InitFormat::CloudInit]
        }
        _ => &[],
    }
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
        SidebarEntry::Software,
        os_list_pane(cache, state, &scroll_id),
        os_view_pane(cache, state, &scroll_id),
        [widget::button("NEXT").on_press_maybe(state.selected.as_ref().map(|_| Message::Next))],
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
                .map(|x| x.id() == img.id)
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

    let col = match img {
        ImageDetails::Format => widget::column![
            svg_big(constants::FORMAT_ICON.clone()),
            title("Format SD Card"),
            description("Format a SD Card to FAT32 for reuse."),
            detail_entry("Format", "FAT32"),
            detail_entry("Init Format", config::InitFormat::None.to_string()),
        ],
        ImageDetails::Local {
            flasher,
            path,
            size,
            init_format,
        } => {
            let path = path.to_string_lossy();
            let mut col = widget::column![
                svg_big(constants::FILE_ADD_ICON.clone()),
                title(path.clone()),
                detail_entry("Path", path),
                detail_entry("Size", size.to_string()),
            ];

            let formats = init_formats(*flasher);
            if formats.len() > 1 {
                let el = widget::pick_list(
                    formats,
                    if *init_format == config::InitFormat::None {
                        None
                    } else {
                        Some(*init_format)
                    },
                    Message::UpdateInitFormat,
                );
                col = col.push(
                    widget::row![text("Init Format: ").font(constants::FONT_BOLD), el]
                        .align_y(iced::Alignment::Center)
                        .padding(iced::Padding::ZERO.right(16)),
                );
            }

            col
        }
        ImageDetails::Remote {
            id,
            icon,
            title: name,
            description: desc,
            details,
            buttons,
            ..
        } => widget::column![
            bb_iced_widgets::cached_icon(cache, icon)
                .width(iced::Length::Fill)
                .height(100),
            widget::center(
                copy_btn(constants::COPY_ICON.clone()).on_press(Message::CopyImageConfig(*id)),
            ),
            title(name.as_ref()),
            description(desc.as_ref()),
        ]
        .extend(
            details
                .iter()
                .map(|(k, v)| detail_entry(k, v.as_ref()))
                .map(Into::into),
        )
        .extend(buttons.iter().map(|(label, link)| {
            widget::row![button(*label).on_press(Message::OpenUrl(link.clone()))]
                .spacing(16)
                .into()
        })),
    };

    detail_pane(col, scroll_id)
}

fn svg_big<'a>(handle: widget::svg::Handle) -> Element<'a, Message> {
    widget::svg(handle)
        .height(100)
        .width(iced::Length::Fill)
        .into()
}

fn title<'a>(t: impl text::IntoFragment<'a>) -> Element<'a, Message> {
    text(t)
        .size(24)
        .align_x(iced::alignment::Alignment::Center)
        .width(iced::Length::Fill)
        .into()
}

fn description<'a>(t: impl text::IntoFragment<'a>) -> Element<'a, Message> {
    text(t)
        .align_x(iced::alignment::Alignment::Center)
        .width(iced::Length::Fill)
        .into()
}

fn svg_sized<'a>(handle: widget::svg::Handle) -> Element<'a, Message> {
    widget::svg(handle)
        .height(ICON_WIDTH)
        .width(ICON_WIDTH)
        .style(svg_icon_style)
        .into()
}
