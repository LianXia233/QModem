//! qmodem-smsd: Rust skeleton of the SMS daemon
//! (`application/qmodem_smsd`, the C `qmodem_smsd` binary).
//!
//! Scope of this skeleton (see README.md for the full C/Rust parity table):
//! * message import with idempotent dedup, PDU decoding via [`qmodem_pdu`]
//!   ([`import`], [`storage`]);
//! * send/forward queue with `claim`/`ack` semantics matching the C
//!   `delivery_claim`/`delivery_complete` ubus methods ([`storage`],
//!   [`daemon::run_delivery_pass`]);
//! * 30s scheduler tick via `std::thread` + `recv_timeout`
//!   ([`scheduler`], C `scheduler_cb`);
//! * URC event subscription trait for wiring `qmodem-atd` URC lines
//!   ([`urc`], C `qmodem.at.urc` handler).
//!
//! Not yet ported (documented in README.md): ubus object surface, SQLite
//! persistence (behind the [`storage::Storage`] trait), UCI configuration,
//! legacy migration, multipart `send_managed` encoding.

pub mod daemon;
pub mod import;
pub mod message;
pub mod scheduler;
pub mod storage;
pub mod urc;

pub use daemon::{
    run_delivery_pass, spawn_delivery_worker, spawn_scheduler, spawn_urc_worker, DeliveryHook,
    NoopSyncDriver, SmsDaemon, SyncDriver,
};
pub use import::{decode_pdu_hex, import_pdu_segment, segment_from_pdu, ImportError};
pub use message::{ClaimedDelivery, ImportResult, Segment, StorageError};
pub use scheduler::{unix_now, Scheduler, SchedulerHook, DEFAULT_TICK};
pub use storage::{InMemoryStorage, Storage};
pub use urc::{MockUrcSource, UrcEvent, UrcRecvError, UrcSource, UrcStream};
