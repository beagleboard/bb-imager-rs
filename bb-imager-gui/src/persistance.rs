//! This module contains persistance for configuration

use std::io::{Read, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Configuration for GUI that should be presisted
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GuiConfiguration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) sd_customization: Option<SdCustomization>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) bcf_customization: Option<BcfCustomization>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) zepto_customization: Option<BcfCustomization>,
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
        self.sd_customization = Some(t);
    }

    pub(crate) fn update_bcf_customization(&mut self, t: BcfCustomization) {
        self.bcf_customization = Some(t)
    }

    pub(crate) fn update_zepto_customization(&mut self, t: BcfCustomization) {
        self.zepto_customization = Some(t)
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

    #[cfg(feature = "sd")]
    pub(crate) fn sysconfig(self) -> bb_flasher::sd::FlashingSdLinuxConfig {
        bb_flasher::sd::FlashingSdLinuxConfig::sysconfig(
            self.hostname.map(Into::into),
            self.timezone.map(Into::into),
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
            self.timezone.map(Into::into),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bcf_customization_defaults_to_verify() {
        assert!(BcfCustomization::default().verify);
        assert!(!BcfCustomization::default().update_verify(false).verify);
    }

    #[test]
    fn sd_user_validate_rejects_root() {
        assert!(!SdCustomizationUser::new("root".into(), "pw".into()).validate_username());
        assert!(SdCustomizationUser::new("beagle".into(), "pw".into()).validate_username());
    }

    #[test]
    fn sd_user_default_has_empty_password() {
        assert!(SdCustomizationUser::default().password.is_empty());
    }

    #[test]
    fn sd_user_builders_set_fields() {
        let user = SdCustomizationUser::default()
            .update_username("alice".into())
            .update_password("secret".into());
        assert_eq!(user.username, "alice");
        assert_eq!(user.password, "secret");
    }

    #[test]
    fn sd_wifi_builders_set_fields() {
        let wifi = SdCustomizationWifi::default()
            .update_ssid("net".into())
            .update_password("pw".into());
        assert_eq!(wifi.ssid, "net");
        assert_eq!(wifi.password, "pw");
    }

    #[test]
    fn sysconf_validate_user_follows_inner_user() {
        // No user configured is always valid.
        assert!(SdSysconfCustomization::default().validate_user());
        // A configured non-root user is valid; root is not.
        let ok = SdSysconfCustomization::default()
            .update_user(Some(SdCustomizationUser::new("beagle".into(), "pw".into())));
        assert!(ok.validate_user());
        let bad = SdSysconfCustomization::default()
            .update_user(Some(SdCustomizationUser::new("root".into(), "pw".into())));
        assert!(!bad.validate_user());
    }

    #[test]
    fn sysconf_builders_populate_all_fields() {
        let cfg = SdSysconfCustomization::default()
            .update_hostname(Some("beagle".into()))
            .update_timezone(Some("UTC".into()))
            .update_keymap(Some("us".into()))
            .update_ssh(Some("ssh-key".into()))
            .update_usb_enable_dhcp(Some(true))
            .update_wifi(Some(SdCustomizationWifi::default().update_ssid("net".into())))
            .update_user(Some(SdCustomizationUser::new("beagle".into(), "pw".into())));

        assert_eq!(cfg.hostname.as_deref(), Some("beagle"));
        assert_eq!(cfg.timezone.as_deref(), Some("UTC"));
        assert_eq!(cfg.keymap.as_deref(), Some("us"));
        assert_eq!(cfg.ssh.as_deref(), Some("ssh-key"));
        assert_eq!(cfg.usb_enable_dhcp, Some(true));
        assert_eq!(cfg.wifi.as_ref().map(|w| w.ssid.as_str()), Some("net"));
        assert_eq!(cfg.user.as_ref().map(|u| u.username.as_str()), Some("beagle"));
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
    fn sd_customization_wraps_sysconf() {
        let mut sd = SdCustomization::default();
        assert!(sd.sysconf_customization().is_none());
        sd.update_sysconfig(SdSysconfCustomization::default().update_hostname(Some("bb".into())));
        assert_eq!(
            sd.sysconf_customization()
                .and_then(|s| s.hostname.as_deref()),
            Some("bb")
        );
    }

    #[test]
    fn gui_configuration_updates_each_slot() {
        let mut gui = GuiConfiguration::default();
        assert!(gui.sd_customization.is_none());
        assert!(gui.bcf_customization.is_none());
        assert!(gui.zepto_customization.is_none());

        gui.update_sd_customization(SdCustomization::default());
        gui.update_bcf_customization(BcfCustomization::default());
        gui.update_zepto_customization(BcfCustomization::default().update_verify(false));

        assert!(gui.sd_customization.is_some());
        assert!(gui.bcf_customization.is_some());
        assert_eq!(gui.zepto_customization.map(|z| z.verify), Some(false));
    }

    #[test]
    fn empty_gui_configuration_serializes_to_empty_object() {
        // All fields are `skip_serializing_if = "Option::is_none"`.
        let json = serde_json::to_string(&GuiConfiguration::default()).unwrap();
        assert_eq!(json, "{}");
    }

    #[test]
    fn gui_configuration_round_trips_through_json() {
        let mut gui = GuiConfiguration::default();
        gui.update_bcf_customization(BcfCustomization { verify: false });
        gui.update_sd_customization({
            let mut sd = SdCustomization::default();
            sd.update_sysconfig(
                SdSysconfCustomization::default().update_hostname(Some("host".into())),
            );
            sd
        });

        let json = serde_json::to_string(&gui).unwrap();
        let back: GuiConfiguration = serde_json::from_str(&json).unwrap();

        assert_eq!(back.bcf_customization.map(|b| b.verify), Some(false));
        assert_eq!(
            back.sd_customization
                .and_then(|s| s.sysconf_customization().and_then(|c| c.hostname.clone())),
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
            .update_wifi(Some(
                SdCustomizationWifi::default()
                    .update_ssid("net".into())
                    .update_password("pw".into()),
            ));
        let _ = base.clone().sysconfig();
        let _ = base.cloudinit();
    }
}
