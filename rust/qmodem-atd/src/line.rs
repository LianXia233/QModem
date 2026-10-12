//! Line event types shared between the port actors and subscribers.
//!
//! Mirrors the C `at_line_event_t` (ubus_at_daemon.h:66-76): port name,
//! restart epoch, monotonic per-port sequence, and the line itself. The C
//! fixed-capacity ring with `drop_count` becomes a tokio broadcast channel
//! that reports lag via `RecvError::Lagged`.

/// One data line received from a modem port.
#[derive(Debug, Clone)]
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
