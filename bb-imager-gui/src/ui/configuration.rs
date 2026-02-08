use iced::{
    Element,
    widget::{self, text},
};

use crate::{
    BBImagerMessage,
    helpers::{self, FlashingCustomization},
    persistance,
    ui::helpers::{element_with_element, element_with_label, page_type2},
};

const INPUT_WIDTH: u32 = 200;

pub(crate) fn view<'a>(state: &'a crate::CustomizeState) -> Element<'a, BBImagerMessage> {
    page_type2(
        &state.common,
        customization_pane(state),
        [
            widget::button("BACK")
                .on_press(BBImagerMessage::Back)
                .style(widget::button::secondary),
            widget::button("NEXT").on_press(BBImagerMessage::Next),
        ],
    )
}

fn customization_pane<'a>(state: &'a crate::CustomizeState) -> Element<'a, BBImagerMessage> {
    match state.customization() {
        FlashingCustomization::LinuxSdSysconfig(inner) => linux_sd_card(state, inner),
        FlashingCustomization::Bcf(inner) => bcf(inner),
        _ => panic!("No customization"),
    }
}

fn bcf<'a>(state: &'a persistance::BcfCustomization) -> Element<'a, BBImagerMessage> {
    widget::toggler(!state.verify)
        .label("Skip Verification")
        .on_toggle(|x| {
            BBImagerMessage::UpdateFlashConfig(FlashingCustomization::Bcf(
                state.clone().update_verify(!x),
            ))
        })
        .into()
}

fn linux_sd_card<'a>(
    state: &'a crate::CustomizeState,
    config: &'a persistance::SdSysconfCustomization,
) -> Element<'a, BBImagerMessage> {
    let col = widget::column([]);

    // Username and Password
    let col = col.push(
        widget::toggler(config.user.is_some())
            .label("Configure Username and Password")
            .on_toggle(|t| {
                let c = if t { Some(Default::default()) } else { None };
                BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSdSysconfig(
                    config.clone().update_user(c),
                ))
            }),
    );
    let col = match config.user.as_ref() {
        Some(usr) => col.extend([
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
        ]),
        None => col,
    };

    let col = col.push(widget::rule::horizontal(2));

    // Wifi
    let col = col.push(
        widget::toggler(config.wifi.is_some())
            .label("Configure Wireless LAN")
            .on_toggle(|t| {
                let c = if t { Some(Default::default()) } else { None };
                BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSdSysconfig(
                    config.clone().update_wifi(c),
                ))
            }),
    );
    let col = match config.wifi.as_ref() {
        Some(wifi) => col.extend([
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
        ]),
        None => col,
    };

    let col = col.push(widget::rule::horizontal(2));

    // Timezone
    let toggle = widget::toggler(config.timezone.is_some())
        .label("Set Timezone")
        .on_toggle(|t| {
            let tz = if t { helpers::system_timezone() } else { None };
            BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSdSysconfig(
                config.clone().update_timezone(tz.cloned()),
            ))
        });
    let col = match config.timezone.as_ref() {
        Some(tz) => {
            let xc = config.clone();
            col.push(element_with_element(
                toggle.into(),
                widget::combo_box(
                    state.timezones(),
                    "Timezone",
                    Some(&tz.to_owned()),
                    move |t| {
                        BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSdSysconfig(
                            xc.clone().update_timezone(Some(t)),
                        ))
                    },
                )
                .width(INPUT_WIDTH)
                .into(),
            ))
        }
        None => col.push(toggle),
    };

    let col = col.push(widget::rule::horizontal(2));

    // Hostname
    let toggle = widget::toggler(config.hostname.is_some())
        .label("Set Hostname")
        .on_toggle(|t| {
            let hostname = if t { whoami::hostname().ok() } else { None };
            BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSdSysconfig(
                config.clone().update_hostname(hostname),
            ))
        });
    let col = match config.hostname.as_ref() {
        Some(hostname) => col.push(element_with_element(
            toggle.into(),
            widget::text_input("beagle", hostname)
                .on_input(|inp| {
                    BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSdSysconfig(
                        config.clone().update_hostname(Some(inp)),
                    ))
                })
                .width(INPUT_WIDTH)
                .into(),
        )),
        None => col.push(toggle),
    };

    let col = col.push(widget::rule::horizontal(2));

    // Keymap
    let toggle = widget::toggler(config.keymap.is_some())
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
        });
    let col = match config.keymap.as_ref() {
        Some(keymap) => {
            let xc = config.clone();

            col.push(element_with_element(
                toggle.into(),
                widget::combo_box(
                    state.keymaps(),
                    "Keymap",
                    Some(&keymap.to_owned()),
                    move |t| {
                        BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSdSysconfig(
                            xc.clone().update_keymap(Some(t)),
                        ))
                    },
                )
                .width(INPUT_WIDTH)
                .into(),
            ))
        }
        None => col.push(toggle),
    };

    let col = col.push(widget::rule::horizontal(2));

    // SSH Key
    let col = col.extend([
        text("SSH authorization public key").into(),
        widget::center(
            widget::text_input("authorized key", config.ssh.as_deref().unwrap_or("")).on_input(
                |x| {
                    BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSdSysconfig(
                        config
                            .clone()
                            .update_ssh(if x.is_empty() { None } else { Some(x) }),
                    ))
                },
            ),
        )
        .padding(iced::Padding::ZERO.horizontal(16))
        .into(),
    ]);

    let col = col.push(widget::rule::horizontal(2));

    // Enable USB DHCP
    let col = col.push(
        widget::toggler(config.usb_enable_dhcp == Some(true))
            .label("Enable USB DHCP")
            .on_toggle(|x| {
                BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSdSysconfig(
                    config.clone().update_usb_enable_dhcp(Some(x)),
                ))
            }),
    );

    widget::scrollable(col.spacing(16)).into()
}

fn input_with_label<'a, F>(
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
                    t.border = t.border.color(theme.palette().danger);
                    t
                } else {
                    t
                }
            })
            .width(INPUT_WIDTH)
            .into(),
    )
}
