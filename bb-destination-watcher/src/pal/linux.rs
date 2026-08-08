//! Linux backend, built on udisks2's D-Bus signals.
//!
//! udisks2 is used rather than libudev or raw netlink because the GUI already
//! depends on it — `bb-flasher-sd` needs it to open and eject a card — so this
//! introduces no new runtime requirement. zbus exposes the signals as native
//! async streams, so no thread or channel is needed here.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::{StreamExt, stream::BoxStream};
use zbus::{MatchRule, MessageStream, message};

use crate::Error;

/// Interfaces whose arrival or departure means the destination list changed.
///
/// udisks2 also emits `InterfacesAdded`/`InterfacesRemoved` for jobs and other
/// bookkeeping objects — every `udisksctl` invocation produces a pair. Without
/// this filter, routine disk activity would trigger a re-enumeration.
const WATCHED: [&str; 2] = [
    "org.freedesktop.UDisks2.Block",
    "org.freedesktop.UDisks2.Drive",
];

/// Restricts the property-change subscription to block objects.
const BLOCK_PATHS: &str = "/org/freedesktop/UDisks2/block_devices";
const BLOCK_INTERFACE: &str = "org.freedesktop.UDisks2.Block";

pub(crate) struct Watcher {
    /// Held for its lifetime: dropping the client would tear down the D-Bus
    /// connection the signal streams are reading from.
    _client: udisks2::Client,
    events: BoxStream<'static, ()>,
}

impl Watcher {
    pub(crate) async fn start() -> Result<Self, Error> {
        let client = udisks2::Client::new().await.map_err(backend)?;
        let object_manager = client.object_manager();

        // Devices appearing and disappearing outright.
        let added = object_manager
            .receive_interfaces_added()
            .await
            .map_err(backend)?
            .filter_map(|signal| async move {
                let args = signal.args().inspect_err(parse_failed).ok()?;
                let interfaces = args.interfaces_and_properties.keys().map(|i| i.as_str());
                log_signal(
                    "InterfacesAdded",
                    args.object_path.as_str(),
                    interfaces.clone(),
                );
                is_watched(interfaces).then_some(())
            });

        let removed = object_manager
            .receive_interfaces_removed()
            .await
            .map_err(backend)?
            .filter_map(|signal| async move {
                let args = signal.args().inspect_err(parse_failed).ok()?;
                let interfaces = args.interfaces.iter().map(|i| i.as_str());
                log_signal(
                    "InterfacesRemoved",
                    args.object_path.as_str(),
                    interfaces.clone(),
                );
                is_watched(interfaces).then_some(())
            });

        // Inserting a card into a reader that is already attached does not
        // create an object: the block device persists and only its properties
        // change (notably `Size`, 0 -> the card's size). `InterfacesAdded`
        // alone would therefore miss this application's most common action, so
        // property changes on block objects also count as a reason to
        // re-enumerate.
        //
        // The match rule filters server-side on the object path namespace and
        // on the interface, so unrelated udisks2 chatter is never delivered.
        let rule = MatchRule::builder()
            .msg_type(message::Type::Signal)
            .interface("org.freedesktop.DBus.Properties")
            .map_err(backend)?
            .member("PropertiesChanged")
            .map_err(backend)?
            .path_namespace(BLOCK_PATHS)
            .map_err(backend)?
            .arg(0, BLOCK_INTERFACE)
            .map_err(backend)?
            .build();

        let properties =
            MessageStream::for_match_rule(rule, object_manager.inner().connection(), None)
                .await
                .map_err(backend)?
                .filter_map(|msg| async move {
                    let msg = msg.ok()?;
                    log_signal(
                        "PropertiesChanged",
                        msg.header().path()?.as_str(),
                        std::iter::once(BLOCK_INTERFACE),
                    );
                    Some(())
                });

        let events =
            futures_util::stream::select(futures_util::stream::select(added, removed), properties)
                .boxed();

        Ok(Self {
            _client: client,
            events,
        })
    }
}

fn backend(e: impl std::fmt::Display) -> Error {
    Error::Backend(e.to_string())
}

/// A signal we subscribed to but could not decode is a hole in the watcher, so
/// it is worth surfacing rather than dropping silently.
fn parse_failed(e: &zbus::Error) {
    tracing::warn!("Ignoring undecodable udisks2 signal: {e}");
}

/// Every signal seen, before filtering.
///
/// udisks2 is chatty and the interface names are the whole basis of the filter,
/// so seeing what actually arrived is the fastest way to diagnose either missed
/// changes or needless re-enumeration.
fn log_signal<'a>(signal: &str, path: &str, interfaces: impl Iterator<Item = &'a str>) {
    if tracing::enabled!(tracing::Level::DEBUG) {
        tracing::debug!(
            "{signal} {path} [{}]",
            interfaces.collect::<Vec<_>>().join(", ")
        );
    }
}

fn is_watched<'a>(mut interfaces: impl Iterator<Item = &'a str>) -> bool {
    interfaces.any(|i| WATCHED.contains(&i))
}

impl futures_core::Stream for Watcher {
    type Item = ();

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.events.as_mut().poll_next(cx)
    }
}
