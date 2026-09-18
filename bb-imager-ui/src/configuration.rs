use std::sync::Arc;

use iced::{Element, widget};

use crate::Message;
use crate::helpers::{detail_pane, page_type2};

const INPUT_WIDTH: u32 = 200;

/// Customization shared by every Linux SD card image, regardless of the init
/// format writing it out.
#[derive(Default, Debug, Clone)]
pub struct CloudInit {
    pub hostname: Option<Arc<str>>,
    pub timezone: Option<chrono_tz::Tz>,
    pub keymap: Option<&'static str>,
    pub user: Option<(Arc<str>, Arc<str>)>,
    pub wifi: Option<(Arc<str>, Arc<str>)>,
    pub ssh: Arc<str>,
}

impl CloudInit {
    fn is_invalid(&self) -> bool {
        self.hostname.as_ref().is_some_and(|x| x.is_empty())
            || self
                .user
                .as_ref()
                .is_some_and(|(uname, pass)| is_uname_invalid(uname) || pass.is_empty())
            || self
                .wifi
                .as_ref()
                .is_some_and(|(ssid, pass)| ssid.is_empty() || pass.is_empty())
    }
}

/// [`CloudInit`], plus the extras only `sysconf` can express.
#[derive(Default, Debug, Clone)]
pub struct SysConfig {
    pub common: CloudInit,
    pub usb_enable_dhcp: bool,
}

#[derive(Debug, Clone)]
pub enum Customization {
    SysConfig(SysConfig),
    CloudInit(CloudInit),
}

impl Customization {
    fn is_invalid(&self) -> bool {
        match self {
            Self::SysConfig(x) => x.common.is_invalid(),
            Self::CloudInit(x) => x.is_invalid(),
        }
    }
}

#[derive(Debug)]
pub struct State {
    pub customization: Customization,
    /// Username filled in when the user toggle is switched on.
    pub default_username: &'static str,
    /// Timezone filled in when the timezone toggle is switched on.
    pub default_timezone: Option<chrono_tz::Tz>,
    /// Keymap filled in when the keymap toggle is switched on.
    pub default_keymap: &'static str,
    pub timezones: widget::combo_box::State<chrono_tz::Tz>,
    pub keymaps: widget::combo_box::State<&'static str>,
}

pub fn view(state: &State, scroll_id: widget::Id) -> Element<'_, Message> {
    page_type2(
        customization_pane(state, &scroll_id),
        [
            widget::button("RESET")
                .style(widget::button::danger)
                .on_press(Message::Reset),
            widget::button("BACK")
                .on_press(Message::Back)
                .style(widget::button::secondary),
            widget::button("NEXT").on_press_maybe(if state.customization.is_invalid() {
                None
            } else {
                Some(Message::Next)
            }),
        ],
    )
}

fn customization_pane<'a>(state: &'a State, scroll_id: &widget::Id) -> Element<'a, Message> {
    let col = match &state.customization {
        Customization::SysConfig(c) => sysconfig(state, c),
        Customization::CloudInit(c) => cloudinit(state, c),
    };

    detail_pane(col, scroll_id)
}

fn sysconfig<'a>(state: &'a State, c: &'a SysConfig) -> widget::Column<'a, Message> {
    widget::column(linux_sd_common(state, &c.common).into_iter().map(|x| {
        x.map(|common| {
            Message::UpdateCustomization(Customization::SysConfig(SysConfig {
                common,
                usb_enable_dhcp: c.usb_enable_dhcp,
            }))
        })
    }))
    .push(widget::rule::horizontal(2))
    .push(
        widget::toggler(c.usb_enable_dhcp)
            .label("Enable USB DHCP")
            .on_toggle(|usb_enable_dhcp| {
                Message::UpdateCustomization(Customization::SysConfig(SysConfig {
                    common: c.common.clone(),
                    usb_enable_dhcp,
                }))
            }),
    )
}

fn cloudinit<'a>(state: &'a State, c: &'a CloudInit) -> widget::Column<'a, Message> {
    widget::column(
        linux_sd_common(state, c)
            .into_iter()
            .map(|x| x.map(|y| Message::UpdateCustomization(Customization::CloudInit(y)))),
    )
}

/// The fields every Linux SD card image shares.
///
/// Each element speaks in whole [`CloudInit`]s so the two init formats only have
/// to wrap the result, rather than thread a constructor through every widget.
fn linux_sd_common<'a>(
    state: &'a State,
    c: &'a CloudInit,
) -> impl IntoIterator<Item = Element<'a, CloudInit>> {
    let default_timezone = state.default_timezone.unwrap_or_default();
    let default_username = state.default_username;
    let default_keymap = state.default_keymap;

    [
        elements_with_toggle(
            c.user.as_ref(),
            "Configure Username and Password",
            move |t| {
                let mut temp = c.clone();
                if t {
                    temp.user = Some((default_username.into(), "".into()));
                } else {
                    temp.user.take();
                }
                temp
            },
            |(uname, pass)| {
                [
                    input_with_label(
                        "Username",
                        "username",
                        uname,
                        |t| {
                            let mut temp = c.clone();
                            temp.user = temp.user.map(|(_, p)| (t.into(), p));
                            temp
                        },
                        is_uname_invalid(uname),
                    ),
                    input_with_label(
                        "Password",
                        "password",
                        pass,
                        |t| {
                            let mut temp = c.clone();
                            temp.user = temp.user.map(|(u, _)| (u, t.into()));
                            temp
                        },
                        pass.is_empty(),
                    ),
                ]
            },
        ),
        widget::rule::horizontal(2).into(),
        elements_with_toggle(
            c.wifi.as_ref(),
            "Configure Wireless LAN",
            move |t| {
                let mut temp = c.clone();
                if t {
                    temp.wifi = Some(("".into(), "".into()));
                } else {
                    temp.wifi.take();
                }
                temp
            },
            |(ssid, pass)| {
                [
                    input_with_label(
                        "SSID",
                        "SSID",
                        ssid,
                        |t| {
                            let mut temp = c.clone();
                            temp.wifi = temp.wifi.map(|(_, p)| (t.into(), p));
                            temp
                        },
                        ssid.is_empty(),
                    ),
                    input_with_label(
                        "Password",
                        "password",
                        pass,
                        |t| {
                            let mut temp = c.clone();
                            temp.wifi = temp.wifi.map(|(s, _)| (s, t.into()));
                            temp
                        },
                        pass.is_empty(),
                    ),
                ]
            },
        ),
        widget::rule::horizontal(2).into(),
        element_with_toggle(
            c.timezone.as_ref(),
            "Set Timezone",
            move |t| {
                let mut temp = c.clone();
                if t {
                    temp.timezone = Some(default_timezone);
                } else {
                    temp.timezone.take();
                }
                temp
            },
            |tz| {
                let config = c.clone();
                widget::combo_box(&state.timezones, "Timezone", Some(tz), move |t| {
                    let mut temp = config.clone();
                    temp.timezone = Some(t);
                    temp
                })
                .width(INPUT_WIDTH)
                .into()
            },
        ),
        widget::rule::horizontal(2).into(),
        element_with_toggle(
            c.hostname.as_ref(),
            "Set Hostname",
            move |t| {
                let mut temp = c.clone();
                if t {
                    temp.hostname = Some("".into());
                } else {
                    temp.hostname.take();
                }
                temp
            },
            |hostname| {
                text_input(
                    "beagle",
                    hostname,
                    |t| {
                        let mut temp = c.clone();
                        temp.hostname = Some(t.into());
                        temp
                    },
                    hostname.is_empty(),
                )
                .into()
            },
        ),
        widget::rule::horizontal(2).into(),
        element_with_toggle(
            c.keymap.as_ref(),
            "Set Keymap",
            move |t| {
                let mut temp = c.clone();
                if t {
                    temp.keymap = Some(default_keymap);
                } else {
                    temp.keymap.take();
                }
                temp
            },
            |keymap| {
                let config = c.clone();
                widget::combo_box(&state.keymaps, "Keymap", Some(keymap), move |t| {
                    let mut temp = config.clone();
                    temp.keymap = Some(t);
                    temp
                })
                .width(INPUT_WIDTH)
                .into()
            },
        ),
        widget::rule::horizontal(2).into(),
        widget::text("SSH authorization public key").into(),
        widget::center(widget::text_input("authorized key", &c.ssh).on_input(|t| {
            let mut temp = c.clone();
            temp.ssh = t.into();
            temp
        }))
        .padding(iced::Padding::ZERO.horizontal(16))
        .into(),
    ]
}

/// A toggle, plus the single widget configuring it once it is on.
fn element_with_toggle<'a, T>(
    val: Option<&'a T>,
    label: &'static str,
    toggle_cb: impl Fn(bool) -> CloudInit + 'a,
    el: impl FnOnce(&'a T) -> Element<'a, CloudInit>,
) -> Element<'a, CloudInit> {
    let toggler = widget::toggler(val.is_some())
        .label(label)
        .on_toggle(toggle_cb);

    match val {
        Some(val) => widget::row![toggler, widget::space::horizontal(), el(val)]
            .align_y(iced::Alignment::Center)
            .padding(iced::Padding::ZERO.right(16))
            .into(),
        None => toggler.into(),
    }
}

/// A toggle, plus the several widgets configuring it once it is on.
fn elements_with_toggle<'a, T, I>(
    val: Option<&'a T>,
    label: &'static str,
    toggle_cb: impl Fn(bool) -> CloudInit + 'a,
    el: impl FnOnce(&'a T) -> I,
) -> Element<'a, CloudInit>
where
    I: IntoIterator<Item = Element<'a, CloudInit>>,
{
    let toggler = widget::toggler(val.is_some())
        .label(label)
        .on_toggle(toggle_cb);

    match val {
        Some(val) => widget::column([toggler.into()].into_iter().chain(el(val)))
            .spacing(16)
            .into(),
        None => toggler.into(),
    }
}

fn input_with_label<'a>(
    label: &'static str,
    placeholder: &'static str,
    value: &'a str,
    cb: impl Fn(String) -> CloudInit + 'a,
    invalid_val: bool,
) -> Element<'a, CloudInit> {
    widget::row![
        widget::text(label),
        widget::space::horizontal(),
        text_input(placeholder, value, cb, invalid_val)
    ]
    .align_y(iced::Alignment::Center)
    .padding(iced::Padding::ZERO.horizontal(16))
    .into()
}

fn text_input<'a>(
    placeholder: &'static str,
    value: &'a str,
    cb: impl Fn(String) -> CloudInit + 'a,
    invalid_val: bool,
) -> widget::TextInput<'a, CloudInit> {
    widget::text_input(placeholder, value)
        .on_input(cb)
        .width(INPUT_WIDTH)
        .style(move |theme, status| {
            let mut t = widget::text_input::default(theme, status);

            if invalid_val {
                t.border = t.border.color(theme.palette().danger);
            }

            t
        })
}

fn is_uname_invalid(uname: &str) -> bool {
    uname == "root"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_customization_is_valid() {
        assert!(!Customization::CloudInit(Default::default()).is_invalid());
        assert!(!Customization::SysConfig(Default::default()).is_invalid());
    }

    #[test]
    fn root_username_is_invalid() {
        let c = CloudInit {
            user: Some(("root".into(), "pw".into())),
            ..Default::default()
        };
        assert!(c.is_invalid());
    }

    #[test]
    fn empty_required_fields_are_invalid() {
        let empty_hostname = CloudInit {
            hostname: Some("".into()),
            ..Default::default()
        };
        assert!(empty_hostname.is_invalid());

        let empty_password = CloudInit {
            user: Some(("beagle".into(), "".into())),
            ..Default::default()
        };
        assert!(empty_password.is_invalid());

        let empty_ssid = CloudInit {
            wifi: Some(("".into(), "pw".into())),
            ..Default::default()
        };
        assert!(empty_ssid.is_invalid());

        let empty_wifi_password = CloudInit {
            wifi: Some(("net".into(), "".into())),
            ..Default::default()
        };
        assert!(empty_wifi_password.is_invalid());
    }

    #[test]
    fn fully_configured_customization_is_valid() {
        let c = CloudInit {
            hostname: Some("beagle".into()),
            timezone: Some(chrono_tz::Tz::UTC),
            keymap: Some("us"),
            user: Some(("beagle".into(), "pw".into())),
            wifi: Some(("net".into(), "pw".into())),
            ssh: "".into(),
        };
        assert!(!c.is_invalid());
    }

    /// An unset SSH key is the empty string, so it must not gate NEXT.
    #[test]
    fn empty_ssh_key_is_valid() {
        assert!(!CloudInit::default().is_invalid());
    }
}
