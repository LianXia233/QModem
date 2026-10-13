//! qmodem-atd: AT command daemon (std::thread actor model, zero external
//! crates).
//!
//! Rust rewrite of `application/qmodem/src/ubus_at_daemon` replacing the C model of
//! "one pthread per port + condition variables + a single
//! `waiting_for_response` slot" with per-port actor threads:
//!
//! * one [`AtPort`](port::spawn_port) actor thread per serial port owns its
//!   transport exclusively (removes the C version's use-after-free class
//!   around `port->fd`, at_handler.c:193-195,307-320);
//! * commands queue on a per-port bounded mpsc channel and execute
//!   strictly serially, each answered on its own oneshot (removes the
//!   single `current_response` slot that concurrent callers overwrite in
//!   each other, at_handler.c:150-158);
//! * timeouts use monotonic `Instant` deadlines bounded by
//!   `recv_timeout`/read slices instead of `CLOCK_REALTIME`
//!   (at_handler.c:208-215);
//! * URC/data lines are published on a hand-written broadcast hub with
//!   `seq`/`epoch` counters mirroring the C line_events queue semantics
//!   (line_events.c / ubus_at_daemon.h:66-89).

pub mod json;
pub mod line;
pub mod port;
pub mod registry;
pub mod transport;
pub mod ubus_proto;

#[cfg(feature = "service")]
pub mod service;

#[cfg(feature = "ubus")]
pub mod ubus_ffi;

pub use json::Value;
pub use line::{AtLine, LineHub, LineReceiver, RecvError};
pub use port::{AtError, AtHandle, AtPortConfig, AtRequest, AtResponse, ResponseStatus};
pub use registry::Registry;
pub use transport::{LoopbackHandle, LoopbackPort, ReadOutcome, SerialPort};
