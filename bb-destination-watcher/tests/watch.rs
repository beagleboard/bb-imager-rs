//! Exercises the real platform backend.
//!
//! Skips rather than fails where the platform service is unavailable — CI
//! containers frequently have no udisks2, and a headless macOS runner may have
//! no DiskArbitration session. The point is to catch a backend that is wired up
//! wrongly, not to assert what the environment provides.

use std::time::Duration;

use futures_util::StreamExt;

/// Nothing is being plugged in while this runs, so an event here means the
/// backend is reporting unrelated churn. On Linux that is usually a missing
/// interface filter letting mount/unmount or job objects through, which would
/// put the GUI back to re-enumerating constantly.
#[tokio::test]
async fn stays_quiet_when_no_devices_change() {
    let Ok(mut watcher) = bb_destination_watcher::Watcher::new().await else {
        eprintln!("skipping: no destination watcher available in this environment");
        return;
    };

    let event = tokio::time::timeout(Duration::from_millis(750), watcher.next()).await;

    assert!(
        event.is_err(),
        "watcher produced {event:?} without any device change"
    );
}
