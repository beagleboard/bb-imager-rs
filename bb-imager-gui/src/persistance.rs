//! This module contains persistance for configuration

use std::{io::Read, path::PathBuf};

use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

/// Configuration for GUI that should be presisted
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GuiConfiguration {
    #[serde(skip_serializing_if = "Option::is_none")]
    sd_customization: Option<SdCustomization>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bcf_customization: Option<BcfCustomization>,
    #[cfg(feature = "pb2_mspm0")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pb2_mspm0_customization: Option<Pb2Mspm0Customization>,
}

impl GuiConfiguration {
    pub(crate) fn load() -> std::io::Result<Self> {
        let mut data = Vec::with_capacity(512);
        let config_p = Self::config_path().unwrap();

        let mut config = std::fs::File::open(config_p)?;
        config.read_to_end(&mut data)?;

        Ok(serde_json::from_slice(&data).unwrap())
    }

    pub(crate) async fn save(&self) -> std::io::Result<()> {
        let data = serde_json::to_string_pretty(self).unwrap();
        let config_p = Self::config_path().unwrap();

        tracing::info!("Configuration Path: {:?}", config_p);
        tokio::fs::create_dir_all(config_p.parent().unwrap()).await?;

        let mut config = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(config_p)
            .await?;

        config.write_all(data.as_bytes()).await?;

        Ok(())
    }

    fn config_path() -> Option<PathBuf> {
        let dirs = crate::helpers::project_dirs()?;
        Some(dirs.config_local_dir().join("config.json").to_owned())
    }

    pub(crate) const fn sd_customization(&self) -> Option<&SdCustomization> {
        self.sd_customization.as_ref()
    }

    pub(crate) const fn bcf_customization(&self) -> Option<&BcfCustomization> {
        self.bcf_customization.as_ref()
    }

    #[cfg(feature = "pb2_mspm0")]
    pub(crate) const fn pb2_mspm0_customization(&self) -> Option<&Pb2Mspm0Customization> {
        self.pb2_mspm0_customization.as_ref()
    }

    pub(crate) fn update_sd_customization(&mut self, t: SdCustomization) {
        self.sd_customization = Some(t);
    }

    pub(crate) fn update_bcf_customization(&mut self, t: BcfCustomization) {
        self.bcf_customization = Some(t)
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub(crate) struct SdCustomization {
    #[serde(skip_serializing_if = "Option::is_none")]
    sysconf: Option<SdSysconfCustomization>,
}

impl SdCustomization {
    pub(crate) fn sysconf_customization(&self) -> Option<&SdSysconfCustomization> {
        self.sysconf.as_ref()
    }

    pub(crate) fn update_sysconfig(&mut self, t: SdSysconfCustomization) {
        self.sysconf = Some(t)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SdSysconfCustomization {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) timezone: Option<String>,
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

    pub(crate) fn update_timezone(mut self, t: Option<String>) -> Self {
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

    pub(crate) fn validate_user(&self) -> bool {
        match &self.user {
            Some(x) => x.validate_username(),
            None => true,
        }
    }
}

impl From<SdSysconfCustomization> for bb_flasher::sd::FlashingSdLinuxConfig {
    fn from(value: SdSysconfCustomization) -> Self {
        Self::sysconfig(
            value.hostname.map(Into::into),
            value.timezone.map(Into::into),
            value.keymap.map(Into::into),
            value.user.map(|x| (x.username.into(), x.password.into())),
            value.wifi.map(|x| (x.ssid.into(), x.password.into())),
            value.ssh.map(Into::into),
            value.usb_enable_dhcp,
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

    pub(crate) fn update_username(mut self, t: String) -> Self {
        self.username = t;
        self
    }

    pub(crate) fn update_password(mut self, t: String) -> Self {
        self.password = t;
        self
    }

    pub(crate) fn validate_username(&self) -> bool {
        self.username != "root"
    }
}

impl Default for SdCustomizationUser {
    fn default() -> Self {
        Self::new(whoami::username().unwrap_or_default(), String::new())
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SdCustomizationWifi {
    pub(crate) ssid: String,
    pub(crate) password: String,
}

impl SdCustomizationWifi {
    pub(crate) fn update_ssid(mut self, t: String) -> Self {
        self.ssid = t;
        self
    }

    pub(crate) fn update_password(mut self, t: String) -> Self {
        self.password = t;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BcfCustomization {
    pub(crate) verify: bool,
}

impl BcfCustomization {
    pub(crate) fn update_verify(mut self, t: bool) -> Self {
        self.verify = t;
        self
    }
}

impl Default for BcfCustomization {
    fn default() -> Self {
        Self { verify: true }
    }
}

#[cfg(feature = "pb2_mspm0")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Pb2Mspm0Customization {
    pub(crate) persist_eeprom: bool,
}

#[cfg(feature = "pb2_mspm0")]
impl Pb2Mspm0Customization {
    pub(crate) fn update_persist_eeprom(mut self, t: bool) -> Self {
        self.persist_eeprom = t;
        self
    }
}

#[cfg(feature = "pb2_mspm0")]
impl Default for Pb2Mspm0Customization {
    fn default() -> Self {
        Self {
            persist_eeprom: true,
        }
    }
}
