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
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub(crate) struct SdCustomization {
    #[serde(default)]
    pub(crate) sysconf: SdSysconfCustomization,
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

impl From<SdSysconfCustomization> for bb_imager_ui::configuration::CloudInit {
    fn from(value: SdSysconfCustomization) -> Self {
        Self {
            hostname: value.hostname.map(Into::into),
            timezone: value.timezone,
            // The stored keymap is a `String`; the page works in the
            // `&'static str`s from `KEYMAP_LAYOUTS`, so it has to be resolved
            // back through the table.
            keymap: value
                .keymap
                .as_deref()
                .and_then(crate::constants::keymap_layout),
            user: value.user.map(|u| (u.username.into(), u.password.into())),
            wifi: value.wifi.map(|w| (w.ssid.into(), w.password.into())),
            ssh: value.ssh.map(Into::into).unwrap_or_default(),
        }
    }
}

impl From<SdSysconfCustomization> for bb_imager_ui::configuration::SysConfig {
    fn from(value: SdSysconfCustomization) -> Self {
        Self {
            usb_enable_dhcp: value.usb_enable_dhcp.unwrap_or_default(),
            common: value.into(),
        }
    }
}

impl From<&bb_imager_ui::configuration::CloudInit> for SdSysconfCustomization {
    fn from(value: &bb_imager_ui::configuration::CloudInit) -> Self {
        // Cloud-init has no USB DHCP toggle to report, so that field keeps
        // whatever the platform defaults to.
        Self::default()
            .update_hostname(value.hostname.as_ref().map(ToString::to_string))
            .update_timezone(value.timezone)
            .update_keymap(value.keymap.map(Into::into))
            .update_user(value.user.as_ref().map(|(username, password)| {
                SdCustomizationUser::new(username.to_string(), password.to_string())
            }))
            .update_wifi(
                value
                    .wifi
                    .as_ref()
                    .map(|(ssid, password)| SdCustomizationWifi {
                        ssid: ssid.to_string(),
                        password: password.to_string(),
                    }),
            )
            // An unset SSH key is the empty string in the page, but absent here.
            .update_ssh((!value.ssh.is_empty()).then(|| value.ssh.to_string()))
    }
}

impl From<&bb_imager_ui::configuration::SysConfig> for SdSysconfCustomization {
    fn from(value: &bb_imager_ui::configuration::SysConfig) -> Self {
        SdSysconfCustomization::from(&value.common)
            .update_usb_enable_dhcp(Some(value.usb_enable_dhcp))
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
    pub(crate) fn update_hostname(mut self, t: Option<String>) -> Self {
        self.hostname = t;
        self
    }

    pub(crate) fn update_timezone(mut self, t: Option<chrono_tz::Tz>) -> Self {
        self.timezone = t;
        self
    }

    pub(crate) fn update_keymap(mut self, t: Option<String>) -> Self {
        self.keymap = t;
        self
    }

    pub(crate) fn update_user(mut self, t: Option<SdCustomizationUser>) -> Self {
        self.user = t;
        self
    }

    pub(crate) fn update_wifi(mut self, t: Option<SdCustomizationWifi>) -> Self {
        self.wifi = t;
        self
    }

    pub(crate) fn update_ssh(mut self, t: Option<String>) -> Self {
        self.ssh = t;
        self
    }

    pub(crate) fn update_usb_enable_dhcp(mut self, t: Option<bool>) -> Self {
        self.usb_enable_dhcp = t;
        self
    }

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

impl SdCustomizationUser {
    pub(crate) const fn new(username: String, password: String) -> Self {
        Self { username, password }
    }
}

impl Default for SdCustomizationUser {
    fn default() -> Self {
        Self::new(crate::helpers::default_user().to_owned(), String::new())
    }
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
    fn sd_user_default_has_empty_password() {
        assert!(SdCustomizationUser::default().password.is_empty());
    }

    #[test]
    fn sysconf_builders_populate_all_fields() {
        let cfg = SdSysconfCustomization::default()
            .update_hostname(Some("beagle".into()))
            .update_timezone(Some("UTC".parse().unwrap()))
            .update_keymap(Some("us".into()))
            .update_ssh(Some("ssh-key".into()))
            .update_usb_enable_dhcp(Some(true))
            .update_wifi(Some(SdCustomizationWifi {
                ssid: "net".into(),
                password: String::new(),
            }))
            .update_user(Some(SdCustomizationUser::new("beagle".into(), "pw".into())));

        assert_eq!(cfg.hostname.as_deref(), Some("beagle"));
        assert_eq!(cfg.timezone, Some(chrono_tz::Tz::UTC));
        assert_eq!(cfg.keymap.as_deref(), Some("us"));
        assert_eq!(cfg.ssh.as_deref(), Some("ssh-key"));
        assert_eq!(cfg.usb_enable_dhcp, Some(true));
        assert_eq!(cfg.wifi.as_ref().map(|w| w.ssid.as_str()), Some("net"));
        assert_eq!(
            cfg.user.as_ref().map(|u| u.username.as_str()),
            Some("beagle")
        );
    }

    #[test]
    fn sysconf_default_usb_dhcp_is_platform_specific() {
        let default = SdSysconfCustomization::default();
        if cfg!(target_os = "macos") {
            assert_eq!(default.usb_enable_dhcp, Some(true));
        } else {
            assert_eq!(default.usb_enable_dhcp, None);
        }
    }

    #[test]
    fn gui_configuration_round_trips_through_json() {
        let gui = GuiConfiguration {
            sd_customization: SdCustomization {
                sysconf: SdSysconfCustomization {
                    hostname: Some("host".into()),
                    ..Default::default()
                },
            },
        };

        let json = serde_json::to_string(&gui).unwrap();
        let back: GuiConfiguration = serde_json::from_str(&json).unwrap();

        assert_eq!(
            back.sd_customization.sysconf.hostname,
            Some("host".to_string())
        );
    }

    #[cfg(feature = "sd")]
    #[test]
    fn sysconf_converts_to_flasher_configs_without_panicking() {
        // Exercises the sysconfig/cloudinit bridges into bb_flasher.
        let base = SdSysconfCustomization::default()
            .update_hostname(Some("beagle".into()))
            .update_user(Some(SdCustomizationUser::new("beagle".into(), "pw".into())))
            .update_wifi(Some(SdCustomizationWifi {
                ssid: "net".into(),
                password: "pw".into(),
            }));
        let _ = base.clone().sysconfig();
        let _ = base.cloudinit();
    }
}
