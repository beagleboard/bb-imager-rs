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
