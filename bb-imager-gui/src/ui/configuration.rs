use iced::{
    Element,
    widget::{self, text},
};

use crate::{
    BBImagerMessage,
    helpers::{self, FlashingCustomization},
    pages::ConfigurationId,
    persistance::SdSysconfCustomization,
};

use super::helpers::home_btn_text;

pub(crate) fn view<'a>(
    app_settings: crate::persistance::AppSettings,
    customization: Option<&'a FlashingCustomization>,
    timezones: &'a widget::combo_box::State<String>,
    keymaps: &'a widget::combo_box::State<String>,
    page_id: ConfigurationId,
) -> Element<'a, BBImagerMessage> {
    widget::container(
        iced_aw::Tabs::new_with_tabs(
            [
                (
                    ConfigurationId::Customization,
                    iced_aw::TabLabel::Text(String::from("Customization")),
                    customization_page(customization, timezones, keymaps),
                ),
                (
                    ConfigurationId::Settings,
                    iced_aw::TabLabel::Text(String::from("Settings")),
                    global_settings(app_settings),
                ),
                (
                    ConfigurationId::About,
                    iced_aw::TabLabel::Text(String::from("About")),
                    about_page(),
                ),
            ],
            |x| BBImagerMessage::ReplaceScreen(crate::Screen::ExtraConfiguration(x)),
        )
        .set_active_tab(&page_id),
    )
    .padding(10)
    .height(iced::Length::Fill)
    .into()
}

fn about_page() -> Element<'static, BBImagerMessage> {
    widget::responsive(move |size| {
        const HEADER_FOOTER_HEIGHT: f32 = 90.0;

        let mid_el = widget::column![
            widget::image(widget::image::Handle::from_bytes(
                crate::constants::WINDOW_ICON
            )),
            crate::constants::APP_NAME,
            crate::constants::APP_RELEASE,
            crate::constants::APP_DESC,
            widget::button(text("Website").style(|_| widget::text::Style {
                color: Some(iced::Color::from_rgb(0.0, 0.0, 1.0))
            }))
            .style(widget::button::text)
            .on_press(BBImagerMessage::OpenUrl(
                crate::constants::APP_WEBSITE.into()
            )),
            widget::container(crate::constants::APP_LINCESE)
                .style(widget::container::bordered_box)
                .padding(10)
        ]
        .spacing(2)
        .align_x(iced::Center);

        widget::column![
            widget::vertical_space().height(2),
            widget::horizontal_rule(2),
            widget::scrollable(mid_el).height(size.height - HEADER_FOOTER_HEIGHT),
            widget::horizontal_rule(2),
            home_btn_text("BACK", true, iced::Length::Fill)
                .style(widget::button::secondary)
                .width(iced::Length::FillPortion(1))
                .on_press(BBImagerMessage::CancelCustomization),
        ]
        .spacing(10)
        .height(iced::Length::Fill)
        .width(iced::Length::Fill)
        .align_x(iced::Alignment::Center)
        .into()
    })
    .into()
}

fn global_settings(
    app_settings: crate::persistance::AppSettings,
) -> Element<'static, BBImagerMessage> {
    widget::responsive(move |size| {
        const HEADER_FOOTER_HEIGHT: f32 = 90.0;

        let log_file_p = helpers::log_file_path().to_string_lossy().to_string();
        let mid_el = widget::column![
            widget::container(
                widget::toggler(app_settings.skip_confirmation == Some(true))
                    .label("Disable confirmation dialog")
                    .on_toggle(move |t| BBImagerMessage::UpdateSettings(
                        app_settings.update_skip_confirmation(Some(t))
                    ))
            )
            .width(iced::Length::Fill)
            .padding(10)
            .style(widget::container::bordered_box),
            widget::container(element_with_label(
                "Log File",
                widget::text_input(&log_file_p, &log_file_p)
                    .on_input(|_| BBImagerMessage::Null)
                    .into()
            ))
            .style(widget::container::bordered_box)
        ]
        .spacing(5);

        widget::column![
            widget::vertical_space().height(2),
            widget::horizontal_rule(2),
            widget::scrollable(mid_el).height(size.height - HEADER_FOOTER_HEIGHT),
            widget::horizontal_rule(2),
            home_btn_text("BACK", true, iced::Length::Fill)
                .style(widget::button::secondary)
                .width(iced::Length::FillPortion(1))
                .on_press(BBImagerMessage::CancelCustomization),
        ]
        .spacing(10)
        .height(iced::Length::Fill)
        .width(iced::Length::Fill)
        .into()
    })
    .into()
}

fn customization_page<'a>(
    customization: Option<&'a FlashingCustomization>,
    timezones: &'a widget::combo_box::State<String>,
    keymaps: &'a widget::combo_box::State<String>,
) -> Element<'a, BBImagerMessage> {
    widget::responsive(move |size| {
        const HEADER_FOOTER_HEIGHT: f32 = 90.0;

        let action_btn_row = widget::row![
            home_btn_text("RESET", true, iced::Length::Fill)
                .style(widget::button::secondary)
                .width(iced::Length::FillPortion(1))
                .on_press(BBImagerMessage::ResetCustomization),
            widget::horizontal_space().width(iced::Length::FillPortion(3)),
            home_btn_text("ABORT", true, iced::Length::Fill)
                .style(widget::button::secondary)
                .width(iced::Length::FillPortion(1))
                .on_press(BBImagerMessage::CancelCustomization),
            widget::horizontal_space().width(iced::Length::FillPortion(3)),
            home_btn_text("SAVE", true, iced::Length::Fill)
                .style(widget::button::secondary)
                .width(iced::Length::FillPortion(1))
                .on_press_maybe({
                    match customization {
                        Some(customization) if customization.validate() => {
                            Some(BBImagerMessage::SaveCustomization)
                        }
                        _ => None,
                    }
                }),
        ]
        .padding(4)
        .width(iced::Length::Fill);

        let form = match customization {
            Some(customization) => match customization {
                FlashingCustomization::LinuxSdSysconfig(x) => linux_sd_form(timezones, keymaps, x),
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
                FlashingCustomization::Pb2Mspm0(x) => {
                    widget::column![
                        widget::toggler(x.persist_eeprom)
                            .label("Persist EEPROM")
                            .on_toggle(move |y| {
                                BBImagerMessage::UpdateFlashConfig(FlashingCustomization::Pb2Mspm0(
                                    x.clone().update_persist_eeprom(y),
                                ))
                            })
                    ]
                }
                _ => widget::column([]),
            },
            _ => widget::column([]),
        }
        .spacing(5);

        widget::column![
            widget::vertical_space().height(2),
            widget::horizontal_rule(2),
            widget::scrollable(form).height(size.height - HEADER_FOOTER_HEIGHT),
            widget::horizontal_rule(2),
            action_btn_row,
        ]
        .spacing(10)
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
    config: &'a SdSysconfCustomization,
) -> widget::Column<'a, BBImagerMessage> {
    widget::column![
        hostname_form(config).width(iced::Length::Fill),
        timezone_form(timezones, config).width(iced::Length::Fill),
        keymap_form(keymaps, config).width(iced::Length::Fill),
        uname_pass_form(config).width(iced::Length::Fill),
        wifi_form(config).width(iced::Length::Fill),
        ssh_form(config).width(iced::Length::Fill),
        usb_enable_dhcp_form(config)
    ]
}

fn usb_enable_dhcp_form(config: &SdSysconfCustomization) -> widget::Container<'_, BBImagerMessage> {
    let form = widget::toggler(config.usb_enable_dhcp == Some(true))
        .label("Enable USB DHCP")
        .on_toggle(|x| {
            BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSdSysconfig(
                config.clone().update_usb_enable_dhcp(Some(x)),
            ))
        })
        .width(iced::Length::Fill);

    widget::container(form)
        .padding(10)
        .style(widget::container::bordered_box)
}

fn ssh_form<'a>(config: &'a SdSysconfCustomization) -> widget::Container<'a, BBImagerMessage> {
    let form = widget::column![
        widget::text("SSH authorization public key"),
        widget::text_input("authorized key", config.ssh.as_deref().unwrap_or("")).on_input(|x| {
            BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSdSysconfig(
                config
                    .clone()
                    .update_ssh(if x.is_empty() { None } else { Some(x) }),
            ))
        })
    ]
    .spacing(6);

    widget::container(form)
        .padding(10)
        .style(widget::container::bordered_box)
}

fn keymap_form<'a>(
    keymaps: &'a widget::combo_box::State<String>,
    config: &'a SdSysconfCustomization,
) -> widget::Container<'a, BBImagerMessage> {
    let mut form = widget::row![
        widget::toggler(config.keymap.is_some())
            .label("Set Keymap")
            .on_toggle(|t| {
                let keymap = if t {
                    Some(helpers::system_keymap())
                } else {
                    None
                };
                BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSdSysconfig(
                    config.clone().update_keymap(keymap),
                ))
            }),
        widget::horizontal_space()
    ];

    if let Some(keymap) = &config.keymap {
        let xc = config.clone();

        let keymap_box = widget::combo_box(keymaps, "Keymap", Some(&keymap.to_owned()), move |t| {
            BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSdSysconfig(
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

fn hostname_form(config: &SdSysconfCustomization) -> widget::Container<'_, BBImagerMessage> {
    let mut form = widget::row![
        widget::toggler(config.hostname.is_some())
            .label("Set Hostname")
            .on_toggle(|t| {
                let hostname = if t {
                    whoami::fallible::hostname().ok()
                } else {
                    None
                };
                BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSdSysconfig(
                    config.clone().update_hostname(hostname),
                ))
            }),
        widget::horizontal_space()
    ];

    if let Some(hostname) = config.hostname.as_ref() {
        let xc = config.clone();

        let hostname_box = widget::text_input("beagle", hostname)
            .on_input(move |inp| {
                BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSdSysconfig(
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
    config: &'a SdSysconfCustomization,
) -> widget::Container<'a, BBImagerMessage> {
    let mut form = widget::row![
        widget::toggler(config.timezone.is_some())
            .label("Set Timezone")
            .on_toggle(|t| {
                let tz = if t { helpers::system_timezone() } else { None };
                BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSdSysconfig(
                    config.clone().update_timezone(tz.cloned()),
                ))
            }),
        widget::horizontal_space()
    ];

    if let Some(tz) = config.timezone.as_ref() {
        let xc = config.clone();

        let timezone_box =
            widget::combo_box(timezones, "Timezone", Some(&tz.to_owned()), move |t| {
                BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSdSysconfig(
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

fn uname_pass_form(config: &SdSysconfCustomization) -> widget::Container<'_, BBImagerMessage> {
    let mut form = widget::column![
        widget::toggler(config.user.is_some())
            .label("Configure Username and Password")
            .on_toggle(|t| {
                let c = if t { Some(Default::default()) } else { None };
                BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSdSysconfig(
                    config.clone().update_user(c),
                ))
            })
    ];

    if let Some(usr) = config.user.as_ref() {
        form = form.extend([
            input_with_label(
                "Username",
                "username",
                &usr.username,
                |inp| {
                    FlashingCustomization::LinuxSdSysconfig(
                        config
                            .clone()
                            .update_user(Some(usr.clone().update_username(inp))),
                    )
                },
                !usr.validate_username(),
            )
            .into(),
            input_with_label(
                "Password",
                "password",
                &usr.password,
                |inp| {
                    FlashingCustomization::LinuxSdSysconfig(
                        config
                            .clone()
                            .update_user(Some(usr.clone().update_password(inp))),
                    )
                },
                false,
            )
            .into(),
        ]);
    }

    widget::container(form)
        .padding(10)
        .style(widget::container::bordered_box)
}

fn wifi_form(config: &SdSysconfCustomization) -> widget::Container<'_, BBImagerMessage> {
    let mut form = widget::column![
        widget::toggler(config.wifi.is_some())
            .label("Configure Wireless LAN")
            .on_toggle(|t| {
                let c = if t { Some(Default::default()) } else { None };
                BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSdSysconfig(
                    config.clone().update_wifi(c),
                ))
            })
    ];

    if let Some(wifi) = config.wifi.as_ref() {
        form = form.extend([
            input_with_label(
                "SSID",
                "SSID",
                &wifi.ssid,
                |inp| {
                    FlashingCustomization::LinuxSdSysconfig(
                        config
                            .clone()
                            .update_wifi(Some(wifi.clone().update_ssid(inp))),
                    )
                },
                false,
            )
            .into(),
            input_with_label(
                "Password",
                "password",
                &wifi.password,
                |inp| {
                    FlashingCustomization::LinuxSdSysconfig(
                        config
                            .clone()
                            .update_wifi(Some(wifi.clone().update_password(inp))),
                    )
                },
                false,
            )
            .into(),
        ]);
    }

    widget::container(form)
        .padding(10)
        .style(widget::container::bordered_box)
}

pub(crate) fn input_with_label<'a, F>(
    label: &'static str,
    placeholder: &'static str,
    val: &'a str,
    update_config_cb: F,
    invalid_val: bool,
) -> widget::Row<'a, BBImagerMessage>
where
    F: 'a + Fn(String) -> FlashingCustomization,
{
    element_with_label(
        label,
        widget::text_input(placeholder, val)
            .on_input(move |inp| BBImagerMessage::UpdateFlashConfig(update_config_cb(inp)))
            .style(move |theme, status| {
                let mut t = widget::text_input::default(theme, status);

                if invalid_val {
                    t.border = t.border.color(iced::Color::from_rgb(1.0, 0.0, 0.0));
                    t
                } else {
                    t
                }
            })
            .width(200)
            .into(),
    )
}

pub(crate) fn element_with_label<'a>(
    label: &'static str,
    el: Element<'a, BBImagerMessage>,
) -> widget::Row<'a, BBImagerMessage> {
    widget::row![text(label), widget::horizontal_space(), el]
        .padding(10)
        .spacing(10)
        .align_y(iced::Alignment::Center)
}
