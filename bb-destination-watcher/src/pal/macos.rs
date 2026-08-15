//! macOS backend, built on DiskArbitration.
//!
//! # Why a dedicated thread
//!
//! `CFRetained<T>` is only `Send` when `T: Send + Sync`, which `DASession` is
//! not. Holding the session inside [`Watcher`] would therefore make the whole
//! watcher `!Send`, and it has to cross threads to be driven by an async
//! executor. So the session, its callbacks and its run loop all stay on one
//! owned thread, and the only thing shared with the outside world is an mpsc
//! receiver.
//!
//! This also keeps to the run-loop API that `bb-drivelist`'s macOS enumerator
//! already uses successfully, rather than `DASessionSetDispatchQueue`, which
//! would drag in `dispatch2` and re-introduce the `Send` question.

use std::ffi::c_void;
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use futures_channel::mpsc;
use objc2_core_foundation::{CFRunLoop, kCFRunLoopDefaultMode};
use objc2_disk_arbitration::{
    DADisk, DARegisterDiskAppearedCallback, DARegisterDiskDisappearedCallback, DASession,
    DAUnregisterCallback,
};

use crate::Error;

/// How long each run-loop turn blocks before the stop flag is re-checked. Also
/// the worst-case delay between dropping the watcher and the thread exiting.
const RUN_LOOP_TURN_SECS: f64 = 0.25;

/// How long `start` waits for the thread to report that registration succeeded.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) struct Watcher {
    rx: mpsc::UnboundedReceiver<()>,
    stop: Arc<AtomicBool>,
}

impl Watcher {
    pub(crate) async fn start() -> Result<Self, Error> {
        let (tx, rx) = mpsc::unbounded();
        let stop = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);

        let stop_thread = Arc::clone(&stop);
        std::thread::Builder::new()
            .name("bb-destination-watcher".to_owned())
            .spawn(move || run(&tx, &stop_thread, &ready_tx))
            .map_err(|e| Error::Backend(format!("could not spawn watcher thread: {e}")))?;

        // Brief and bounded: the thread only has to create a session and
        // register two callbacks before reporting back.
        match ready_rx.recv_timeout(STARTUP_TIMEOUT) {
            Ok(Ok(())) => Ok(Self { rx, stop }),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(Error::Backend(format!("watcher thread did not start: {e}"))),
        }
    }
}

/// Owns every DiskArbitration object; never leaves this thread.
fn run(
    tx: &mpsc::UnboundedSender<()>,
    stop: &AtomicBool,
    ready: &std::sync::mpsc::SyncSender<Result<(), Error>>,
) {
    let fail = |ready: &std::sync::mpsc::SyncSender<_>, msg: &str| {
        let _ = ready.send(Err(Error::Backend(msg.to_owned())));
    };

    // `None` selects the default allocator.
    let Some(session) = (unsafe { DASession::new(None) }) else {
        fail(ready, "DASessionCreate returned null");
        return;
    };
    let Some(run_loop) = CFRunLoop::current() else {
        fail(ready, "no current run loop on watcher thread");
        return;
    };
    let Some(mode) = (unsafe { kCFRunLoopDefaultMode }) else {
        fail(ready, "kCFRunLoopDefaultMode unavailable");
        return;
    };

    // Safety: `tx` outlives the scheduled session — it is dropped at the end of
    // this function, after the callbacks have been unregistered below.
    let context = std::ptr::from_ref(tx).cast::<c_void>().cast_mut();

    unsafe {
        // Registered separately rather than sharing one function, so each
        // (callback, context) pair is distinct and can be unregistered on its own.
        DARegisterDiskAppearedCallback(&session, None, Some(disk_appeared), context);
        DARegisterDiskDisappearedCallback(&session, None, Some(disk_disappeared), context);
        session.schedule_with_run_loop(&run_loop, mode);
    }

    let _ = ready.send(Ok(()));

    while !stop.load(Ordering::Relaxed) {
        CFRunLoop::run_in_mode(unsafe { kCFRunLoopDefaultMode }, RUN_LOOP_TURN_SECS, false);
    }

    unsafe {
        session.unschedule_from_run_loop(&run_loop, mode);
        DAUnregisterCallback(&session, fn_ptr(disk_appeared), context);
        DAUnregisterCallback(&session, fn_ptr(disk_disappeared), context);
    }
}

type DiskCallback = unsafe extern "C-unwind" fn(NonNull<DADisk>, *mut c_void);

fn fn_ptr(f: DiskCallback) -> NonNull<c_void> {
    NonNull::new((f as *const ()).cast_mut().cast::<c_void>())
        .expect("function pointers are never null")
}

unsafe extern "C-unwind" fn disk_appeared(_disk: NonNull<DADisk>, context: *mut c_void) {
    notify(context);
}

unsafe extern "C-unwind" fn disk_disappeared(_disk: NonNull<DADisk>, context: *mut c_void) {
    notify(context);
}

/// Appearance and disappearance mean the same thing to a consumer: re-enumerate.
fn notify(context: *mut c_void) {
    if context.is_null() {
        return;
    }

    // Safety: `context` is the address of the `UnboundedSender` owned by `run`,
    // which unregisters these callbacks before returning.
    let tx = unsafe { &*context.cast::<mpsc::UnboundedSender<()>>() };
    // A closed receiver just means the watcher was dropped.
    let _ = tx.unbounded_send(());
}

impl Drop for Watcher {
    fn drop(&mut self) {
        // Deliberately does not join: the thread notices within one run-loop
        // turn and tears itself down, and `Drop` may run on the UI thread.
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl futures_core::Stream for Watcher {
    type Item = ();

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx).poll_next(cx)
    }
}
