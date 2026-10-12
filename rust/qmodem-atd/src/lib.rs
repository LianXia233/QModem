//! qmodem-atd: async AT command daemon.
//!
//! Rust rewrite of `application/ubus_at_daemon` replacing the C model of
//! "one pthread per port + condition variables + a single
//! `waiting_for_response` slot" with tokio tasks:
//!
//! * one [`AtPort`] actor task per serial port owns its transport
//!   exclusively (removes the C version's use-after-free class around
//!   `port->fd`, at_handler.c:193-195,307-320);
//! * commands queue on a per-port mpsc channel and execute strictly
//!   serially, each answered on its own oneshot (removes the single
//!   `current_response` slot that concurrent callers overwrite in each
//!   other, at_handler.c:150-158);
//! * timeouts use `tokio::time::timeout`'s monotonic clock instead of
//!   `CLOCK_REALTIME` (at_handler.c:208-215);
//! * URC/data lines are published on a broadcast channel with
//!   `seq`/`epoch` counters mirroring the C line_events queue semantics
//!   (line_events.c / ubus_at_daemon.h:66-89).

pub mod line;
pub mod port;
pub mod registry;
pub mod transport;

#[cfg(feature = "service")]
pub mod service;

#[cfg(feature = "ubus")]
pub mod ubus_ffi;

pub use line::AtLine;
pub use port::{AtError, AtHandle, AtRequest, AtPortConfig, ResponseStatus, AtResponse};
pub use registry::Registry;
pub use transport::{LoopbackHandle, LoopbackPort, SerialPort};
