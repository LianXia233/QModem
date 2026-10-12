//! Shared helpers for the qmodem-pdu integration tests (zero-dependency).

// Each integration-test crate compiles this module standalone, so not every
// helper is referenced by every crate.
#![allow(dead_code, clippy::too_many_arguments)]

use qmodem_pdu::{Dcs, Timestamp};

/// Decode a hex string (whitespace tolerated) into bytes.
pub fn hex(s: &str) -> Vec<u8> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(s.len().is_multiple_of(2), "odd hex string: {s}");
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap())
        .collect()
}

/// Fixed sample timestamp (2024-02-29 23:59:59 GMT) used by most vectors.
pub fn sample_scts() -> Timestamp {
    Timestamp {
        year: 2024,
        month: 2,
        day: 29,
        hour: 23,
        minute: 59,
        second: 59,
        timezone_quarters: 0,
    }
}

/// Semi-octet coding of `digits` (odd length pads 0xF in the high nibble).
pub fn semi(digits: &str) -> Vec<u8> {
    let b = digits.as_bytes();
    assert!(b.iter().all(|c| c.is_ascii_digit()));
    (0..b.len().div_ceil(2))
        .map(|i| {
            let lo = b[2 * i] - b'0';
            let hi = if 2 * i + 1 < b.len() {
                b[2 * i + 1] - b'0'
            } else {
                0x0f
            };
            (hi << 4) | lo
        })
        .collect()
}

/// Assemble an SMS-DELIVER PDU (TP-MTI 00, MMS bit 0x04) from its fields.
///
/// `smsc`: `None` for an empty SMSC field, or `(digits, ton_is_international)`.
/// `udl` is the raw TP-UDL byte (septets for GSM 7-bit, octets otherwise).
pub fn deliver(
    smsc: Option<(&str, bool)>,
    sender: &str,
    toa_international: bool,
    pid: u8,
    dcs: Dcs,
    scts: &Timestamp,
    udl: u8,
    ud: &[u8],
) -> Vec<u8> {
    let dcs_byte = match dcs {
        Dcs::Gsm7Bit => 0x00u8,
        Dcs::EightBit => 0x04,
        Dcs::Ucs2 => 0x08,
    };
    let mut buf = Vec::new();
    match smsc {
        None => buf.push(0x00),
        Some((digits, intl)) => {
            let semi = semi(digits);
            buf.push((1 + semi.len()) as u8);
            buf.push(if intl { 0x91 } else { 0x81 });
            buf.extend_from_slice(&semi);
        }
    }
    buf.push(0x04); // TP-MTI SMS-DELIVER + TP-MMS
    let sender_semi = semi(sender);
    buf.push(sender.len() as u8);
    buf.push(if toa_international { 0x91 } else { 0x81 });
    buf.extend_from_slice(&sender_semi);
    buf.push(pid);
    buf.push(dcs_byte);
    buf.extend_from_slice(&qmodem_pdu::encode_scts(scts));
    buf.push(udl);
    buf.extend_from_slice(ud);
    buf
}

/// Assemble an SMS-DELIVER whose user data starts with a raw UDH block
/// (UDHI set, first UD octet is TP-UDHL).
pub fn deliver_with_udh(
    sender: &str,
    toa_international: bool,
    dcs: Dcs,
    scts: &Timestamp,
    udh: &[u8],
    payload: &[u8],
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(0x00); // no SMSC
    buf.push(0x44); // SMS-DELIVER + TP-UDHI + TP-MMS
    let sender_semi = semi(sender);
    buf.push(sender.len() as u8);
    buf.push(if toa_international { 0x91 } else { 0x81 });
    buf.extend_from_slice(&sender_semi);
    buf.push(0x00); // TP-PID
    let dcs_byte = match dcs {
        Dcs::Gsm7Bit => 0x00u8,
        Dcs::EightBit => 0x04,
        Dcs::Ucs2 => 0x08,
    };
    buf.push(dcs_byte);
    buf.extend_from_slice(&qmodem_pdu::encode_scts(scts));
    // For 7-bit DCS the TP-UDL counts septets including UDH filler; for
    // 8-bit/UCS-2 it counts octets including the UDH.
    match dcs {
        Dcs::Gsm7Bit => {
            let filler = (udh.len() * 8).div_ceil(7);
            let total_septets = filler + payload.len();
            buf.push(total_septets as u8);
            buf.extend_from_slice(udh); // UDH octets precede the packed text
            let mut septets = vec![0u8; filler];
            septets.extend_from_slice(payload);
            buf.extend_from_slice(&pack7(&septets));
        }
        _ => {
            buf.push((udh.len() + payload.len()) as u8);
            buf.extend_from_slice(udh);
            buf.extend_from_slice(payload);
        }
    }
    buf
}

/// Independent bit-exact GSM 7-bit packer (TS 23.038 §6.2.1, LSB-first):
/// each septet is expanded bit by bit and re-grouped into octets, so the
/// conformance vectors are not validated with the implementation under test.
pub fn pack7(septets: &[u8]) -> Vec<u8> {
    let mut bits: Vec<bool> = Vec::with_capacity(septets.len() * 7);
    for &s in septets {
        assert!(s < 0x80);
        for i in 0..7 {
            bits.push((s >> i) & 1 == 1);
        }
    }
    bits.chunks(8)
        .map(|c| {
            c.iter()
                .enumerate()
                .fold(0u8, |acc, (i, b)| acc | ((*b as u8) << i))
        })
        .collect()
}

/// xorshift64* PRNG (seeded deterministically) for the property tests.
pub struct Lcg(u64);

impl Lcg {
    pub fn new(seed: u64) -> Self {
        Lcg(seed | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545F4914F6CDD1D)
    }

    /// Uniform byte.
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 32) as u8
    }

    /// Value in `0..=n` (inclusive).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64 + 1)) as usize
    }
}
