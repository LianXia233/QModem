//! Core data types shared by the smsd skeleton.
//!
//! Mirrors the C types in `application/qmodem_smsd/src/sms_db.h`
//! (`sms_segment_t`, `sms_import_result_t`) plus the delivery rows used by
//! the `deliveries` table.

/// One received SMS segment as observed on a modem storage slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// Modem UCI section id (C `modem_id`).
    pub modem_id: String,
    /// Storage family, "SM" (SIM) or "ME" (device).
    pub storage: String,
    /// Index inside the modem storage (`CMGL`/`CMGR` index).
    pub source_index: i64,
    /// Raw TPDU hex. Dedup key component; decoded via [`qmodem_pdu::decode`].
    pub pdu: String,
    /// Sender number (may be filled from the PDU when empty).
    pub sender: String,
    /// SCTS as unix seconds.
    pub timestamp: i64,
    /// Decoded text (may be filled from the PDU when empty).
    pub content: String,
    /// Concatenation reference (0 when not multipart).
    pub reference: u16,
    /// Total parts (1 for single-part messages).
    pub total_parts: u8,
    /// Part number, 1-based.
    pub part_number: u8,
}

/// Result of an import, mirroring C `sms_import_result_t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ImportResult {
    /// Same `(modem_id, storage, source_index, pdu)` was already imported.
    pub duplicate: bool,
    /// Source is committed, so the modem slot can be deleted safely.
    pub safe_to_delete: bool,
    /// Message row id when a (new) message was published.
    pub message_id: Option<i64>,
    /// Whether the message became visible to clients at this revision.
    pub published: bool,
    /// Message revision (1 on first publish).
    pub revision: i64,
    /// `source_messages` row id.
    pub source_id: i64,
}

/// A claimed delivery handed to the forwarding worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedDelivery {
    pub delivery_id: i64,
    pub message_id: i64,
    pub revision: i64,
    pub sender: Option<String>,
    pub content: String,
    pub complete: bool,
    pub attempts: u32,
}

/// Storage layer error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    /// Referenced row does not exist.
    NotFound,
    /// State transition is not allowed (e.g. ack on a non-claimed delivery).
    Conflict(&'static str),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::NotFound => write!(f, "row not found"),
            StorageError::Conflict(why) => write!(f, "conflicting state: {why}"),
        }
    }
}

impl std::error::Error for StorageError {}
