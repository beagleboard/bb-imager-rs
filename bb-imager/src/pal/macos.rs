use std::{
    io::Write,
    os::fd::FromRawFd,
    process::{Command, Stdio},
};

use security_framework::authorization::{Authorization, AuthorizationItemSetBuilder, Flags};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

impl crate::common::Destination {
    pub async fn open(&self) -> crate::error::Result<File> {
        if let Self::SdCard { path, .. } = self {
            let path = path.clone();
            tokio::task::spawn_blocking(move || open_auth(path))
                .await
                .unwrap()
        } else {
            unreachable!()
        }
    }
}

fn open_auth(path: String) -> crate::error::Result<File> {
    let rights = AuthorizationItemSetBuilder::new()
        .add_right(format!("sys.openfile.readwrite.{}", &path))
        .unwrap()
        .build();

    let auth = Authorization::new(
        Some(rights),
        None,
        Flags::INTERACTION_ALLOWED | Flags::EXTEND_RIGHTS | Flags::PREAUTHORIZE,
    )
    .unwrap();

    let form = auth.make_external_form().unwrap();

    let mut cmd = Command::new("/usr/libexec/authopen")
        .args(["-stdoutpipe", "-extauth", "-o", "2", &path])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = cmd.stdin.take().unwrap();
    let form_bytes: Vec<u8> = form.bytes.into_iter().map(|x| x as u8).collect();
    stdin.write_all(&form_bytes).unwrap();
    drop(stdin);

    let output = cmd.wait_with_output().unwrap();

    tracing::info!("Raw output: {output:#?}");
    tracing::info!("String output: {}", String::from_utf8_lossy(&output.stdout));

    let fd = i32::from_ne_bytes(output.stdout.try_into().unwrap());
    Ok(unsafe { tokio::fs::File::from_raw_fd(fd) })
}

/// TODO: Remove once a new version of rs_drivelist is published to crates.io
pub(crate) mod rs_drivelist {
    use rs_drivelist::device::{DeviceDescriptor, MountPoint};
    use serde::Deserialize;
    use std::process::Command;

    #[derive(Deserialize, Debug)]
    struct Disks {
        #[serde(rename = "AllDisksAndPartitions")]
        all_disks_and_partitions: Vec<Disk>,
    }

    #[derive(Deserialize, Debug)]
    struct Disk {
        #[serde(rename = "DeviceIdentifier")]
        device_identifier: String,
        #[serde(rename = "OSInternal")]
        os_internal: bool,
        #[serde(rename = "Size")]
        size: u64,
        #[serde(rename = "Content")]
        content: String,
        #[serde(rename = "Partitions")]
        partitions: Vec<Partition>,
    }

    #[derive(Deserialize, Debug)]
    struct Partition {
        #[serde(rename = "MountPoint")]
        mount_point: Option<String>,
        #[serde(rename = "Content")]
        content: String,
        #[serde(rename = "Size")]
        size: u64,
    }

    impl From<Disk> for DeviceDescriptor {
        fn from(value: Disk) -> Self {
            DeviceDescriptor {
                enumerator: "diskutil".to_string(),
                description: value.content,
                size: value.size,
                mountpoints: value.partitions.into_iter().map(MountPoint::from).collect(),
                device: format!("/dev/{}", value.device_identifier),
                raw: format!("/dev/r{}", value.device_identifier),
                isSystem: value.os_internal,
                isRemovable: !value.os_internal,
                ..Default::default()
            }
        }
    }

    impl From<Partition> for MountPoint {
        fn from(value: Partition) -> Self {
            MountPoint {
                path: value.mount_point.unwrap_or_default(),
                label: Some(value.content),
                totalBytes: Some(value.size),
                availableBytes: None,
            }
        }
    }

    pub(crate) fn diskutil() -> anyhow::Result<Vec<DeviceDescriptor>> {
        let output = Command::new("diskutil").args(["list", "-plist"]).output()?;

        if let Some(code) = output.status.code() {
            if code != 0 {
                return Err(anyhow::Error::msg(format!("lsblk ExitCode: {}", code)));
            }
        }

        if output.stderr.len() > 0 {
            return Err(anyhow::Error::msg(format!(
                "lsblk stderr: {}",
                std::str::from_utf8(&output.stderr).unwrap()
            )));
        }

        let parsed: Disks = plist::from_bytes(&output.stdout).unwrap();

        Ok(parsed
            .all_disks_and_partitions
            .into_iter()
            .map(DeviceDescriptor::from)
            .collect())
    }
}
