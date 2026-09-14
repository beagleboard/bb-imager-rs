use bb_config::config::InitFormat;
use iced::widget;

use crate::Message;
use crate::helpers::page_layout;

const INPUT_WIDTH: u32 = 200;

const SD_SELECTIONS: &[InitFormat] =
    &[InitFormat::None, InitFormat::Sysconf, InitFormat::CloudInit];

#[derive(Default, Debug, Clone)]
pub struct CloudInit {
    pub hostname: Option<std::sync::Arc<str>>,
    pub timezone: Option<chrono_tz::Tz>,
    pub keymap: Option<&'static str>,
    pub user: Option<(std::sync::Arc<str>, std::sync::Arc<str>)>,
    pub wifi: Option<(std::sync::Arc<str>, std::sync::Arc<str>)>,
    pub ssh: std::sync::Arc<str>,
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

#[derive(Default, Debug, Clone)]
pub struct SysConfig {
    pub common: CloudInit,
    pub usb_enable_dhcp: bool,
}

#[derive(Debug, Clone, Default)]
pub enum SelectableSd {
    #[default]
    None,
    SysConfig(SysConfig),
    CloudInit(CloudInit),
}

impl SelectableSd {
    fn init_format(&self) -> InitFormat {
        match self {
            SelectableSd::None => InitFormat::None,
            SelectableSd::SysConfig(_) => InitFormat::Sysconf,
            SelectableSd::CloudInit(_) => InitFormat::CloudInit,
        }
    }
}

/// TODO: Add SelectableSd
#[derive(Debug, Clone)]
pub enum Customization {
    SelectableSd(SelectableSd),
    SysConfig(SysConfig),
    CloudInit(CloudInit),
}

impl Customization {
    fn is_invalid(&self) -> bool {
        match self {
            Customization::SysConfig(x)
            | Customization::SelectableSd(SelectableSd::SysConfig(x)) => x.common.is_invalid(),
            Customization::CloudInit(x)
            | Customization::SelectableSd(SelectableSd::CloudInit(x)) => x.is_invalid(),
            Customization::SelectableSd(SelectableSd::None) => false,
        }
    }
}

#[derive(Debug)]
pub struct State {
    pub customization: Customization,
    pub default_username: &'static str,
    pub default_timezone: Option<chrono_tz::Tz>,
    pub default_keymap: &'static str,
    pub timezones: widget::combo_box::State<chrono_tz::Tz>,
    pub keymaps: widget::combo_box::State<&'static str>,
}

pub fn view<'a, D: Clone + 'a>(
    s: &'a State,
    scroll_id: widget::Id,
) -> iced::Element<'a, Message<D>> {
    let mut layout = widget::column![];

    let temp = match &s.customization {
        Customization::SysConfig(c) => sysconfig(
            c,
            s.default_username,
            s.default_timezone.unwrap_or_default(),
            s.default_keymap,
            &s.timezones,
            &s.keymaps,
        ),
        Customization::CloudInit(c) => cloudinit(
            c,
            s.default_username,
            s.default_timezone.unwrap_or_default(),
            s.default_keymap,
            &s.timezones,
            &s.keymaps,
        ),
        Customization::SelectableSd(x) => {
            let init_selection = widget::row![
                "Init Format",
                widget::space::horizontal(),
                widget::pick_list(
                    SD_SELECTIONS,
                    Some(x.init_format()),
                    Message::SelectInitFormat,
                )
            ]
            .padding(iced::Padding::ZERO.horizontal(16));

            layout = layout.extend([init_selection.into(), widget::rule::horizontal(2).into()]);

            match x {
                SelectableSd::None => widget::column![],
                SelectableSd::SysConfig(c) => sysconfig(
                    c,
                    s.default_username,
                    s.default_timezone.unwrap_or_default(),
                    s.default_keymap,
                    &s.timezones,
                    &s.keymaps,
                )
                .spacing(16),
                SelectableSd::CloudInit(c) => cloudinit(
                    c,
                    s.default_username,
                    s.default_timezone.unwrap_or_default(),
                    s.default_keymap,
                    &s.timezones,
                    &s.keymaps,
                )
                .spacing(16),
            }
        }
    };

    page_layout(
        (
            [
                ("Device", false, Some(Message::GotoDevicePage)),
                ("Software", false, Some(Message::GotoSoftwarePage)),
                ("Destination", false, Some(Message::GotoDestinationPage)),
                ("Customization", true, None),
            ],
            [("App Options", false, Some(Message::GotoAppOptions))],
        ),
        layout
            .extend([
                widget::scrollable(temp.spacing(16).padding(iced::Padding::ZERO.horizontal(16)))
                    .id(scroll_id)
                    .height(iced::Fill)
                    .into(),
                widget::rule::horizontal(2).into(),
                widget::row![
                    widget::button("RESET")
                        .style(widget::button::secondary)
                        .on_press(Message::Reset),
                    widget::space::horizontal(),
                    widget::button("NEXT").on_press_maybe(if s.customization.is_invalid() {
                        None
                    } else {
                        Some(Message::Next)
                    },)
                ]
                .padding(iced::Padding::ZERO.horizontal(16))
                .into(),
            ])
            .padding(iced::Padding::ZERO.vertical(16))
            .spacing(16),
    )
}

fn sysconfig<'a, D: 'a + Clone>(
    c: &'a SysConfig,
    default_user: &'static str,
    default_timezone: chrono_tz::Tz,
    default_keymap: &'static str,
    timezone_state: &'a widget::combo_box::State<chrono_tz::Tz>,
    keymap_state: &'a widget::combo_box::State<&'static str>,
) -> widget::Column<'a, Message<D>> {
    widget::column(
        linux_sd_common(
            &c.common,
            default_user,
            default_timezone,
            default_keymap,
            timezone_state,
            keymap_state,
        )
        .into_iter()
        .map(|x| {
            x.map(|y| {
                let mut temp = c.clone();
                temp.common = y;
                Message::UpdateCustomizaton(Customization::SysConfig(temp))
            })
        }),
    )
    .push(widget::rule::horizontal(2))
    .push(
        widget::toggler(c.usb_enable_dhcp)
            .label("Enable USB DHCP")
            .on_toggle(|t| {
                let mut temp = c.clone();
                temp.usb_enable_dhcp = t;
                Message::UpdateCustomizaton(Customization::SysConfig(temp))
            }),
    )
}

fn cloudinit<'a, D: 'a + Clone>(
    c: &'a CloudInit,
    default_user: &'static str,
    default_timezone: chrono_tz::Tz,
    default_keymap: &'static str,
    timezone_state: &'a widget::combo_box::State<chrono_tz::Tz>,
    keymap_state: &'a widget::combo_box::State<&'static str>,
) -> widget::Column<'a, Message<D>> {
    widget::column(
        linux_sd_common(
            c,
            default_user,
            default_timezone,
            default_keymap,
            timezone_state,
            keymap_state,
        )
        .into_iter()
        .map(|x| x.map(|y| Message::UpdateCustomizaton(Customization::CloudInit(y)))),
    )
}

fn linux_sd_common<'a>(
    c: &'a CloudInit,
    default_user: &'static str,
    default_timezone: chrono_tz::Tz,
    default_keymap: &'static str,
    timezone_state: &'a widget::combo_box::State<chrono_tz::Tz>,
    keymap_state: &'a widget::combo_box::State<&'static str>,
) -> impl IntoIterator<Item = iced::Element<'a, CloudInit>> {
    [
        elements_with_toggle(
            c.user.as_ref(),
            "Configure Username and Password",
            |t| {
                let mut temp = c.clone();
                if t {
                    temp.user = Some((default_user.into(), "".into()));
                } else {
                    temp.user.take();
                }
                temp
            },
            |(uname, pass)| {
                [
                    input_with_label(
                        "Username",
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
            |t| {
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
                        pass,
                        |t| {
                            let mut temp = c.clone();
                            temp.wifi = temp.wifi.map(|(u, _)| (u, t.into()));
                            temp
                        },
                        pass.is_empty(),
                    ),
                ]
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
            move |s| {
                text_input(
                    "Hostname",
                    s,
                    move |t| {
                        let mut temp = c.clone();
                        temp.hostname = Some(t.into());
                        temp
                    },
                    s.is_empty(),
                )
                .into()
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
            |s| {
                let temp = c.clone();
                widget::combo_box(timezone_state, "Timezone", Some(s), move |t| {
                    let mut temp = temp.clone();
                    temp.timezone = Some(t);
                    temp
                })
                .width(INPUT_WIDTH)
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
            |s| {
                let temp = c.clone();
                widget::combo_box(keymap_state, "Keymap", Some(s), move |t| {
                    let mut temp = temp.clone();
                    temp.keymap = Some(t);
                    temp
                })
                .width(INPUT_WIDTH)
                .into()
            },
        ),
        widget::rule::horizontal(2).into(),
        "SSH Authorization Public Key".into(),
        widget::text_input("Authorized Key", &c.ssh)
            .on_input(move |x| {
                let mut temp = c.clone();
                temp.ssh = x.into();
                temp
            })
            .into(),
    ]
}

fn element_with_toggle<'a, D: 'a + Clone, T>(
    state: Option<&'a T>,
    label: &'static str,
    toggle_cb: impl Fn(bool) -> D + 'a,
    el: impl FnOnce(&'a T) -> iced::Element<'a, D>,
) -> iced::Element<'a, D> {
    let hostname_toggler = widget::toggler(state.is_some())
        .label(label)
        .on_toggle(toggle_cb);
    if let Some(val) = state {
        widget::row![hostname_toggler, widget::space::horizontal(), el(val)]
            .align_y(iced::Center)
            .into()
    } else {
        hostname_toggler.into()
    }
}

fn elements_with_toggle<'a, D: 'a + Clone, T, I>(
    state: Option<&'a T>,
    label: &'static str,
    toggle_cb: impl Fn(bool) -> D + 'a,
    el: impl FnOnce(&'a T) -> I,
) -> iced::Element<'a, D>
where
    I: IntoIterator<Item = iced::Element<'a, D>>,
{
    let toggler = widget::toggler(state.is_some())
        .label(label)
        .on_toggle(toggle_cb);
    if let Some(val) = state {
        widget::column([toggler.into()].into_iter().chain(el(val)))
            .spacing(12)
            .into()
    } else {
        toggler.into()
    }
}

fn input_with_label<'a, D: 'a + Clone>(
    label: &'static str,
    value: &'a str,
    cb: impl Fn(String) -> D + 'a,
    invalid_val: bool,
) -> iced::Element<'a, D> {
    widget::row![
        label,
        widget::space::horizontal(),
        text_input(label, value, cb, invalid_val)
    ]
    .into()
}

fn text_input<'a, D: 'a + Clone>(
    label: &'static str,
    value: &'a str,
    cb: impl Fn(String) -> D + 'a,
    invalid_val: bool,
) -> widget::TextInput<'a, D> {
    widget::text_input(label, value)
        .on_input(cb)
        .width(INPUT_WIDTH)
        .style(move |theme, status| {
            let mut t = widget::text_input::default(theme, status);

            if invalid_val {
                t.border = t.border.color(theme.palette().danger);
                t
            } else {
                t
            }
        })
}

fn is_uname_invalid(uname: &str) -> bool {
    uname == "root"
}
