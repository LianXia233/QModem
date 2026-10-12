//! Port registry: name -> handle map shared by the daemon and the JSON
//! service. Replaces the C global `g_daemon_ctx.ports` linked list guarded
//! by `ports_mutex` (ubus_at_daemon.h:193-208).

use std::collections::BTreeMap;
use std::sync::RwLock;

use crate::port::AtHandle;

#[derive(Default)]
pub struct Registry {
    inner: RwLock<BTreeMap<String, AtHandle>>,
}

impl Registry {
    pub fn new() -> Registry {
        Registry::default()
    }

    pub fn insert(&self, name: impl Into<String>, handle: AtHandle) {
        self.inner
            .write()
            .unwrap()
            .insert(name.into(), handle);
    }

    pub fn get(&self, name: &str) -> Option<AtHandle> {
        self.inner.read().unwrap().get(name).cloned()
    }

    /// Sorted port names (stable output for the `list` command).
    pub fn list(&self) -> Vec<String> {
        self.inner.read().unwrap().keys().cloned().collect()
    }
}
