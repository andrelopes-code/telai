use std::net::SocketAddr;

use bytes::Bytes;
use tokio::sync::{broadcast, watch};

use crate::protocol::{self, Status};

/// About 640 ms of 10 ms capture packets. A viewer that falls further behind
/// skips ahead instead of accumulating latency.
const AUDIO_BACKLOG: usize = 64;

/// Fan-out point between the capture threads and the connected viewers.
///
/// Video keeps only the latest frame, so a slow viewer drops frames instead of
/// falling behind. Audio keeps a short backlog, because every gap is audible.
#[derive(Clone)]
pub struct Hub {
    video: watch::Sender<Option<Bytes>>,
    audio: broadcast::Sender<Bytes>,
    status: watch::Sender<Status>,
    viewers: watch::Sender<Vec<SocketAddr>>,
}

pub(crate) struct Subscription {
    pub video: watch::Receiver<Option<Bytes>>,
    pub audio: broadcast::Receiver<Bytes>,
    pub status: watch::Receiver<Status>,
}

impl Hub {
    #[must_use]
    pub fn new() -> Self {
        Self {
            video: watch::Sender::new(None),
            audio: broadcast::Sender::new(AUDIO_BACKLOG),
            status: watch::Sender::new(Status::default()),
            viewers: watch::Sender::new(Vec::new()),
        }
    }

    pub fn publish_video(&self, packet: Bytes) {
        self.video.send_replace(Some(packet));
    }

    pub fn publish_audio(&self, samples: &[f32]) {
        if self.audio.receiver_count() > 0 {
            // An error only means that every viewer left in the meantime.
            let _ = self.audio.send(protocol::audio_packet(samples));
        }
    }

    pub fn set_status(&self, status: Status) {
        if !status.live {
            self.video.send_replace(None);
        }

        self.status.send_replace(status);
    }

    /// Addresses of the connected viewers.
    #[must_use]
    pub fn viewers(&self) -> watch::Receiver<Vec<SocketAddr>> {
        self.viewers.subscribe()
    }

    pub(crate) fn subscribe(&self) -> Subscription {
        let mut video = self.video.subscribe();
        video.mark_changed();

        Subscription {
            video,
            audio: self.audio.subscribe(),
            status: self.status.subscribe(),
        }
    }

    #[cfg(test)]
    pub(crate) fn latest_video(&self) -> Option<Bytes> {
        self.video.borrow().clone()
    }

    pub(crate) fn add_viewer(&self, address: SocketAddr) {
        self.viewers.send_modify(|viewers| viewers.push(address));
    }

    pub(crate) fn remove_viewer(&self, address: SocketAddr) {
        self.viewers
            .send_modify(|viewers| viewers.retain(|viewer| *viewer != address));
    }
}

impl Default for Hub {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_subscription_sees_the_latest_frame() {
        let hub = Hub::new();
        hub.publish_video(Bytes::from_static(b"frame"));

        let subscription = hub.subscribe();

        assert!(subscription.video.has_changed().unwrap());
        assert_eq!(subscription.video.borrow().as_deref(), Some(&b"frame"[..]));
    }

    #[test]
    fn going_offline_clears_the_latest_frame() {
        let hub = Hub::new();
        hub.publish_video(Bytes::from_static(b"frame"));

        hub.set_status(Status::default());

        assert!(hub.subscribe().video.borrow().is_none());
    }

    #[test]
    fn viewers_are_tracked_by_address() {
        let hub = Hub::new();
        let first: SocketAddr = "127.0.0.1:50001".parse().unwrap();
        let second: SocketAddr = "127.0.0.1:50002".parse().unwrap();

        hub.add_viewer(first);
        hub.add_viewer(second);
        hub.remove_viewer(first);

        assert_eq!(*hub.viewers().borrow(), vec![second]);
    }
}
