//! Line event types shared between the port actors and subscribers.
//!
//! Mirrors the C `at_line_event_t` (ubus_at_daemon.h:66-76): port name,
//! restart epoch, monotonic per-port sequence, and the line itself. The C
//! fixed-capacity ring with `drop_count` becomes a hand-written broadcast
//! hub (`Mutex<Vec<SubQueue>>` + condvar): full subscriber queues drop
//! their OLDEST entry and report the loss via [`RecvError::Lagged`],
//! preserving the tokio-broadcast / `drop_count` semantics without any
//! external crate.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// One data line received from a modem port.
#[derive(Debug, Clone, PartialEq)]
pub struct AtLine {
    /// Port name (e.g. `/dev/ttyUSB2` or a loopback port id).
    pub port: String,
    /// Received line, CR/LF stripped.
    pub line: String,
    /// Per-port monotonic sequence number (matches C `sequence`).
    pub seq: u64,
    /// Port restart epoch; increments each time the port task is recreated
    /// (matches C `restart_epoch` in ubus_at_daemon.h:143).
    pub epoch: u64,
}

/// Error returned by [`LineReceiver::recv`] / [`LineReceiver::recv_timeout`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecvError {
    /// The subscriber queue overflowed and `n` oldest lines were dropped
    /// before this call (C `drop_count` semantics).
    Lagged(u64),
    /// All hub senders are gone (and the buffer is drained).
    Closed,
}

struct SubState {
    buf: VecDeque<AtLine>,
    dropped: u64,
    closed: bool,
}

struct SubQueue {
    state: Mutex<SubState>,
    cv: Condvar,
}

/// Broadcast hub for [`AtLine`] events. Clone-free: share it via `Arc`.
pub struct LineHub {
    subs: Mutex<Vec<Arc<SubQueue>>>,
    capacity: usize,
}

/// Receiving side of one subscription; dropping it unsubscribes.
pub struct LineReceiver {
    q: Arc<SubQueue>,
}

impl LineHub {
    /// Create a hub whose per-subscriber queue holds `capacity` lines.
    pub fn with_capacity(capacity: usize) -> LineHub {
        LineHub {
            subs: Mutex::new(Vec::new()),
            capacity: capacity.max(1),
        }
    }

    /// Register a new subscriber.
    pub fn subscribe(&self) -> LineReceiver {
        let q = Arc::new(SubQueue {
            state: Mutex::new(SubState {
                buf: VecDeque::new(),
                dropped: 0,
                closed: false,
            }),
            cv: Condvar::new(),
        });
        self.subs.lock().unwrap().push(q.clone());
        LineReceiver { q }
    }

    /// Publish a line to every live subscriber. Returns the number of
    /// subscribers the line was delivered to. Full queues lose their
    /// oldest entry (counted, reported as `Lagged` on the next recv).
    pub fn send(&self, line: AtLine) -> usize {
        let mut subs = self.subs.lock().unwrap();
        // Prune subscriptions whose receiver is gone.
        subs.retain(|q| !q.state.lock().unwrap().closed);
        for q in subs.iter() {
            let mut g = q.state.lock().unwrap();
            if g.closed {
                continue;
            }
            if g.buf.len() >= self.capacity {
                g.buf.pop_front();
                g.dropped += 1;
            }
            g.buf.push_back(line.clone());
            drop(g);
            q.cv.notify_all();
        }
        subs.len()
    }

    fn close_all(&mut self) {
        for q in self.subs.get_mut().unwrap().iter() {
            let mut g = q.state.lock().unwrap();
            g.closed = true;
            drop(g);
            q.cv.notify_all();
        }
    }
}

impl Drop for LineHub {
    fn drop(&mut self) {
        self.close_all();
    }
}

impl LineReceiver {
    /// Block until a line is available, the subscription lags, or the hub
    /// (and every sender) is gone.
    pub fn recv(&self) -> Result<AtLine, RecvError> {
        let mut g = self.q.state.lock().unwrap();
        loop {
            if g.dropped > 0 {
                return Err(RecvError::Lagged(std::mem::take(&mut g.dropped)));
            }
            if let Some(l) = g.buf.pop_front() {
                return Ok(l);
            }
            if g.closed {
                return Err(RecvError::Closed);
            }
            g = self.q.cv.wait(g).unwrap();
        }
    }

    /// Like [`recv`](Self::recv) but gives up after `timeout`;
    /// `Ok(None)` means nothing arrived in time.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<Option<AtLine>, RecvError> {
        let deadline = Instant::now() + timeout;
        let mut g = self.q.state.lock().unwrap();
        loop {
            if g.dropped > 0 {
                return Err(RecvError::Lagged(std::mem::take(&mut g.dropped)));
            }
            if let Some(l) = g.buf.pop_front() {
                return Ok(Some(l));
            }
            if g.closed {
                return Err(RecvError::Closed);
            }
            let now = Instant::now();
            if now >= deadline {
                return Ok(None);
            }
            let (ng, _) = self.q.cv.wait_timeout(g, deadline - now).unwrap();
            g = ng;
        }
    }
}

impl Drop for LineReceiver {
    fn drop(&mut self) {
        let mut g = self.q.state.lock().unwrap();
        g.closed = true;
        drop(g);
        self.q.cv.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str) -> AtLine {
        AtLine {
            port: "p0".into(),
            line: text.to_string(),
            seq: 1,
            epoch: 0,
        }
    }

    #[test]
    fn fanout_to_all_subscribers() {
        let hub = LineHub::with_capacity(8);
        let rx1 = hub.subscribe();
        let rx2 = hub.subscribe();
        assert_eq!(hub.send(line("RING")), 2);
        assert_eq!(rx1.recv().unwrap().line, "RING");
        assert_eq!(rx2.recv().unwrap().line, "RING");
    }

    #[test]
    fn lag_reports_dropped_oldest() {
        let hub = LineHub::with_capacity(2);
        let rx = hub.subscribe();
        hub.send(line("a"));
        hub.send(line("b"));
        hub.send(line("c"));
        // Oldest entry dropped, loss reported before the buffered lines.
        assert_eq!(rx.recv(), Err(RecvError::Lagged(1)));
        assert_eq!(rx.recv().unwrap().line, "b");
        assert_eq!(rx.recv().unwrap().line, "c");
        // Queue empty: recv_timeout reports absence instead of blocking.
        assert_eq!(rx.recv_timeout(Duration::from_millis(20)), Ok(None));
    }

    #[test]
    fn closed_when_hub_dropped() {
        let hub = LineHub::with_capacity(4);
        let rx = hub.subscribe();
        hub.send(line("x"));
        drop(hub);
        // Buffered lines drain first, then Closed.
        assert_eq!(rx.recv().unwrap().line, "x");
        assert_eq!(rx.recv(), Err(RecvError::Closed));
    }

    #[test]
    fn dropping_receiver_unsubscribes() {
        let hub = LineHub::with_capacity(4);
        let rx = hub.subscribe();
        assert_eq!(hub.send(line("a")), 1);
        drop(rx);
        assert_eq!(hub.send(line("b")), 0);
    }
}
