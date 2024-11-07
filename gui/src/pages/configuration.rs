use iced::{
    widget::{self, text},
    Element,
};

use crate::{
    helpers::{self, home_btn},
    BBImagerMessage,
};

use super::Screen;

pub fn view<'a>(
    flashing_config: Option<&'a bb_imager::FlashingConfig>,
    timezones: &'a widget::combo_box::State<String>,
    keymaps: &'a widget::combo_box::State<String>,
) -> Element<'a, BBImagerMessage> {
    widget::responsive(move |size| {
        let action_btn_row = widget::row![
            home_btn("BACK", true, iced::Length::Fill)
                .style(widget::button::secondary)
                .width(iced::Length::FillPortion(1))
                .on_press(BBImagerMessage::SwitchScreen(Screen::Home)),
            widget::horizontal_space().width(iced::Length::FillPortion(5)),
            home_btn("WRITE", true, iced::Length::Fill)
                .style(widget::button::secondary)
                .width(iced::Length::FillPortion(1))
                .on_press(BBImagerMessage::StartFlashing)
        ]
        .padding(32)
        .width(iced::Length::Fill);

        let form = match flashing_config.expect("Missing flashing config") {
            bb_imager::FlashingConfig::LinuxSd(None) => widget::column([]),
            bb_imager::FlashingConfig::LinuxSd(Some(x)) => linux_sd_form(timezones, keymaps, x),
            bb_imager::FlashingConfig::Bcf(x) => widget::column![widget::toggler(!x.verify)
                .label("Skip Verification")
                .on_toggle(|y| {
                    BBImagerMessage::UpdateFlashConfig(bb_imager::FlashingConfig::Bcf(
                        x.clone().update_verify(!y),
                    ))
                })],
            bb_imager::FlashingConfig::Msp430 => widget::column([]),
        }
        .spacing(5);

        widget::column![
            text("Extra Configuration").size(28),
            widget::horizontal_rule(2),
            widget::scrollable(form).height(size.height - 210.0),
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
    config: &'a bb_imager::FlashingSdLinuxConfig,
) -> widget::Column<'a, BBImagerMessage> {
    let xc = config.clone();
    let timezone_box =
        widget::combo_box(timezones, "Timezone", config.timezone.as_ref(), move |t| {
            let tz = if t.is_empty() { None } else { Some(t) };
            BBImagerMessage::UpdateFlashConfig(bb_imager::FlashingConfig::LinuxSd(Some(
                xc.clone().update_timezone(tz),
            )))
        })
        .width(200);

    let xc = config.clone();
    let keymap_box = widget::combo_box(keymaps, "Keymap", config.keymap.as_ref(), move |t| {
        let tz = if t.is_empty() { None } else { Some(t) };
        BBImagerMessage::UpdateFlashConfig(bb_imager::FlashingConfig::LinuxSd(Some(
            xc.clone().update_keymap(tz),
        )))
    })
    .width(200);

    widget::column![
        widget::container(
            widget::toggler(!config.verify)
                .label("Skip Verification")
                .on_toggle(|y| {
                    BBImagerMessage::UpdateFlashConfig(bb_imager::FlashingConfig::LinuxSd(Some(
                        config.clone().update_verify(!y),
                    )))
                })
        )
        .padding(10)
        .width(iced::Length::Fill)
        .style(widget::container::bordered_box),
        widget::container(helpers::input_with_label(
            "Set Hostname",
            "beagle",
            config.hostname.as_deref().unwrap_or_default(),
            |inp| {
                let h = if inp.is_empty() { None } else { Some(inp) };
                bb_imager::FlashingConfig::LinuxSd(Some(config.clone().update_hostname(h)))
            }
        ))
        .style(widget::container::bordered_box),
        widget::container(helpers::element_with_label(
            "Set Timezone",
            timezone_box.into()
        ))
        .style(widget::container::bordered_box),
        widget::container(helpers::element_with_label("Set Keymap", keymap_box.into()))
            .style(widget::container::bordered_box),
        uname_pass_form(config).width(iced::Length::Fill),
        wifi_form(config).width(iced::Length::Fill)
    ]
}

fn uname_pass_form(
    config: &bb_imager::FlashingSdLinuxConfig,
) -> widget::Container<BBImagerMessage> {
    let mut form = widget::column![widget::toggler(config.user.is_some())
        .label("Configure Username and Password")
        .on_toggle(|t| {
            let c = if t {
                Some((String::new(), String::new()))
            } else {
                None
            };
            BBImagerMessage::UpdateFlashConfig(bb_imager::FlashingConfig::LinuxSd(Some(
                config.clone().update_user(c),
            )))
        })];

    if let Some((u, p)) = &config.user {
        form = form.extend([
            helpers::input_with_label("Username", "username", u, |inp| {
                bb_imager::FlashingConfig::LinuxSd(Some(
                    config.clone().update_user(Some((inp, p.clone()))),
                ))
            })
            .into(),
            helpers::input_with_label("Password", "password", p, |inp| {
                bb_imager::FlashingConfig::LinuxSd(Some(
                    config.clone().update_user(Some((u.clone(), inp))),
                ))
            })
            .into(),
        ]);
    }

    widget::container(form)
        .padding(10)
        .style(widget::container::bordered_box)
}

fn wifi_form(config: &bb_imager::FlashingSdLinuxConfig) -> widget::Container<BBImagerMessage> {
    let mut form = widget::column![widget::toggler(config.wifi.is_some())
        .label("Configure Wireless LAN")
        .on_toggle(|t| {
            let c = if t {
                Some((String::new(), String::new()))
            } else {
                None
            };
            BBImagerMessage::UpdateFlashConfig(bb_imager::FlashingConfig::LinuxSd(Some(
                config.clone().update_wifi(c),
            )))
        })];

    if let Some((ssid, psk)) = &config.wifi {
        form = form.extend([
            helpers::input_with_label("SSID", "SSID", ssid, |inp| {
                bb_imager::FlashingConfig::LinuxSd(Some(
                    config.clone().update_wifi(Some((inp, psk.clone()))),
                ))
            })
            .into(),
            helpers::input_with_label("Password", "password", psk, |inp| {
                bb_imager::FlashingConfig::LinuxSd(Some(
                    config.clone().update_wifi(Some((ssid.clone(), inp))),
                ))
            })
            .into(),
        ]);
    }

    widget::container(form)
        .padding(10)
        .style(widget::container::bordered_box)
}
