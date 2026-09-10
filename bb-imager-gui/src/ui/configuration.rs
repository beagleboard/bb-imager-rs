use iced::{
    Element,
    widget::{self, text},
};

use crate::{
    BBImagerMessage,
    helpers::{self, FlashingCustomization},
    persistance,
    ui::helpers::{
        action_button, detail_pane, element_with_element, element_with_label, focusable_toggler,
        page_type2,
    },
};

const INPUT_WIDTH: u32 = 200;

pub(crate) fn view<'a>(state: &'a crate::state::CustomizeState) -> Element<'a, BBImagerMessage> {
    page_type2(
        customization_pane(state),
        [
            action_button("RESET")
                .style(widget::button::danger)
                .on_press(BBImagerMessage::ResetFlashingConfig),
            action_button("BACK")
                .on_press(BBImagerMessage::Back)
                .style(widget::button::secondary),
            action_button("NEXT").on_press_maybe(if state.ctx.customization.validate() {
                Some(BBImagerMessage::Next)
            } else {
                None
            }),
        ],
    )
}

fn customization_pane<'a>(state: &'a crate::state::CustomizeState) -> Element<'a, BBImagerMessage> {
    match &state.ctx.customization {
        FlashingCustomization::LinuxSdSysconfig(inner) => linux_sd_card_sysconfig(state, inner),
        FlashingCustomization::LinuxSdCloudInit(inner) => linux_sd_card_cloudinit(state, inner),
        _ => panic!("No customization"),
    }
}

fn linux_sd_card_common<'a>(
    state: &'a crate::state::CustomizeState,
    config: &'a persistance::SdSysconfCustomization,
    wrap: impl Fn(persistance::SdSysconfCustomization) -> FlashingCustomization + Copy + 'static,
) -> widget::Column<'a, BBImagerMessage> {
    let mut col = widget::column([]);

    // Username and Password
    col = col.push(focusable_toggler(
        config.user.is_some(),
        "Configure Username and Password",
        move |t| {
            let c = if t { Some(Default::default()) } else { None };
            BBImagerMessage::UpdateFlashConfig(wrap(config.clone().update_user(c)))
        },
    ));
    if let Some(usr) = config.user.as_ref() {
        col = col.extend([
            input_with_label(
                "Username",
                "username",
                &usr.username,
                move |inp| {
                    wrap(
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
                move |inp| {
                    wrap(
                        config
                            .clone()
                            .update_user(Some(usr.clone().update_password(inp))),
                    )
                },
                false,
            )
            .into(),
        ])
    }

    col = col.push(widget::rule::horizontal(2));

    // Wifi
    col = col.push(focusable_toggler(
        config.wifi.is_some(),
        "Configure Wireless LAN",
        move |t| {
            let c = if t { Some(Default::default()) } else { None };
            BBImagerMessage::UpdateFlashConfig(wrap(config.clone().update_wifi(c)))
        },
    ));
    if let Some(wifi) = config.wifi.as_ref() {
        col = col.extend([
            input_with_label(
                "SSID",
                "SSID",
                &wifi.ssid,
                move |inp| {
                    wrap(
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
                move |inp| {
                    wrap(
                        config
                            .clone()
                            .update_wifi(Some(wifi.clone().update_password(inp))),
                    )
                },
                false,
            )
            .into(),
        ])
    };

    col = col.push(widget::rule::horizontal(2));

    // Timezone
    let toggle = focusable_toggler(config.timezone.is_some(), "Set Timezone", move |t| {
        let tz = if t { helpers::system_timezone() } else { None };
        BBImagerMessage::UpdateFlashConfig(wrap(config.clone().update_timezone(tz)))
    });
    col = match config.timezone.as_ref() {
        Some(tz) => {
            let xc = config.clone();
            // The configuration stores the zone as a string, so it has to be resolved
            // back to a `Tz` for the combo box to show it as the current selection.
            col.push(element_with_element(
                toggle,
                widget::combo_box(&state.common.timezones, "Timezone", Some(tz), move |t| {
                    BBImagerMessage::UpdateFlashConfig(wrap(xc.clone().update_timezone(Some(t))))
                })
                .width(INPUT_WIDTH)
                .into(),
            ))
        }
        None => col.push(toggle),
    };

    col = col.push(widget::rule::horizontal(2));

    // Hostname
    let toggle = focusable_toggler(config.hostname.is_some(), "Set Hostname", move |t| {
        let hostname = if t { Some(String::new()) } else { None };
        BBImagerMessage::UpdateFlashConfig(wrap(config.clone().update_hostname(hostname)))
    });
    col = match config.hostname.as_ref() {
        Some(hostname) => col.push(element_with_element(
            toggle,
            widget::text_input("beagle", hostname)
                .on_input(move |inp| {
                    BBImagerMessage::UpdateFlashConfig(wrap(
                        config.clone().update_hostname(Some(inp)),
                    ))
                })
                .width(INPUT_WIDTH)
                .into(),
        )),
        None => col.push(toggle),
    };

    col = col.push(widget::rule::horizontal(2));

    // Keymap
    let toggle = focusable_toggler(config.keymap.is_some(), "Set Keymap", move |t| {
        let keymap = if t {
            Some(helpers::system_keymap().to_string())
        } else {
            None
        };
        BBImagerMessage::UpdateFlashConfig(wrap(config.clone().update_keymap(keymap)))
    });
    col = match config.keymap.as_ref() {
        Some(keymap) => {
            let xc = config.clone();
            // The stored keymap is a `String`; the combo box options are the
            // `&'static str`s from `KEYMAP_LAYOUTS`, so the selection has to be
            // resolved back through the table.
            let selection = crate::constants::keymap_layout(keymap);

            col.push(element_with_element(
                toggle,
                widget::combo_box(
                    &state.common.keymaps,
                    "Keymap",
                    selection.as_ref(),
                    move |t| {
                        BBImagerMessage::UpdateFlashConfig(wrap(
                            xc.clone().update_keymap(Some(t.to_string())),
                        ))
                    },
                )
                .width(INPUT_WIDTH)
                .into(),
            ))
        }
        None => col.push(toggle),
    };

    col = col.push(widget::rule::horizontal(2));

    // SSH Key
    col.extend([
        text("SSH authorization public key").into(),
        widget::center(
            widget::text_input("authorized key", config.ssh.as_deref().unwrap_or("")).on_input(
                move |x| {
                    BBImagerMessage::UpdateFlashConfig(wrap(
                        config
                            .clone()
                            .update_ssh(if x.is_empty() { None } else { Some(x) }),
                    ))
                },
            ),
        )
        .padding(iced::Padding::ZERO.horizontal(16))
        .into(),
    ])
}

fn linux_sd_card_cloudinit<'a>(
    state: &'a crate::state::CustomizeState,
    config: &'a persistance::SdSysconfCustomization,
) -> Element<'a, BBImagerMessage> {
    let col = linux_sd_card_common(state, config, FlashingCustomization::LinuxSdCloudInit);
    detail_pane(col, &state.common.scroll_id)
}

fn linux_sd_card_sysconfig<'a>(
    state: &'a crate::state::CustomizeState,
    config: &'a persistance::SdSysconfCustomization,
) -> Element<'a, BBImagerMessage> {
    let mut col = linux_sd_card_common(state, config, FlashingCustomization::LinuxSdSysconfig);

    col = col.push(widget::rule::horizontal(2));
    // Enable USB DHCP
    col = col.push(focusable_toggler(
        config.usb_enable_dhcp == Some(true),
        "Enable USB DHCP",
        |x| {
            BBImagerMessage::UpdateFlashConfig(FlashingCustomization::LinuxSdSysconfig(
                config.clone().update_usb_enable_dhcp(Some(x)),
            ))
        },
    ));

    detail_pane(col, &state.common.scroll_id)
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
