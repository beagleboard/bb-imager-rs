//! Windows backend, built on `CM_Register_Notification` (cfgmgr32).
//!
//! The textbook mechanism is `WM_DEVICECHANGE` with `DBT_DEVICEARRIVAL`, but
//! that needs a window procedure, and in a winit-based application the window
//! proc belongs to winit and raw messages are not exposed. `CM_Register_Notification`
//! (Windows 10 1709+) is callback-based, needs no `HWND`, and can be registered
//! from any thread.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures_channel::mpsc;
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_NOTIFY_ACTION, CM_NOTIFY_EVENT_DATA, CM_NOTIFY_FILTER, CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE,
    CM_Register_Notification, CM_Unregister_Notification, CR_SUCCESS, HCMNOTIFICATION,
};
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Ioctl::GUID_DEVINTERFACE_DISK;

use crate::Error;

pub(crate) struct Watcher {
    rx: mpsc::UnboundedReceiver<()>,
    notification: HCMNOTIFICATION,
    /// Owns the sender the callback borrows. Declared after `notification` only
    /// for clarity; the explicit `Drop` below controls the actual ordering.
    _sender: Box<mpsc::UnboundedSender<()>>,
}

impl Watcher {
    pub(crate) async fn start() -> Result<Self, Error> {
        let (tx, rx) = mpsc::unbounded();

        // Boxed so the address handed to the callback stays valid while this
        // `Watcher` lives, and is freed only after the notification is
        // unregistered in `Drop`.
        let sender = Box::new(tx);
        let context = (&raw const *sender).cast::<std::ffi::c_void>();

        let mut filter = CM_NOTIFY_FILTER {
            cbSize: std::mem::size_of::<CM_NOTIFY_FILTER>() as u32,
            FilterType: CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE,
            ..Default::default()
        };
        filter.u.DeviceInterface.ClassGuid = GUID_DEVINTERFACE_DISK;

        let mut notification = HCMNOTIFICATION::default();
        let ret = unsafe {
            CM_Register_Notification(&filter, Some(context), Some(callback), &mut notification)
        };

        if ret != CR_SUCCESS {
            return Err(Error::Backend(format!(
                "CM_Register_Notification failed: CONFIGRET {}",
                ret.0
            )));
        }

        Ok(Self {
            rx,
            notification,
            _sender: sender,
        })
    }
}

/// Invoked by the configuration manager on one of its own threads.
///
/// Must return promptly, and must never call `CM_Unregister_Notification` on
/// its own registration — that deadlocks. So this only sends and returns.
unsafe extern "system" fn callback(
    _notify: HCMNOTIFICATION,
    context: *const std::ffi::c_void,
    _action: CM_NOTIFY_ACTION,
    _event_data: *const CM_NOTIFY_EVENT_DATA,
    _event_data_size: u32,
) -> u32 {
    if !context.is_null() {
        // Safety: `context` is the address of the `Box<UnboundedSender>` owned by
        // the live `Watcher`; `Drop` unregisters before the box is freed.
        let tx = unsafe { &*context.cast::<mpsc::UnboundedSender<()>>() };
        // Both arrival and removal mean the same thing: re-enumerate. A closed
        // receiver just means nobody is listening any more.
        let _ = tx.unbounded_send(());
    }

    ERROR_SUCCESS.0
}

impl Drop for Watcher {
    fn drop(&mut self) {
        if !self.notification.is_invalid() {
            // Blocks until in-flight callbacks return, after which the boxed
            // sender is safe to free.
            let _ = unsafe { CM_Unregister_Notification(self.notification) };
        }
    }
}

impl futures_core::Stream for Watcher {
    type Item = ();

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx).poll_next(cx)
    }
}
