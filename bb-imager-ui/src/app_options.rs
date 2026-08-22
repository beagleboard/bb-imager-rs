use iced::widget;

use crate::Message;
use crate::constants::FONT_BOLD;
use crate::helpers::page_layout;

const HEADING_SIZE: u32 = 26;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviousPage {
    #[default]
    Device,
    Software,
    Destination,
    Customization,
    Review {
        has_customization: bool,
    },
    Flashing {
        has_customization: bool,
    },
}

impl PreviousPage {
    const fn item<D>(&self) -> (&'static str, bool, Option<Message<D>>) {
        match self {
            Self::Device => ("Device", false, Some(Message::GotoDevicePage)),
            Self::Software => ("Software", false, Some(Message::GotoSoftwarePage)),
            Self::Destination => ("Destination", false, Some(Message::GotoDestinationPage)),
            Self::Customization => ("Customization", false, Some(Message::GotoCustomizationPage)),
            Self::Review { .. } => ("Review", false, Some(Message::GotoReviewPage)),
            Self::Flashing { .. } => ("Flashing", false, Some(Message::GotoFlashingPage)),
        }
    }

    fn items<D>(&self) -> Vec<(&'static str, bool, Option<Message<D>>)> {
        match self {
            Self::Device => vec![self.item()],
            Self::Software => vec![Self::Device.item(), self.item()],
            Self::Destination => vec![Self::Device.item(), Self::Software.item(), self.item()],
            Self::Customization => vec![
                Self::Device.item(),
                Self::Software.item(),
                Self::Destination.item(),
                self.item(),
            ],
            Self::Review { has_customization } => {
                let mut temp = vec![
                    Self::Device.item(),
                    Self::Software.item(),
                    Self::Destination.item(),
                ];

                if *has_customization {
                    temp.push(Self::Customization.item());
                }

                temp.push(self.item());

                temp
            }
            Self::Flashing { has_customization } => {
                let mut temp = vec![
                    ("Device", false, None),
                    ("Software", false, None),
                    ("Destination", false, None),
                ];

                if *has_customization {
                    temp.push(("Customization", false, None));
                }

                temp.extend([("Review", false, None), self.item()]);

                temp
            }
        }
    }
}

#[derive(Default)]
pub struct State {
    pub previous_page: PreviousPage,
    pub cache_dir: std::sync::Arc<str>,
    pub log_file: std::sync::Arc<str>,
    pub license: widget::text_editor::Content,
}

pub fn view<'a, D: Clone + 'a>(
    s: &'a State,
    scroll_id: widget::Id,
) -> iced::Element<'a, Message<D>> {
    page_layout(
        (s.previous_page.items(), [("App Options", true, None)]),
        widget::scrollable(
            widget::column![
                widget::text("App Options")
                    .font(FONT_BOLD)
                    .size(HEADING_SIZE),
                input_with_label("Cache Directory", &s.cache_dir),
                input_with_label("Log File", &s.log_file),
                widget::text_editor(&s.license).on_action(Message::EditorEvent)
            ]
            .padding(16)
            .spacing(16),
        )
        .id(scroll_id)
        .height(iced::Fill),
    )
}

fn input_with_label<'a, D: Clone + 'a>(
    label: &'static str,
    value: &'a str,
) -> iced::Element<'a, Message<D>> {
    widget::row![
        widget::text(label).width(150),
        widget::text_input(value, value).on_input(|_| Message::Null)
    ]
    .into()
}
