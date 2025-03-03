use thiserror::Error;
use tokio::fs::File;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to open destination {0}")]
    FailedToOpenDestionation(String),
}

pub(crate) async fn format_sd(_dst: &str) -> crate::error::Result<()> {
    unimplemented!()
}

#[cfg(not(feature = "macos_authopen"))]
pub(crate) async fn open_sd(dst: &str) -> crate::error::Result<File> {
    tokio::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(false)
        .open(dst)
        .await
        .map_err(Into::into)
}

#[cfg(feature = "macos_authopen")]
pub(crate) async fn open_sd(dst: &str) -> crate::error::Result<File> {
    use nix::cmsg_space;
    use nix::sys::socket::{ControlMessageOwned, MsgFlags};
    use security_framework::authorization::{Authorization, AuthorizationItemSetBuilder, Flags};
    use std::{
        io::{IoSliceMut, Write},
        os::{
            fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
            unix::net::UnixStream,
        },
        process::{Command, Stdio},
    };

    fn open_auth(path: String) -> crate::error::Result<File> {
        let rights = AuthorizationItemSetBuilder::new()
            .add_right(format!("sys.openfile.readwrite.{}", &path))
            .expect("Failed to create right")
            .build();

        let auth = Authorization::new(
            Some(rights),
            None,
            Flags::INTERACTION_ALLOWED | Flags::EXTEND_RIGHTS | Flags::PREAUTHORIZE,
        )
        .expect("Failed to create authorization");

        let form = auth
            .make_external_form()
            .expect("Failed to make external form");
        let (pipe0, pipe1) = UnixStream::pair().expect("Failed to create socket");

        let _ = Command::new("diskutil")
            .args(["unmountDisk", &path])
            .output()
            .map_err(|_| Error::FailedToOpenDestionation(format!("Failed to unmount disk")))?;

        let mut cmd = Command::new("/usr/libexec/authopen")
            .args(["-stdoutpipe", "-extauth", "-o", "2", &path])
            .stdin(Stdio::piped())
            .stdout(OwnedFd::from(pipe1))
            .spawn()
            .map_err(|_| Error::FailedToOpenDestionation(format!("Failed to open disk")))?;

        // Send authorization form
        let mut stdin = cmd.stdin.take().expect("Missing stdin");
        let form_bytes: Vec<u8> = form.bytes.into_iter().map(|x| x as u8).collect();
        stdin
            .write_all(&form_bytes)
            .expect("Failed to write to stdin");
        drop(stdin);

        const IOV_BUF_SIZE: usize =
            unsafe { nix::libc::CMSG_SPACE(std::mem::size_of::<std::ffi::c_int>() as u32) } as usize;
        let mut iov_buf = [0u8; IOV_BUF_SIZE];
        let mut iov = [IoSliceMut::new(&mut iov_buf)];

        let mut cmsg = cmsg_space!([RawFd; 1]);

        match nix::sys::socket::recvmsg::<()>(
            pipe0.as_raw_fd(),
            &mut iov,
            Some(&mut cmsg),
            MsgFlags::empty(),
        ) {
            Ok(result) => {
                tracing::info!("Result: {:#?}", result);

                for msg in result.cmsgs().expect("Unexpected error") {
                    if let ControlMessageOwned::ScmRights(scm_rights) = msg {
                        for fd in scm_rights {
                            tracing::debug!("receive file descriptor: {fd}");
                            return Ok(unsafe { tokio::fs::File::from_raw_fd(fd) });
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Macos Error: {}", e);
            }
        }

        let _ = cmd.wait();

        Err(Error::FailedToOpenDestionation("Authopen failed to open the file".to_string()).into())
    }

    let path = dst.to_string();
    tokio::task::spawn_blocking(move || open_auth(path))
        .await
        .expect("Tokio runtime failed to spawn blocking task")
}
