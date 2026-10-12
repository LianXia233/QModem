//! Daemon wiring: storage + scheduler + URC worker + delivery worker.
//!
//! Mirrors the C `main.c` runtime shape:
//! * `sync_modem` — one sync pass (hook call) plus a `sync_runs` record.
//! * `urc_sync_worker` — a worker thread consumes URC events, merges
//!   duplicates while one sync is in flight (C: "events arriving while a job
//!   is pending are merged"), detects sequence gaps and triggers sync with
//!   trigger `urc` / `urc_gap`.
//! * delivery worker — claims due deliveries and hands them to a
//!   [`DeliveryHook`]; ack semantics live in [`crate::storage::Storage`].

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::message::ClaimedDelivery;
use crate::scheduler::{unix_now, Scheduler, SchedulerHook};
use crate::storage::Storage;
use crate::urc::{UrcEvent, UrcSource, UrcStream};

/// Modem sync implementation (C side effect of `sync_modem`: run
/// `tom_modem -o r`, import entries). The skeleton ships
/// [`NoopSyncDriver`]; real modem I/O lands with the SQLite port.
pub trait SyncDriver: Send + Sync {
    fn sync(&self, modem_id: &str, trigger: &str) -> u32;
}

/// URC port -> modem section resolver (C: UCI at_port match).
pub type PortRouter = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

/// Default no-op driver: always imports nothing.
pub struct NoopSyncDriver;

impl SyncDriver for NoopSyncDriver {
    fn sync(&self, _modem_id: &str, _trigger: &str) -> u32 {
        0
    }
}

/// Delivery sink (forwarding worker). C equivalent: the Lua/HTTP forwarder
/// that consumes `delivery_claim` results.
pub trait DeliveryHook: Send + Sync {
    /// Return `Ok(())` on success; `Err(why)` schedules a backoff retry.
    fn deliver(&self, delivery: &ClaimedDelivery) -> Result<(), String>;
}

/// Core daemon over a [`Storage`].
pub struct SmsDaemon {
    storage: Arc<dyn Storage>,
    driver: Arc<dyn SyncDriver>,
    /// port -> modem section mapping (C resolves via UCI at_port options).
    port_map: HashMap<String, String>,
    pub(crate) last_error: Mutex<Option<String>>,
}

impl SmsDaemon {
    #[must_use]
    pub fn new(
        storage: Arc<dyn Storage>,
        driver: Arc<dyn SyncDriver>,
        port_map: HashMap<String, String>,
    ) -> Self {
        Self { storage, driver, port_map, last_error: Mutex::new(None) }
    }

    #[must_use]
    pub fn storage(&self) -> &Arc<dyn Storage> {
        &self.storage
    }

    /// One sync pass (C `sync_modem` skeleton): run the driver, then record
    /// the finished run in `sync_runs`.
    pub fn sync_modem(&self, modem_id: &str, trigger: &str) -> u32 {
        let started_at = unix_now();
        let imported = self.driver.sync(modem_id, trigger);
        let error = None::<&str>;
        match self
            .storage
            .record_sync(modem_id, trigger, started_at, imported, error)
        {
            Ok(()) => {
                *self.last_error.lock().expect("daemon poisoned") = None;
            }
            Err(e) => {
                *self.last_error.lock().expect("daemon poisoned") = Some(e.to_string());
            }
        }
        imported
    }

    /// Resolve a URC port to a modem section (C: UCI at_port match).
    #[must_use]
    pub fn resolve_port(&self, port: &str) -> Option<&str> {
        self.port_map.get(port).map(String::as_str)
    }
}

impl SchedulerHook for SmsDaemon {
    fn on_tick_sync(&self, modem_id: &str, trigger: &str) -> u32 {
        self.sync_modem(modem_id, trigger)
    }
}

/// Spawn the scheduler thread on top of this daemon.
pub fn spawn_scheduler(daemon: &Arc<SmsDaemon>, tick: Duration, poll_interval: i64) -> Scheduler {
    let hook: Arc<dyn SchedulerHook> = daemon.clone();
    Scheduler::start(tick, poll_interval, Arc::clone(&daemon.storage), hook)
}

/// URC worker state (C `urc_sync_*` globals).
struct UrcWorker {
    daemon: Arc<SmsDaemon>,
    source: Arc<dyn UrcSource>,
    stop: Arc<AtomicBool>,
    /// Merge flag: when a job is pending, further events are dropped.
    pending: Mutex<bool>,
    router: PortRouter,
}

impl UrcWorker {
    fn spawn(self) -> JoinHandle<()> {
        std::thread::Builder::new()
            .name("smsd-urc".into())
            .spawn(move || self.run())
            .expect("spawn smsd-urc")
    }

    fn run(self) {
        let mut stream: Box<dyn UrcStream> = self.source.subscribe();
        while !self.stop.load(Ordering::SeqCst) {
            match stream.next_event(Duration::from_millis(250)) {
                Ok(Some(event)) => self.ingest(event),
                Ok(None) | Err(crate::urc::UrcRecvError::Timeout) => continue,
                Err(crate::urc::UrcRecvError::Disconnected) => break,
            }
        }
    }

    /// Event intake; mirrors C `urc_event` + `urc_sync_worker` handshake.
    fn ingest(&self, event: UrcEvent) {
        if !event.is_sms_new() {
            return;
        }
        let Some(modem_id) = (self.router)(&event.port) else { return };
        // Merge: drop events while a job is already pending (C comment C10).
        {
            let mut pending = self.pending.lock().expect("urc pending poisoned");
            if *pending {
                return;
            }
            *pending = true;
        }
        // Gap detection + record (C `sms_db_record_event`).
        let gap = match (event.epoch, event.sequence) {
            (Some(epoch), Some(sequence)) => self
                .daemon
                .storage
                .record_event(&modem_id, epoch, sequence)
                .unwrap_or(false),
            _ => false,
        };
        let _ = self.daemon.sync_modem(&modem_id, if gap { "urc_gap" } else { "urc" });
        *self.pending.lock().expect("urc pending poisoned") = false;
    }
}

/// Spawn the URC worker. `router` maps event ports to modem sections.
pub fn spawn_urc_worker(
    daemon: Arc<SmsDaemon>,
    source: Arc<dyn UrcSource>,
    router: PortRouter,
) -> (Arc<AtomicBool>, JoinHandle<()>) {
    let stop = Arc::new(AtomicBool::new(false));
    let worker = UrcWorker {
        daemon,
        source,
        stop: Arc::clone(&stop),
        pending: Mutex::new(false),
        router,
    };
    (stop, worker.spawn())
}

/// One delivery pass: claim due deliveries and hand them to `hook`, acking
/// with success/failure. Returns the number of deliveries attempted.
pub fn run_delivery_pass(storage: &Arc<dyn Storage>, hook: &dyn DeliveryHook) -> usize {
    let now = unix_now();
    let mut attempted = 0;
    while let Some(delivery) = storage.claim_delivery(now).expect("claim") {
        attempted += 1;
        match hook.deliver(&delivery) {
            Ok(()) => {
                storage
                    .complete_delivery(delivery.delivery_id, true, None, now)
                    .expect("ack");
            }
            Err(why) => {
                storage
                    .complete_delivery(delivery.delivery_id, false, Some(&why), now)
                    .expect("ack");
            }
        }
    }
    attempted
}

/// Spawn a looping delivery worker on top of [`run_delivery_pass`].
pub fn spawn_delivery_worker(
    daemon: Arc<SmsDaemon>,
    hook: Arc<dyn DeliveryHook>,
) -> (Arc<AtomicBool>, JoinHandle<()>) {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);
    let handle = std::thread::Builder::new()
        .name("smsd-delivery".into())
        .spawn(move || {
            while !stop_clone.load(Ordering::SeqCst) {
                run_delivery_pass(&daemon.storage, hook.as_ref());
                std::thread::sleep(Duration::from_millis(250));
            }
        })
        .expect("spawn smsd-delivery");
    (stop, handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::import_pdu_segment;
    use crate::storage::InMemoryStorage;
    use std::sync::atomic::AtomicU32;

    const PDU: &str = "0891683108101505F0040B913118986065F9000871218051002323044F60597D";

    struct RecordingDriver {
        calls: AtomicU32,
    }

    impl SyncDriver for RecordingDriver {
        fn sync(&self, _modem_id: &str, _trigger: &str) -> u32 {
            self.calls.fetch_add(1, Ordering::SeqCst);
            0
        }
    }

    struct OkHook;
    impl DeliveryHook for OkHook {
        fn deliver(&self, _delivery: &ClaimedDelivery) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn urc_worker_merges_and_syncs_with_gap() {
        let storage: Arc<dyn Storage> = Arc::new(InMemoryStorage::new());
        let driver = Arc::new(RecordingDriver { calls: AtomicU32::new(0) });
        let mut port_map = HashMap::new();
        port_map.insert("/dev/ttyUSB2".to_string(), "modem_a".to_string());
        let daemon = Arc::new(SmsDaemon::new(Arc::clone(&storage), driver.clone(), port_map));
        let source = Arc::new(crate::urc::MockUrcSource::new());
        let router: PortRouter =
            Arc::new(|port| if port == "/dev/ttyUSB2" { Some("modem_a".into()) } else { None });
        let (stop, handle) = spawn_urc_worker(Arc::clone(&daemon), source.clone(), router);

        // Give the worker time to subscribe before publishing.
        std::thread::sleep(Duration::from_millis(200));
        let publisher = source.publisher();
        let mut ev = UrcEvent {
            port: "/dev/ttyUSB2".into(),
            owner: crate::urc::URC_OWNER_SMS.into(),
            urc_id: crate::urc::URC_ID_SMS_NEW.into(),
            epoch: Some(1),
            sequence: Some(1),
        };
        publisher.publish(ev.clone());
        // Gap: sequence jumps 1 -> 3.
        ev.sequence = Some(3);
        publisher.publish(ev);
        std::thread::sleep(Duration::from_millis(400));
        stop.store(true, Ordering::SeqCst);
        handle.join().unwrap();

        assert!(driver.calls.load(Ordering::SeqCst) >= 2);
        // record_sync ran for both events (worker path).
        assert!(storage.sync_run_count("modem_a") >= 2);
    }

    #[test]
    fn delivery_pass_acks_and_never_reclaims_done() {
        let storage: Arc<dyn Storage> = Arc::new(InMemoryStorage::new());
        import_pdu_segment(storage.as_ref(), "m1", "SM", 1, PDU, 100, 100, true).unwrap();
        let hook = OkHook;
        assert_eq!(run_delivery_pass(&storage, &hook), 1);
        // Done deliveries are not handed out again.
        assert_eq!(run_delivery_pass(&storage, &hook), 0);
        assert_eq!(storage.claim_delivery(200).unwrap(), None);
    }
}
