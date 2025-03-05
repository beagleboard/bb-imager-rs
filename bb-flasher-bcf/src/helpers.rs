use futures::channel::mpsc;

use crate::Status;

pub(crate) fn chan_send(chan: Option<&mut mpsc::Sender<Status>>, msg: Status) {
    if let Some(c) = chan {
        let _ = c.try_send(msg);
    }
}
