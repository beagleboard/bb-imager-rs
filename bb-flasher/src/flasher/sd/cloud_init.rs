use std::collections::HashMap;

use serde::Serialize;

#[derive(Serialize, Default)]
pub(crate) struct CloudInitConfig {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    users: Vec<User>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    ssh_authorized_keys: Vec<Box<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    keyboard: Option<Keyboard>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timezone: Option<Box<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hostname: Option<Box<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    network: Option<Network>,
}

impl CloudInitConfig {
    pub(crate) fn new(
        hostname: Option<Box<str>>,
        timezone: Option<Box<str>>,
        keymap: Option<Box<str>>,
        user: Option<(Box<str>, Box<str>)>,
        wifi: Option<(Box<str>, Box<str>)>,
        ssh: Option<Box<str>>,
    ) -> Self {
        Self {
            users: user
                .map(|(name, plain_text_passwd)| {
                    vec![User {
                        name,
                        plain_text_passwd,
                    }]
                })
                .unwrap_or_default(),
            ssh_authorized_keys: ssh.map(|x| vec![x]).unwrap_or_default(),
            keyboard: keymap.map(|x| Keyboard { layout: x }),
            timezone,
            hostname,
            network: wifi.map(|(ssid, password)| Network {
                version: 2,
                renderer: "NetworkManager",
                wifis: HashMap::from([(
                    "wlo1",
                    WifiInterface {
                        access_points: HashMap::from([(ssid, AccessPoint { password })]),
                    },
                )]),
            }),
        }
    }

    pub(crate) fn to_file_data(&self) -> Box<[u8]> {
        let mut temp = String::new();

        temp.push_str("#cloud-config\n");
        temp.push_str(&yaml_serde::to_string(self).unwrap());

        temp.into_bytes().into()
    }
}

#[derive(Serialize)]
struct Network {
    version: u8,
    renderer: &'static str,
    wifis: HashMap<&'static str, WifiInterface>,
}

#[derive(Debug, Serialize)]
struct WifiInterface {
    #[serde(rename = "access-points")]
    access_points: HashMap<Box<str>, AccessPoint>,
}

#[derive(Debug, Serialize)]
struct AccessPoint {
    password: Box<str>,
}

#[derive(Serialize)]
struct User {
    name: Box<str>,
    plain_text_passwd: Box<str>,
}

#[derive(Serialize)]
struct Keyboard {
    layout: Box<str>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user() {
        let data = CloudInitConfig {
            users: vec![User {
                name: "beagle".into(),
                plain_text_passwd: "password".into(),
            }],
            ..Default::default()
        };
        let expected = r#"
users:
- name: beagle
  plain_text_passwd: password"#;

        assert_eq!(
            yaml_serde::to_string(&data).unwrap().trim(),
            expected.trim()
        );
    }

    #[test]
    fn keyboard() {
        let data = CloudInitConfig {
            keyboard: Some(Keyboard {
                layout: "us".into(),
            }),
            ..Default::default()
        };
        let expected = r#"
keyboard:
  layout: us"#;

        assert_eq!(
            yaml_serde::to_string(&data).unwrap().trim(),
            expected.trim()
        );
    }

    #[test]
    fn timezone() {
        let data = CloudInitConfig {
            timezone: Some("America/New_York".into()),
            ..Default::default()
        };
        let expected = r#"
timezone: America/New_York"#;

        assert_eq!(
            yaml_serde::to_string(&data).unwrap().trim(),
            expected.trim()
        );
    }

    #[test]
    fn hostname() {
        let data = CloudInitConfig {
            hostname: Some("myhost".into()),
            ..Default::default()
        };
        let expected = r#"
hostname: myhost"#;

        assert_eq!(
            yaml_serde::to_string(&data).unwrap().trim(),
            expected.trim()
        );
    }

    #[test]
    fn network() {
        let data = CloudInitConfig {
            network: Some(Network {
                version: 2,
                renderer: "NetworkManager",
                wifis: HashMap::from([(
                    "wlp2s0b1",
                    WifiInterface {
                        access_points: HashMap::from([(
                            "network_ssid_name".into(),
                            AccessPoint {
                                password: "password".into(),
                            },
                        )]),
                    },
                )]),
            }),
            ..Default::default()
        };
        let expected = r#"
network:
  version: 2
  renderer: NetworkManager
  wifis:
    wlp2s0b1:
      access-points:
        network_ssid_name:
          password: password
        "#;

        assert_eq!(
            yaml_serde::to_string(&data).unwrap().trim(),
            expected.trim()
        );
    }
}
