use std::{fs::File, path::Path};

use crate::{Error, Result};

pub(crate) fn format(_dst: &Path) -> Result<()> {
    unimplemented!()
}

#[cfg(not(feature = "macos_authopen"))]
pub(crate) fn open(dst: &Path) -> Result<File> {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(false)
        .open(dst)
        .map_err(Into::into)
}

#[cfg(feature = "macos_authopen")]
pub(crate) fn open(dst: &Path) -> Result<File> {
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

    let rights = AuthorizationItemSetBuilder::new()
        .add_right(format!("sys.openfile.readwrite.{}", dst.to_str().unwrap()))
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
        .args(["unmountDisk", dst.to_str().unwrap()])
        .output()
        .map_err(|_| Error::FailedToOpenDestination("Failed to unmount disk".to_string()))?;

    let mut cmd = Command::new("/usr/libexec/authopen")
        .args(["-stdoutpipe", "-extauth", "-o", "2", dst.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(OwnedFd::from(pipe1))
        .spawn()
        .map_err(|_| Error::FailedToOpenDestination("Failed to open disk".to_string()))?;

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
                    if let Some(fd) = scm_rights.into_iter().next() {
                        tracing::debug!("receive file descriptor");
                        return Ok(unsafe { File::from_raw_fd(fd) });
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!("Macos Error: {}", e);
        }
    }

    let _ = cmd.wait();

    Err(Error::FailedToOpenDestination(
        "Authopen failed to open the file".to_string(),
    ))
}

/// TODO: Implement real eject
impl crate::helpers::Eject for std::fs::File {
    fn eject(self) -> std::io::Result<()> {
        let _ = self.sync_all();
        Ok(())
    }
}
