//! This module contains persistance for configuration

use std::io::{Read, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Configuration for GUI that should be presisted
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GuiConfiguration {
    #[serde(default)]
    pub(crate) sd_customization: SdCustomization,
}

impl From<&GuiConfiguration> for bb_imager_ui::customization::SysConfig {
    fn from(value: &GuiConfiguration) -> Self {
        Self {
            common: value.into(),
            usb_enable_dhcp: value
                .sd_customization
                .sysconf
                .as_ref()
                .and_then(|y| y.usb_enable_dhcp)
                .unwrap_or_default(),
        }
    }
}

impl From<&GuiConfiguration> for bb_imager_ui::customization::CloudInit {
    fn from(value: &GuiConfiguration) -> Self {
        Self {
            hostname: value
                .sd_customization
                .sysconf
                .as_ref()
                .and_then(|y| y.hostname.clone().map(Into::into)),
            timezone: value
                .sd_customization
                .sysconf
                .as_ref()
                .and_then(|y| y.timezone),
            keymap: value
                .sd_customization
                .sysconf
                .as_ref()
                .and_then(|y| y.keymap.as_deref())
                .and_then(crate::constants::keymap_layout),
            user: value.sd_customization.sysconf.as_ref().and_then(|y| {
                y.user
                    .clone()
                    .map(|u| (u.username.into(), u.password.into()))
            }),
            wifi: value
                .sd_customization
                .sysconf
                .as_ref()
                .and_then(|y| y.wifi.clone().map(|u| (u.ssid.into(), u.password.into()))),
            ssh: value
                .sd_customization
                .sysconf
                .as_ref()
                .and_then(|y| y.ssh.clone().map(Into::into))
                .unwrap_or_default(),
        }
    }
}

impl GuiConfiguration {
    pub(crate) fn load() -> std::io::Result<Self> {
        let mut data = Vec::with_capacity(512);
        let config_p = Self::config_path().unwrap();

        let mut config = std::fs::File::open(config_p)?;
        config.read_to_end(&mut data)?;

        Ok(serde_json::from_slice(&data).unwrap())
    }

    pub(crate) fn save(&self) -> std::io::Result<()> {
        let data = serde_json::to_string_pretty(self).unwrap();
        let config_p = Self::config_path().unwrap();

        tracing::info!("Configuration Path: {:?}", config_p);
        std::fs::create_dir_all(config_p.parent().unwrap())?;

        let mut config = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(config_p)?;

        config.write_all(data.as_bytes())?;

        Ok(())
    }

    fn config_path() -> Option<PathBuf> {
        let dirs = crate::helpers::project_dirs()?;
        Some(dirs.config_local_dir().join("config.json").to_owned())
    }

    pub(crate) fn update_sd_customization(&mut self, t: SdCustomization) {
        self.sd_customization = t;
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub(crate) struct SdCustomization {
    #[serde(skip_serializing_if = "Option::is_none")]
    sysconf: Option<SdSysconfCustomization>,
}

impl SdCustomization {
    pub(crate) fn update_sysconfig(&mut self, t: SdSysconfCustomization) {
        self.sysconf = Some(t)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SdSysconfCustomization {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) timezone: Option<chrono_tz::Tz>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) keymap: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) user: Option<SdCustomizationUser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) wifi: Option<SdCustomizationWifi>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) ssh: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) usb_enable_dhcp: Option<bool>,
}

impl From<&bb_imager_ui::customization::SysConfig> for SdSysconfCustomization {
    fn from(value: &bb_imager_ui::customization::SysConfig) -> Self {
        let mut temp: Self = (&value.common).into();
        temp.usb_enable_dhcp = Some(value.usb_enable_dhcp);

        temp
    }
}

impl From<&bb_imager_ui::customization::CloudInit> for SdSysconfCustomization {
    fn from(value: &bb_imager_ui::customization::CloudInit) -> Self {
        let ssh = if value.ssh.is_empty() {
            None
        } else {
            Some(value.ssh.to_string())
        };

        Self {
            hostname: value.hostname.as_ref().map(ToString::to_string),
            timezone: value.timezone,
            keymap: value.keymap.map(Into::into),
            user: value.user.as_ref().map(|(u, p)| SdCustomizationUser {
                username: u.to_string(),
                password: p.to_string(),
            }),
            wifi: value.wifi.as_ref().map(|(s, p)| SdCustomizationWifi {
                ssid: s.to_string(),
                password: p.to_string(),
            }),
            ssh,
            usb_enable_dhcp: None,
        }
    }
}
impl From<SdSysconfCustomization> for bb_imager_ui::customization::SysConfig {
    fn from(value: SdSysconfCustomization) -> Self {
        Self {
            usb_enable_dhcp: value.usb_enable_dhcp.unwrap_or_default(),
            common: value.into(),
        }
    }
}

impl From<SdSysconfCustomization> for bb_imager_ui::customization::CloudInit {
    fn from(value: SdSysconfCustomization) -> Self {
        Self {
            hostname: value.hostname.map(Into::into),
            timezone: value.timezone,
            keymap: value
                .keymap
                .as_deref()
                .and_then(crate::constants::keymap_layout),
            user: value.user.map(|u| (u.username.into(), u.password.into())),
            wifi: value
                .wifi
                .clone()
                .map(|u| (u.ssid.into(), u.password.into())),
            ssh: value.ssh.map(Into::into).unwrap_or_default(),
        }
    }
}

impl Default for SdSysconfCustomization {
    fn default() -> Self {
        Self {
            hostname: None,
            timezone: None,
            keymap: None,
            user: None,
            wifi: None,
            ssh: None,
            usb_enable_dhcp: if cfg!(target_os = "macos") {
                Some(true)
            } else {
                None
            },
        }
    }
}

impl SdSysconfCustomization {
    #[cfg(feature = "sd")]
    pub(crate) fn sysconfig(self) -> bb_flasher::sd::FlashingSdLinuxConfig {
        bb_flasher::sd::FlashingSdLinuxConfig::sysconfig(
            self.hostname.map(Into::into),
            self.timezone.map(|x| x.to_string()).map(Into::into),
            self.keymap.map(Into::into),
            self.user.map(|x| (x.username.into(), x.password.into())),
            self.wifi.map(|x| (x.ssid.into(), x.password.into())),
            self.ssh.map(Into::into),
            self.usb_enable_dhcp,
        )
    }

    #[cfg(feature = "sd")]
    pub(crate) fn cloudinit(self) -> bb_flasher::sd::FlashingSdLinuxConfig {
        bb_flasher::sd::FlashingSdLinuxConfig::cloud_init(
            self.hostname.map(Into::into),
            self.timezone.map(|x| x.to_string()).map(Into::into),
            self.keymap.map(Into::into),
            self.user.map(|x| (x.username.into(), x.password.into())),
            self.wifi.map(|x| (x.ssid.into(), x.password.into())),
            self.ssh.map(Into::into),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SdCustomizationUser {
    pub(crate) username: String,
    pub(crate) password: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SdCustomizationWifi {
    pub(crate) ssid: String,
    pub(crate) password: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sysconf_default_usb_dhcp_is_platform_specific() {
        let default = SdSysconfCustomization::default();
        if cfg!(target_os = "macos") {
            assert_eq!(default.usb_enable_dhcp, Some(true));
        } else {
            assert_eq!(default.usb_enable_dhcp, None);
        }
    }

    fn saved_config_with_keymap(keymap: &str) -> GuiConfiguration {
        let mut gui = GuiConfiguration::default();
        gui.update_sd_customization(SdCustomization {
            sysconf: Some(SdSysconfCustomization {
                keymap: Some(keymap.to_owned()),
                ..Default::default()
            }),
        });
        gui
    }

    /// A saved keymap has to come back into the UI, or the Customize page shows
    /// nothing while a keymap is still written to the image.
    #[test]
    fn saved_keymap_is_restored_into_the_ui() {
        let gui = saved_config_with_keymap("de");

        let ui: bb_imager_ui::customization::CloudInit = (&gui).into();

        assert_eq!(ui.keymap, Some("de"));
    }

    /// The damaging half of the same bug: the UI value is written back on every
    /// edit, so a keymap that fails to load is erased the next time the user
    /// touches any other field.
    #[test]
    fn restored_keymap_survives_a_round_trip_through_the_ui() {
        let gui = saved_config_with_keymap("de");

        let ui: bb_imager_ui::customization::CloudInit = (&gui).into();
        let saved_again: SdSysconfCustomization = (&ui).into();

        assert_eq!(saved_again.keymap.as_deref(), Some("de"));
    }

    /// The other conversion into the UI, used when a customization is carried
    /// between pages rather than loaded from disk.
    #[test]
    fn keymap_is_restored_from_a_stored_customization() {
        let stored = SdSysconfCustomization {
            keymap: Some("fr".to_owned()),
            ..Default::default()
        };

        let ui: bb_imager_ui::customization::CloudInit = stored.into();

        assert_eq!(ui.keymap, Some("fr"));
    }

    /// A hand-edited config can name a layout that does not exist; it should
    /// read as unset rather than be forced into the picker.
    #[test]
    fn unknown_keymap_is_dropped() {
        let gui = saved_config_with_keymap("not-a-layout");

        let ui: bb_imager_ui::customization::CloudInit = (&gui).into();

        assert_eq!(ui.keymap, None);
    }
}
