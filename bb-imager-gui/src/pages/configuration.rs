use iced::{
    Element,
    widget::{self, text},
};

use crate::{
    BBImagerMessage,
    helpers::{self, home_btn_text},
};

pub fn view<'a>(
    customization: &'a FlashingCustomization,
    timezones: &'a widget::combo_box::State<String>,
    keymaps: &'a widget::combo_box::State<String>,
) -> Element<'a, BBImagerMessage> {
    widget::responsive(move |size| {
        const HEADER_FOOTER_HEIGHT: f32 = 150.0;

        let action_btn_row = widget::row![
            home_btn_text("RESET", true, iced::Length::Fill)
                .style(widget::button::secondary)
                .width(iced::Length::FillPortion(1))
                .on_press(BBImagerMessage::ResetConfig),
            widget::horizontal_space().width(iced::Length::FillPortion(5)),
            home_btn_text("SAVE", true, iced::Length::Fill)
                .style(widget::button::secondary)
                .width(iced::Length::FillPortion(1))
                .on_press(BBImagerMessage::PopScreen),
        ]
        .padding(4)
        .width(iced::Length::Fill);

        let form = match customization {
            FlashingCustomization::LinuxSd(x) => linux_sd_form(timezones, keymaps, x),
            FlashingCustomization::Bcf(x) => widget::column![
                widget::toggler(!x.verify)
                    .label("Skip Verification")
                    .on_toggle(move |y| {
                        BBImagerMessage::UpdateFlashConfig(FlashingCustomization::Bcf(
                            x.clone().update_verify(!y),
                        ))
                    })
            ],
            #[cfg(feature = "pb2_mspm0")]
            FlashingCustomization::Pb2Mspm0 { persist_eeprom } => {
                widget::column![
                    widget::toggler(*persist_eeprom)
                        .label("Persist EEPROM")
                        .on_toggle(move |y| {
                            BBImagerMessage::UpdateFlashConfig(FlashingCustomization::Pb2Mspm0 {
                                persist_eeprom: y,
                            })
                        })
                ]
            }
            _ => widget::column([]),
        }
        .spacing(5);

        widget::column![
            text("Extra Configuration").size(28),
            widget::horizontal_rule(2),
            widget::scrollable(form).height(size.height - HEADER_FOOTER_HEIGHT),
            widget::horizontal_rule(2),
            action_btn_row,
        ]
        .spacing(10)
        .padding(10)
        .height(iced::Length::Fill)
        .width(iced::Length::Fill)
        .align_x(iced::Alignment::Center)
        .into()
    })
    .into()
}

fn linux_sd_form<'a>(
    timezones: &'a widget::combo_box::State<String>,
    keymaps: &'a widget::combo_box::State<String>,
    config: &'a bb_imager::flasher::FlashingSdLinuxConfig,
) -> widget::Column<'a, BBImagerMessage> {
    widget::column![
        widget::container(
            widget::toggler(!config.verify())
                .label("Skip Verification")
                .on_toggle(|y| {
                    BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSd(
                        config.clone().update_verify(!y),
                    ))
                })
        )
        .padding(10)
        .width(iced::Length::Fill)
        .style(widget::container::bordered_box),
        hostname_form(config).width(iced::Length::Fill),
        timezone_form(timezones, config).width(iced::Length::Fill),
        keymap_form(keymaps, config).width(iced::Length::Fill),
        uname_pass_form(config).width(iced::Length::Fill),
        wifi_form(config).width(iced::Length::Fill)
    ]
}

fn keymap_form<'a>(
    keymaps: &'a widget::combo_box::State<String>,
    config: &'a bb_imager::flasher::FlashingSdLinuxConfig,
) -> widget::Container<'a, BBImagerMessage> {
    let mut form = widget::row![
        widget::toggler(config.keymap().is_some())
            .label("Set Keymap")
            .on_toggle(|t| {
                let keymap = if t { Some(String::from("us")) } else { None };
                BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSd(
                    config.clone().update_keymap(keymap),
                ))
            }),
        widget::horizontal_space()
    ];

    if let Some(keymap) = config.keymap() {
        let xc = config.clone();

        let keymap_box = widget::combo_box(keymaps, "Keymap", Some(&keymap.to_owned()), move |t| {
            BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSd(
                xc.clone().update_keymap(Some(t)),
            ))
        })
        .width(200);
        form = form.push(keymap_box);
    }

    widget::container(form)
        .padding(10)
        .style(widget::container::bordered_box)
}

fn hostname_form(
    config: &bb_imager::flasher::FlashingSdLinuxConfig,
) -> widget::Container<BBImagerMessage> {
    let mut form = widget::row![
        widget::toggler(config.hostname().is_some())
            .label("Set Hostname")
            .on_toggle(|t| {
                let hostname = if t {
                    whoami::fallible::hostname().ok()
                } else {
                    None
                };
                BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSd(
                    config.clone().update_hostname(hostname),
                ))
            }),
        widget::horizontal_space()
    ];

    if let Some(hostname) = config.hostname() {
        let xc = config.clone();

        let hostname_box = widget::text_input("beagle", hostname)
            .on_input(move |inp| {
                BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSd(
                    xc.clone().update_hostname(Some(inp)),
                ))
            })
            .width(200);
        form = form.push(hostname_box);
    }

    widget::container(form)
        .padding(10)
        .style(widget::container::bordered_box)
}

fn timezone_form<'a>(
    timezones: &'a widget::combo_box::State<String>,
    config: &'a bb_imager::flasher::FlashingSdLinuxConfig,
) -> widget::Container<'a, BBImagerMessage> {
    let mut form = widget::row![
        widget::toggler(config.timezone().is_some())
            .label("Set Timezone")
            .on_toggle(|t| {
                let tz = if t { helpers::system_timezone() } else { None };
                BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSd(
                    config.clone().update_timezone(tz.cloned()),
                ))
            }),
        widget::horizontal_space()
    ];

    if let Some(tz) = config.timezone() {
        let xc = config.clone();

        let timezone_box =
            widget::combo_box(timezones, "Timezone", Some(&tz.to_owned()), move |t| {
                BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSd(
                    xc.clone().update_timezone(Some(t)),
                ))
            })
            .width(200);
        form = form.push(timezone_box);
    }

    widget::container(form)
        .padding(10)
        .style(widget::container::bordered_box)
}

fn uname_pass_form(
    config: &bb_imager::flasher::FlashingSdLinuxConfig,
) -> widget::Container<BBImagerMessage> {
    let mut form = widget::column![
        widget::toggler(config.user().is_some())
            .label("Configure Username and Password")
            .on_toggle(|t| {
                let c = if t {
                    Some((whoami::username(), String::new()))
                } else {
                    None
                };
                BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSd(
                    config.clone().update_user(c),
                ))
            })
    ];

    if let Some((u, p)) = config.user() {
        form = form.extend([
            helpers::input_with_label("Username", "username", u, |inp| {
                FlashingCustomization::LinuxSd(
                    config.clone().update_user(Some((inp, p.to_owned()))),
                )
            })
            .into(),
            helpers::input_with_label("Password", "password", p, |inp| {
                FlashingCustomization::LinuxSd(
                    config.clone().update_user(Some((u.to_owned(), inp))),
                )
            })
            .into(),
        ]);
    }

    widget::container(form)
        .padding(10)
        .style(widget::container::bordered_box)
}

fn wifi_form(
    config: &bb_imager::flasher::FlashingSdLinuxConfig,
) -> widget::Container<BBImagerMessage> {
    let mut form = widget::column![
        widget::toggler(config.wifi().is_some())
            .label("Configure Wireless LAN")
            .on_toggle(|t| {
                let c = if t {
                    Some((String::new(), String::new()))
                } else {
                    None
                };
                BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSd(
                    config.clone().update_wifi(c),
                ))
            })
    ];

    if let Some((ssid, psk)) = config.wifi() {
        form = form.extend([
            helpers::input_with_label("SSID", "SSID", ssid, |inp| {
                FlashingCustomization::LinuxSd(
                    config.clone().update_wifi(Some((inp, psk.to_owned()))),
                )
            })
            .into(),
            helpers::input_with_label("Password", "password", psk, |inp| {
                FlashingCustomization::LinuxSd(
                    config.clone().update_wifi(Some((ssid.to_owned(), inp))),
                )
            })
            .into(),
        ]);
    }

    widget::container(form)
        .padding(10)
        .style(widget::container::bordered_box)
}

#[derive(Clone, Debug)]
pub enum FlashingCustomization {
    LinuxSdFormat,
    LinuxSd(bb_imager::flasher::FlashingSdLinuxConfig),
    Bcf(bb_imager::flasher::FlashingBcfConfig),
    Msp430,
    #[cfg(feature = "pb2_mspm0")]
    Pb2Mspm0 {
        persist_eeprom: bool,
    },
}

impl FlashingCustomization {
    pub fn new(flasher: bb_imager::Flasher, img: &helpers::BoardImage) -> Self {
        match flasher {
            bb_imager::Flasher::SdCard if img == &helpers::BoardImage::SdFormat => {
                Self::LinuxSdFormat
            }
            bb_imager::Flasher::SdCard => Self::LinuxSd(Default::default()),
            bb_imager::Flasher::BeagleConnectFreedom => Self::Bcf(Default::default()),
            bb_imager::Flasher::Msp430Usb => Self::Msp430,
            #[cfg(feature = "pb2_mspm0")]
            bb_imager::Flasher::Pb2Mspm0 => Self::Pb2Mspm0 {
                persist_eeprom: true,
            },
        }
    }
}
