//! PDU-driven import path.
//!
//! The C daemon receives `{"index","pdu","sender","timestamp",...}` entries
//! from `tom_modem -o r` and uses the raw PDU only as a dedup key component.
//! In the Rust port, PDU decoding is done locally via
//! [`qmodem_pdu::decode`], so `sender`/`content`/multipart layout can be
//! derived from the PDU itself instead of trusting external JSON.

use qmodem_pdu::{decode, SmsDeliver};

use crate::message::{ImportResult, Segment, StorageError};
use crate::storage::Storage;

/// Import error combining PDU decoding and storage failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    /// Hex string is not valid ASCII hex.
    BadHex,
    /// PDU failed to decode.
    Pdu(qmodem_pdu::PduError),
    /// Storage rejected the import.
    Storage(StorageError),
    /// Decoded concat header is out of range.
    BadMultipart,
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::BadHex => write!(f, "pdu hex is not valid"),
            ImportError::Pdu(e) => write!(f, "pdu decode failed: {e}"),
            ImportError::Storage(e) => write!(f, "storage import failed: {e}"),
            ImportError::BadMultipart => write!(f, "multipart header out of range"),
        }
    }
}

impl std::error::Error for ImportError {}

fn hex_decode(hex: &str) -> Result<Vec<u8>, ImportError> {
    let hex = hex.trim();
    if hex.is_empty() || !hex.len().is_multiple_of(2) || !hex.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(ImportError::BadHex);
    }
    let bytes = (0..hex.len() / 2)
        .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(|_| ImportError::BadHex))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(bytes)
}

/// Decode a raw SMS-DELIVER PDU hex string.
pub fn decode_pdu_hex(hex: &str) -> Result<SmsDeliver, ImportError> {
    decode(&hex_decode(hex)?).map_err(ImportError::Pdu)
}

/// Build a [`Segment`] from a decoded PDU. Caller-supplied fallbacks (e.g.
/// the modem-reported index/storage/timestamp) take precedence where the PDU
/// cannot provide them; PDU-provided sender/text win over empty fields.
pub fn segment_from_pdu(
    modem_id: &str,
    storage: &str,
    source_index: i64,
    pdu_hex: &str,
    fallback_timestamp: i64,
) -> Result<Segment, ImportError> {
    let deliver = decode_pdu_hex(pdu_hex)?;
    let (reference, total_parts, part_number) = match deliver.udh {
        Some(udh) => {
            if udh.total_parts < 1 || udh.part_num < 1 || udh.part_num > udh.total_parts {
                return Err(ImportError::BadMultipart);
            }
            (udh.ref_num, udh.total_parts, udh.part_num)
        }
        None => (0, 1, 1),
    };
    Ok(Segment {
        modem_id: modem_id.to_string(),
        storage: storage.to_string(),
        source_index,
        pdu: pdu_hex.trim().to_ascii_uppercase(),
        sender: deliver.sender.clone(),
        timestamp: fallback_timestamp,
        content: deliver.text.clone(),
        reference,
        total_parts,
        part_number,
    })
}

/// One-shot import: decode the PDU and hand the segment to storage.
#[allow(clippy::too_many_arguments)] // flat mirror of C sms_segment_t inputs
pub fn import_pdu_segment(
    storage: &dyn Storage,
    modem_id: &str,
    mem: &str,
    source_index: i64,
    pdu_hex: &str,
    fallback_timestamp: i64,
    now: i64,
    forwarding: bool,
) -> Result<ImportResult, ImportError> {
    let segment = segment_from_pdu(modem_id, mem, source_index, pdu_hex, fallback_timestamp)?;
    storage
        .import_segment(&segment, now, forwarding)
        .map_err(ImportError::Storage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::InMemoryStorage;

    #[test]
    fn bad_hex_is_rejected() {
        assert_eq!(decode_pdu_hex("XYZ"), Err(ImportError::BadHex));
        assert_eq!(decode_pdu_hex("ABC"), Err(ImportError::BadHex));
    }

    #[test]
    fn import_via_pdu_decode_fills_fields() {
        // Cross-validation vector from qmodem-pdu: UCS-2 "你好",
        // sender +13818906569 (qmodem-pdu/src/lib.rs cross_vector_ucs2_nihao).
        let pdu = "0891683108101505F0040B913118986065F9000871218051002323044F60597D";
        let db = InMemoryStorage::new();
        let result = import_pdu_segment(&db, "m1", "SM", 4, pdu, 1_700_000_000, 100, false)
            .expect("import");
        assert!(!result.duplicate);
        assert!(result.published);
        assert_eq!(db.message_count("m1"), 1);
        // Second import of the same PDU dedups regardless of metadata.
        let again = import_pdu_segment(&db, "m1", "SM", 4, pdu, 1_700_000_100, 200, false)
            .expect("import");
        assert!(again.duplicate);
        assert_eq!(db.message_count("m1"), 1);
    }
}
