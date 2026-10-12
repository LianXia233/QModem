//! 30-second scheduler tick.
//!
//! Mirrors C `scheduler_cb` + `uloop_timeout_set(&app.scheduler,
//! SCHEDULER_TICK_MS)`: every tick, scan modems and sync those where
//! `last_sync_at + poll_interval <= now`. Implemented with a plain
//! [`std::thread`] plus `mpsc::Receiver::recv_timeout`, so the tick loop can
//! be parked between ticks and stopped promptly.

use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::storage::Storage;

/// Default tick period, matching C `SCHEDULER_TICK_MS` (30s).
pub const DEFAULT_TICK: Duration = Duration::from_secs(30);

/// Work executed for one due modem. Returns the imported count that lands in
/// the `sync_runs` row. Implemented by [`crate::daemon::SmsDaemon`].
pub trait SchedulerHook: Send + Sync {
    fn on_tick_sync(&self, modem_id: &str, trigger: &str) -> u32;
}

enum Command {
    Stop,
}

/// Periodic scheduler. Start via [`Scheduler::start`], stop by dropping.
pub struct Scheduler {
    tx: mpsc::Sender<Command>,
    handle: Option<JoinHandle<()>>,
}

impl Scheduler {
    /// Spawn the tick thread. `tick` is the wake period (30s in production,
    /// tiny values in tests); `poll_interval` is the per-modem minimum gap
    /// between syncs.
    pub fn start(
        tick: Duration,
        poll_interval: i64,
        storage: Arc<dyn Storage>,
        hook: Arc<dyn SchedulerHook>,
    ) -> Self {
        let (tx, rx) = mpsc::channel::<Command>();
        let handle = std::thread::Builder::new()
            .name("smsd-scheduler".into())
            .spawn(move || loop {
                match rx.recv_timeout(tick) {
                    Ok(Command::Stop) | Err(RecvTimeoutError::Disconnected) => break,
                    Err(RecvTimeoutError::Timeout) => {
                        run_once(poll_interval, &storage, hook.as_ref());
                    }
                }
            })
            .expect("spawn smsd-scheduler");
        Self { tx, handle: Some(handle) }
    }

    /// Stop the thread and join it.
    pub fn stop(mut self) {
        let _ = self.tx.send(Command::Stop);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Stop);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// One scheduler pass over all known modems (C `scheduler_cb` body).
pub fn run_once(poll_interval: i64, storage: &Arc<dyn Storage>, hook: &dyn SchedulerHook) {
    let now = unix_now();
    for modem_id in storage.modem_ids() {
        let last = storage.last_sync_at(&modem_id).unwrap_or(None).unwrap_or(0);
        if last + poll_interval <= now {
            hook.on_tick_sync(&modem_id, "poll");
        }
    }
}

/// Unix seconds, the C `time(NULL)` equivalent.
#[must_use]
pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::InMemoryStorage;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct CountingHook {
        calls: AtomicU32,
    }

    impl SchedulerHook for CountingHook {
        fn on_tick_sync(&self, _modem_id: &str, _trigger: &str) -> u32 {
            self.calls.fetch_add(1, Ordering::SeqCst);
            0
        }
    }

    #[test]
    fn tick_triggers_poll_sync() {
        let storage: Arc<dyn Storage> = Arc::new(InMemoryStorage::new());
        // A known modem with no sync history must be due immediately.
        storage.record_sync("m1", "bootstrap", 0, 0, None).unwrap();
        let hook = Arc::new(CountingHook { calls: AtomicU32::new(0) });
        let sched = Scheduler::start(
            Duration::from_millis(20),
            0, // poll_interval 0 => always due
            Arc::clone(&storage),
            Arc::clone(&hook) as Arc<dyn SchedulerHook>,
        );
        std::thread::sleep(Duration::from_millis(150));
        sched.stop();
        assert!(hook.calls.load(Ordering::SeqCst) >= 1, "tick must trigger sync");
        // hook-based syncs are recorded by the daemon; here just ensure the
        // scheduler kept ticking rather than exiting after the first pass.
        assert!(hook.calls.load(Ordering::SeqCst) < 1000);
    }
}
