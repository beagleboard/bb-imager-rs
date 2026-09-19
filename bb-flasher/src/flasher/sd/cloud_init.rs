use std::collections::HashMap;

use serde::Serialize;

#[derive(Serialize, Default)]
pub(crate) struct CloudInitConfig<'a> {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    users: Vec<User<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    ssh_authorized_keys: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    keyboard: Option<Keyboard<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timezone: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hostname: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    network: Option<Network<'a>>,
}

impl<'a> CloudInitConfig<'a> {
    pub(crate) fn new(
        hostname: Option<&'a str>,
        timezone: Option<&'a str>,
        keymap: Option<&'a str>,
        user: Option<(&'a str, &'a str)>,
        wifi: Option<(&'a str, &'a str)>,
        ssh: Option<&'a str>,
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
struct Network<'a> {
    version: u8,
    renderer: &'static str,
    wifis: HashMap<&'static str, WifiInterface<'a>>,
}

#[derive(Debug, Serialize)]
struct WifiInterface<'a> {
    #[serde(rename = "access-points")]
    access_points: HashMap<&'a str, AccessPoint<'a>>,
}

#[derive(Debug, Serialize)]
struct AccessPoint<'a> {
    password: &'a str,
}

#[derive(Serialize)]
struct User<'a> {
    name: &'a str,
    plain_text_passwd: &'a str,
}

#[derive(Serialize)]
struct Keyboard<'a> {
    layout: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user() {
        let data = CloudInitConfig {
            users: vec![User {
                name: "beagle",
                plain_text_passwd: "password",
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
            keyboard: Some(Keyboard { layout: "us" }),
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
            timezone: Some("America/New_York"),
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
            hostname: Some("myhost"),
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
                            "network_ssid_name",
                            AccessPoint {
                                password: "password",
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
