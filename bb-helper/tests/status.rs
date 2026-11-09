#![cfg(feature = "status")]

use std::sync::mpsc;

use bb_helper::status::{Status, send};

#[test]
fn send_delivers_when_there_is_room() {
    let (tx, rx) = mpsc::sync_channel(1);
    send(Some(&tx), Status::Writing(0.5));
    assert_eq!(rx.try_recv(), Ok(Status::Writing(0.5)));
}

#[test]
fn send_drops_instead_of_blocking() {
    let (tx, rx) = mpsc::sync_channel(1);
    send(Some(&tx), Status::Preparing);
    // The channel is full: dropped, and must not block.
    send(Some(&tx), Status::Verifying);
    assert_eq!(rx.try_recv(), Ok(Status::Preparing));
    assert!(rx.try_recv().is_err());

    drop(rx);
    // Disconnected and absent channels are no-ops too.
    send(Some(&tx), Status::Customizing);
    send(None, Status::Customizing);
}
