//! Storage abstraction.
//!
//! [`Storage`] captures exactly the persistence operations the C daemon
//! performs against `sms.sqlite3` (see `application/qmodem/src/qmodem_smsd/src/sms_db.c`):
//! deduplicated imports keyed by `(modem_id, storage, source_index, pdu)`,
//! `sync_runs` bookkeeping, URC epoch/sequence tracking and the
//! `deliveries` claim/ack state machine.
//!
//! [`InMemoryStorage`] is the reference implementation used by tests and as
//! a drop-in for skeleton deployments. A real SQLite implementation should
//! implement the same trait and be handed to the daemon as
//! `Arc<dyn Storage>`; the on-disk schema and WAL settings are documented in
//! README.md ("SQLite roadmap").

use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::message::{ClaimedDelivery, ImportResult, Segment, StorageError};

/// Seconds after which a stale claim is reclaimed (C: `claimed_at < now-300`).
pub const CLAIM_TIMEOUT_SECS: i64 = 300;
/// Failure backoff base (C: `30*(1<<MIN(attempts,7))`, capped at 3600).
pub const BACKOFF_BASE_SECS: i64 = 30;
/// Failure backoff cap (C: `MIN(3600, ...)`).
pub const BACKOFF_MAX_SECS: i64 = 3600;

/// Persistence contract shared by the in-memory reference implementation and
/// the future SQLite implementation.
pub trait Storage: Send + Sync {
    /// Import one segment with dedup. Mirrors C `sms_db_import_segment`.
    ///
    /// When `forwarding` is set and a message is published, a delivery row is
    /// queued for the forwarding worker (C `queue_delivery`).
    fn import_segment(
        &self,
        segment: &Segment,
        now: i64,
        forwarding: bool,
    ) -> Result<ImportResult, StorageError>;

    /// Publish multipart groups whose wait window expired, mirroring C
    /// `sms_db_publish_expired`: incomplete groups older than
    /// `multipart_wait_secs` are published with `complete=0`.
    fn publish_expired(
        &self,
        modem_id: &str,
        now: i64,
        multipart_wait_secs: i64,
        forwarding: bool,
    ) -> Result<u32, StorageError>;

    /// Record one finished sync run (C `sms_db_record_sync`): appends to
    /// `sync_runs` and bumps `modems.last_sync_at`.
    fn record_sync(
        &self,
        modem_id: &str,
        trigger: &str,
        started_at: i64,
        imported: u32,
        error: Option<&str>,
    ) -> Result<(), StorageError>;

    /// Last successful sync time per modem (drives the scheduler decision
    /// `last_sync_at + poll_interval <= now`, C `scheduler_cb`).
    fn last_sync_at(&self, modem_id: &str) -> Result<Option<i64>, StorageError>;

    /// Track a URC event and report a gap (C `sms_db_record_event`):
    /// gap when a previous epoch exists and either the epoch changed or the
    /// sequence is not exactly `previous + 1`.
    fn record_event(&self, modem_id: &str, epoch: u64, sequence: u64)
        -> Result<bool, StorageError>;

    /// Claim the oldest due pending delivery (C `delivery_claim`):
    /// first reclaim stale claims, then take `state='pending' AND
    /// available_at<=now ORDER BY id LIMIT 1` and mark it claimed with
    /// `attempts+1`.
    fn claim_delivery(&self, now: i64) -> Result<Option<ClaimedDelivery>, StorageError>;

    /// Acknowledge a claimed delivery (C `delivery_complete`): success marks
    /// it done; failure returns it to pending with exponential backoff.
    fn complete_delivery(
        &self,
        delivery_id: i64,
        success: bool,
        error: Option<&str>,
        now: i64,
    ) -> Result<(), StorageError>;

    /// Known modem ids (scheduler scan set).
    fn modem_ids(&self) -> Vec<String>;

    /// Number of published messages for a modem (test/observation helper).
    fn message_count(&self, modem_id: &str) -> u64;

    /// Number of finished sync runs for a modem (test/observation helper).
    fn sync_run_count(&self, modem_id: &str) -> u64;
}

#[derive(Debug, Default, Clone)]
struct ModemState {
    last_sync_at: i64,
    last_event_epoch: u64,
    last_event_sequence: u64,
}

#[derive(Debug, Clone)]
struct SourceRow {
    committed: bool,
    imported_at: i64,
}

#[derive(Debug, Clone)]
struct PartEntry {
    content: String,
    timestamp: i64,
}

#[derive(Debug, Clone)]
struct GroupState {
    modem_id: String,
    sender: String,
    reference: u16,
    total_parts: u8,
    first_seen: i64,
    last_seen: i64,
    parts: BTreeMap<u8, PartEntry>,
    published_message: Option<i64>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // row fields mirror the C `messages` table for the SQLite port
struct MessageRow {
    id: i64,
    modem_id: String,
    direction: &'static str,
    sender: Option<String>,
    recipient: Option<String>,
    timestamp: i64,
    content: String,
    complete: bool,
    revision: i64,
}

#[derive(Debug, Clone)]
struct DeliveryRow {
    id: i64,
    message_id: i64,
    revision: i64,
    /// "pending" | "claimed" | "done"
    state: &'static str,
    attempts: u32,
    available_at: i64,
    claimed_at: Option<i64>,
    last_error: Option<String>,
}

#[derive(Debug)]
struct Inner {
    modems: BTreeMap<String, ModemState>,
    /// Dedup key `(modem_id, storage, source_index, pdu)` -> row.
    sources: BTreeMap<(String, String, i64, String), SourceRow>,
    groups: BTreeMap<i64, GroupState>,
    next_group_id: i64,
    messages: BTreeMap<i64, MessageRow>,
    next_message_id: i64,
    deliveries: BTreeMap<i64, DeliveryRow>,
    next_delivery_id: i64,
    sync_runs: Vec<(String, String, i64, i64, u32, Option<String>)>,
}

/// Thread-safe in-memory [`Storage`] reference implementation.
pub struct InMemoryStorage {
    inner: Mutex<Inner>,
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                modems: BTreeMap::new(),
                sources: BTreeMap::new(),
                groups: BTreeMap::new(),
                next_group_id: 1,
                messages: BTreeMap::new(),
                next_message_id: 1,
                deliveries: BTreeMap::new(),
                next_delivery_id: 1,
                sync_runs: Vec::new(),
            }),
        }
    }

    fn ensure_modem(inner: &mut Inner, modem_id: &str) {
        inner.modems.entry(modem_id.to_string()).or_default();
    }

    #[allow(clippy::too_many_arguments)] // flat mirror of the messages table columns
    fn insert_message(
        inner: &mut Inner,
        modem_id: &str,
        sender: &str,
        timestamp: i64,
        content: &str,
        complete: bool,
        now: i64,
        forwarding: bool,
    ) -> (i64, i64) {
        let id = inner.next_message_id;
        inner.next_message_id += 1;
        inner.messages.insert(
            id,
            MessageRow {
                id,
                modem_id: modem_id.to_string(),
                direction: "received",
                sender: Some(sender.to_string()),
                recipient: None,
                timestamp,
                content: content.to_string(),
                complete,
                revision: 1,
            },
        );
        if forwarding {
            let delivery_id = inner.next_delivery_id;
            inner.next_delivery_id += 1;
            inner.deliveries.insert(
                delivery_id,
                DeliveryRow {
                    id: delivery_id,
                    message_id: id,
                    revision: 1,
                    state: "pending",
                    attempts: 0,
                    available_at: now,
                    claimed_at: None,
                    last_error: None,
                },
            );
        }
        (id, 1)
    }

    fn publish_group(
        inner: &mut Inner,
        group_id: i64,
        now: i64,
        forwarding: bool,
        result: &mut ImportResult,
    ) {
        let (modem_id, sender, total, parts) = {
            let group = &inner.groups[&group_id];
            (
                group.modem_id.clone(),
                group.sender.clone(),
                group.total_parts,
                group.parts.clone(),
            )
        };
        // Part numbers are 1-based; a complete group holds parts 1..=total.
        let complete = (1..=total).all(|p| parts.contains_key(&p));
        let mut text = String::new();
        for p in 1..=total {
            if let Some(part) = parts.get(&p) {
                text.push_str(&part.content);
            }
        }
        let timestamp = parts.values().map(|p| p.timestamp).min().unwrap_or(now);
        let (id, revision) = Self::insert_message(
            inner,
            &modem_id,
            &sender,
            timestamp,
            &text,
            complete,
            now,
            forwarding,
        );
        if let Some(group) = inner.groups.get_mut(&group_id) {
            group.published_message = Some(id);
        }
        result.message_id = Some(id);
        result.published = true;
        result.revision = revision;
    }
}

impl Storage for InMemoryStorage {
    fn import_segment(
        &self,
        segment: &Segment,
        now: i64,
        forwarding: bool,
    ) -> Result<ImportResult, StorageError> {
        let mut inner = self.inner.lock().expect("smsd storage poisoned");
        let mut result = ImportResult::default();
        Self::ensure_modem(&mut inner, &segment.modem_id);

        let key = (
            segment.modem_id.clone(),
            segment.storage.clone(),
            segment.source_index,
            segment.pdu.clone(),
        );
        if let Some(row) = inner.sources.get_mut(&key) {
            // Dedup: refresh imported_at only, never touch messages.
            result.duplicate = true;
            result.source_id = 0;
            result.safe_to_delete = row.committed;
            row.imported_at = now;
            return Ok(result);
        }
        inner
            .sources
            .entry(key)
            .or_insert(SourceRow { committed: false, imported_at: now });
        result.source_id = 0;

        if segment.total_parts > 1 {
            let group_id = {
                let mut found = None;
                for (id, group) in inner.groups.iter() {
                    if group.modem_id == segment.modem_id
                        && group.sender == segment.sender
                        && group.reference == segment.reference
                        && group.total_parts == segment.total_parts
                        && group.published_message.is_none()
                        && now.saturating_sub(group.first_seen)
                            <= crate::storage::LATE_FRAGMENT_WINDOW_SECS
                    {
                        found = Some(*id);
                        break;
                    }
                }
                found.unwrap_or_else(|| {
                    let id = inner.next_group_id;
                    inner.next_group_id += 1;
                    inner.groups.insert(
                        id,
                        GroupState {
                            modem_id: segment.modem_id.clone(),
                            sender: segment.sender.clone(),
                            reference: segment.reference,
                            total_parts: segment.total_parts,
                            first_seen: now,
                            last_seen: now,
                            parts: BTreeMap::new(),
                            published_message: None,
                        },
                    );
                    id
                })
            };
            let group = inner.groups.get_mut(&group_id).expect("group just created");
            group.parts.insert(
                segment.part_number,
                PartEntry {
                    content: segment.content.clone(),
                    timestamp: segment.timestamp,
                },
            );
            group.last_seen = now;
            let count = group.parts.len();
            let total = group.total_parts;
            if count >= total as usize {
                Self::publish_group(&mut inner, group_id, now, forwarding, &mut result);
            }
        } else {
            let (id, revision) = Self::insert_message(
                &mut inner,
                &segment.modem_id,
                &segment.sender,
                segment.timestamp,
                &segment.content,
                true,
                now,
                forwarding,
            );
            result.message_id = Some(id);
            result.published = true;
            result.revision = revision;
        }

        inner
            .sources
            .entry((
                segment.modem_id.clone(),
                segment.storage.clone(),
                segment.source_index,
                segment.pdu.clone(),
            ))
            .and_modify(|row| {
                row.committed = true;
            });
        result.safe_to_delete = true;
        Ok(result)
    }

    fn publish_expired(
        &self,
        modem_id: &str,
        now: i64,
        multipart_wait_secs: i64,
        forwarding: bool,
    ) -> Result<u32, StorageError> {
        let mut inner = self.inner.lock().expect("smsd storage poisoned");
        let mut published = 0;
        let ids: Vec<i64> = inner
            .groups
            .iter()
            .filter(|(_, g)| {
                g.modem_id == modem_id
                    && g.published_message.is_none()
                    && g.first_seen <= now - multipart_wait_secs
            })
            .map(|(id, _)| *id)
            .collect();
        for id in ids {
            let mut result = ImportResult::default();
            Self::publish_group(&mut inner, id, now, forwarding, &mut result);
            if result.published {
                published += 1;
            }
        }
        Ok(published)
    }

    fn record_sync(
        &self,
        modem_id: &str,
        trigger: &str,
        started_at: i64,
        imported: u32,
        error: Option<&str>,
    ) -> Result<(), StorageError> {
        let mut inner = self.inner.lock().expect("smsd storage poisoned");
        Self::ensure_modem(&mut inner, modem_id);
        inner.sync_runs.push((
            modem_id.to_string(),
            trigger.to_string(),
            started_at,
            started_at,
            imported,
            error.map(str::to_string),
        ));
        inner.modems.get_mut(modem_id).unwrap().last_sync_at = started_at;
        Ok(())
    }

    fn last_sync_at(&self, modem_id: &str) -> Result<Option<i64>, StorageError> {
        let inner = self.inner.lock().expect("smsd storage poisoned");
        Ok(inner.modems.get(modem_id).map(|m| m.last_sync_at).filter(|v| *v > 0))
    }

    fn record_event(
        &self,
        modem_id: &str,
        epoch: u64,
        sequence: u64,
    ) -> Result<bool, StorageError> {
        let mut inner = self.inner.lock().expect("smsd storage poisoned");
        Self::ensure_modem(&mut inner, modem_id);
        let state = inner.modems.get_mut(modem_id).unwrap();
        let gap = state.last_event_epoch != 0
            && (epoch != state.last_event_epoch || sequence != state.last_event_sequence + 1);
        state.last_event_epoch = epoch;
        state.last_event_sequence = sequence;
        Ok(gap)
    }

    fn claim_delivery(&self, now: i64) -> Result<Option<ClaimedDelivery>, StorageError> {
        let mut inner = self.inner.lock().expect("smsd storage poisoned");
        // Reclaim stale claims first (C: claimed_at < now-300).
        for row in inner.deliveries.values_mut() {
            if row.state == "claimed" {
                let stale = row.claimed_at.unwrap_or(0) < now - CLAIM_TIMEOUT_SECS;
                if stale {
                    row.state = "pending";
                    row.claimed_at = None;
                    row.available_at = now;
                }
            }
        }
        let candidate = inner
            .deliveries
            .values()
            .filter(|row| row.state == "pending" && row.available_at <= now)
            .min_by_key(|row| row.id)
            .map(|row| row.id);
        let Some(id) = candidate else { return Ok(None) };
        let (message_id, revision, attempts) = {
            let row = inner.deliveries.get_mut(&id).expect("candidate exists");
            row.state = "claimed";
            row.attempts += 1;
            row.claimed_at = Some(now);
            (row.message_id, row.revision, row.attempts)
        };
        let message = &inner.messages[&message_id];
        Ok(Some(ClaimedDelivery {
            delivery_id: id,
            message_id,
            revision,
            sender: message.sender.clone(),
            content: message.content.clone(),
            complete: message.complete,
            attempts,
        }))
    }

    fn complete_delivery(
        &self,
        delivery_id: i64,
        success: bool,
        error: Option<&str>,
        now: i64,
    ) -> Result<(), StorageError> {
        let mut inner = self.inner.lock().expect("smsd storage poisoned");
        let Some(row) = inner.deliveries.get_mut(&delivery_id) else {
            return Err(StorageError::NotFound);
        };
        if row.state != "claimed" {
            return Err(StorageError::Conflict("delivery is not claimed"));
        }
        if success {
            row.state = "done";
        } else {
            // C: available_at = now + MIN(3600, 30*(1<<MIN(attempts,7)))
            let shift = row.attempts.min(7);
            let backoff = BACKOFF_BASE_SECS
                .saturating_mul(1i64 << shift)
                .min(BACKOFF_MAX_SECS);
            row.state = "pending";
            row.available_at = now + backoff;
            row.last_error = error.map(str::to_string);
        }
        Ok(())
    }

    fn modem_ids(&self) -> Vec<String> {
        let inner = self.inner.lock().expect("smsd storage poisoned");
        inner.modems.keys().cloned().collect()
    }

    fn message_count(&self, modem_id: &str) -> u64 {
        let inner = self.inner.lock().expect("smsd storage poisoned");
        inner
            .messages
            .values()
            .filter(|m| m.modem_id == modem_id)
            .count() as u64
    }

    fn sync_run_count(&self, modem_id: &str) -> u64 {
        let inner = self.inner.lock().expect("smsd storage poisoned");
        inner.sync_runs.iter().filter(|r| r.0 == modem_id).count() as u64
    }
}

/// C: `late_fragment_window` default (3600s) used when grouping fragments.
pub const LATE_FRAGMENT_WINDOW_SECS: i64 = 3600;

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(index: i64, pdu: &str, content: &str) -> Segment {
        Segment {
            modem_id: "modem_a".into(),
            storage: "SM".into(),
            source_index: index,
            pdu: pdu.into(),
            sender: "+8613800000000".into(),
            timestamp: 1_700_000_000,
            content: content.into(),
            reference: 0,
            total_parts: 1,
            part_number: 1,
        }
    }

    #[test]
    fn duplicate_import_is_idempotent() {
        let db = InMemoryStorage::new();
        let first = db.import_segment(&segment(1, "ABCD", "hello"), 100, false).unwrap();
        assert!(!first.duplicate);
        assert!(first.published);
        let second = db.import_segment(&segment(1, "ABCD", "hello"), 200, false).unwrap();
        assert!(second.duplicate);
        assert!(second.safe_to_delete);
        assert_eq!(db.message_count("modem_a"), 1);
    }

    #[test]
    fn failed_ack_backs_off_before_reclaim() {
        let db = InMemoryStorage::new();
        let seg = segment(1, "FF00", "queued");
        db.import_segment(&seg, 100, true).unwrap();
        let claimed = db.claim_delivery(100).unwrap().expect("claim");
        assert_eq!(claimed.attempts, 1);
        db.complete_delivery(claimed.delivery_id, false, Some("down"), 100)
            .unwrap();
        // Backoff window: not claimable right away.
        assert!(db.claim_delivery(110).unwrap().is_none());
        // After backoff it becomes claimable again with attempts+1.
        let again = db.claim_delivery(100 + 30 * 2).unwrap().expect("reclaim");
        assert_eq!(again.attempts, 2);
    }

    #[test]
    fn stale_claim_is_reclaimed() {
        let db = InMemoryStorage::new();
        db.import_segment(&segment(1, "EE01", "x"), 100, true).unwrap();
        let claimed = db.claim_delivery(100).unwrap().unwrap();
        // Another claim before the timeout finds nothing.
        assert!(db.claim_delivery(100 + 60).unwrap().is_none());
        // After CLAIM_TIMEOUT_SECS the claim is handed out again.
        let reclaimed = db.claim_delivery(100 + CLAIM_TIMEOUT_SECS + 1).unwrap().unwrap();
        assert_eq!(reclaimed.delivery_id, claimed.delivery_id);
        assert_eq!(reclaimed.attempts, 2);
    }
}
