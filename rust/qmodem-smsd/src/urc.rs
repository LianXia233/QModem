//! URC event subscription.
//!
//! The C daemon registers a ubus handler for `qmodem.at.urc` events with
//! `owner == "qmodem.sms"` and `urc_id == "sms-new"` (see `urc_event` in
//! `application/qmodem/src/qmodem_smsd/src/main.c`). In the Rust port the same payload
//! arrives through a trait so multiple transports can feed the daemon:
//!
//! * [`MockUrcSource`] — in-process fan-out hub used by tests and the
//!   skeleton (same shape as `qmodem_atd::line::LineHub`).
//! * A future adapter over `qmodem_atd::line::LineHub::subscribe()` (the AT
//!   daemon already publishes URC lines with `port`/`seq`/`epoch`; map each
//!   `AtLine` to an [`UrcEvent`] and hand it to the daemon loop). Documented
//!   in README.md ("URC wiring"); intentionally not implemented here to keep
//!   this crate free of a hard atd dependency.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// C `qmodem.at.urc` owner filter value.
pub const URC_OWNER_SMS: &str = "qmodem.sms";
/// C `urc_id` value signalling new SMS.
pub const URC_ID_SMS_NEW: &str = "sms-new";

/// One `qmodem.at.urc` event payload (subset the daemon consumes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrcEvent {
    /// Serial port the URC arrived on (maps to a modem section).
    pub port: String,
    pub owner: String,
    pub urc_id: String,
    /// `restart_epoch` when the publisher tracks restarts.
    pub epoch: Option<u64>,
    /// Monotonic per-port sequence number.
    pub sequence: Option<u64>,
}

impl UrcEvent {
    #[must_use]
    pub fn is_sms_new(&self) -> bool {
        self.owner == URC_OWNER_SMS && self.urc_id == URC_ID_SMS_NEW
    }
}

/// Receive errors on a URC stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrcRecvError {
    /// Timed out without an event; not fatal.
    Timeout,
    /// Publisher went away; stream is dead.
    Disconnected,
}

/// A stream of URC events.
pub trait UrcStream: Send {
    fn next_event(&mut self, timeout: Duration) -> Result<Option<UrcEvent>, UrcRecvError>;
}

/// Factory for URC streams (one per daemon worker).
pub trait UrcSource: Send + Sync {
    fn subscribe(&self) -> Box<dyn UrcStream>;
}

struct Hub {
    /// Live subscriber senders; closed receivers are dropped lazily.
    subscribers: Mutex<Vec<Sender<UrcEvent>>>,
}

impl Hub {
    fn publish(&self, event: UrcEvent) -> usize {
        let mut subs = self.subscribers.lock().expect("urc hub poisoned");
        subs.retain(|tx| tx.send(event.clone()).is_ok());
        subs.len()
    }
}

/// Fan-out [`UrcSource`] used by tests and the skeleton wiring. Mirrors the
/// broadcast semantics of `qmodem_atd::line::LineHub`: every subscriber sees
/// every event published after it subscribed.
pub struct MockUrcSource {
    hub: Arc<Hub>,
}

impl MockUrcSource {
    #[must_use]
    pub fn new() -> Self {
        Self { hub: Arc::new(Hub { subscribers: Mutex::new(Vec::new()) }) }
    }

    /// Publish one event to all live subscribers.
    pub fn publisher(&self) -> MockUrcPublisher<'_> {
        MockUrcPublisher { hub: &self.hub }
    }
}

impl Default for MockUrcSource {
    fn default() -> Self {
        Self::new()
    }
}

impl UrcSource for MockUrcSource {
    fn subscribe(&self) -> Box<dyn UrcStream> {
        let (tx, rx) = mpsc::channel();
        self.hub.subscribers.lock().expect("urc hub poisoned").push(tx);
        Box::new(ChannelUrcStream { rx })
    }
}

/// Publish handle for a [`MockUrcSource`].
pub struct MockUrcPublisher<'a> {
    hub: &'a Hub,
}

impl MockUrcPublisher<'_> {
    /// Send one event; returns the number of live receivers it reached.
    pub fn publish(&self, event: UrcEvent) -> usize {
        self.hub.publish(event)
    }
}

struct ChannelUrcStream {
    rx: Receiver<UrcEvent>,
}

impl UrcStream for ChannelUrcStream {
    fn next_event(&mut self, timeout: Duration) -> Result<Option<UrcEvent>, UrcRecvError> {
        match self.rx.recv_timeout(timeout) {
            Ok(event) => Ok(Some(event)),
            Err(RecvTimeoutError::Timeout) => Err(UrcRecvError::Timeout),
            Err(RecvTimeoutError::Disconnected) => Err(UrcRecvError::Disconnected),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(seq: u64) -> UrcEvent {
        UrcEvent {
            port: "/dev/ttyUSB2".into(),
            owner: URC_OWNER_SMS.into(),
            urc_id: URC_ID_SMS_NEW.into(),
            epoch: Some(1),
            sequence: Some(seq),
        }
    }

    #[test]
    fn fanout_reaches_subscriber() {
        let source = MockUrcSource::new();
        let mut stream = source.subscribe();
        assert_eq!(source.publisher().publish(event(1)), 1);
        let got = stream.next_event(Duration::from_secs(1)).unwrap().unwrap();
        assert!(got.is_sms_new());
        assert_eq!(got.sequence, Some(1));
    }

    #[test]
    fn other_owners_are_filtered_by_is_sms_new() {
        let mut foreign = event(1);
        foreign.owner = "other.owner".into();
        assert!(!foreign.is_sms_new());
    }
}
