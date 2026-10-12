//! 3GPP TS 23.038 SMS PDU codec.
//!
//! Rust rewrite of `application/libqmodem-sms/src/pdu.c` with corrected,
//! memory-safe semantics:
//!
//! * every index access is bounds-checked; truncated input yields
//!   [`PduError::Truncated`] instead of an out-of-bounds read
//!   (C `pdu_decode` reads `buffer[sms_deliver_start + 2]` and friends
//!   without verifying the buffer covers the sender address, pdu.c:552-562);
//! * the always-false digit validation `c < '0' && c > '9'` in
//!   `EncodePhoneNumber` (pdu.c:335) is replaced by strict digit checking
//!   that actually rejects non-digit input;
//! * SMS text length limits (160 septets / 70 UCS-2 units) are enforced up
//!   front instead of relying on the fixed 320-byte stack buffer
//!   (pdu.c:418-421), which multi-byte UTF-8 expansion in `AsciiToG7bit`
//!   (pdu.c:301-318) can overflow;
//! * UCS-2 decoded messages are converted to UTF-8 (the C version copies
//!   raw UCS-2 bytes into the output buffer, pdu.c:616-617).

/// SMS PDU codec error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PduError {
    /// Input buffer ended earlier than the PDU structure requires.
    Truncated,
    /// Field content is malformed (bad digit, bad BCD, bad UTF-16, ...).
    Invalid(&'static str),
    /// Encoded message exceeds 3GPP length limits (160 septets / 70 UCS-2 units).
    TooLong,
    /// Phone number contains characters other than ASCII digits.
    InvalidNumber,
}

impl std::fmt::Display for PduError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PduError::Truncated => write!(f, "pdu truncated"),
            PduError::Invalid(s) => write!(f, "invalid pdu: {s}"),
            PduError::TooLong => write!(f, "message too long"),
            PduError::InvalidNumber => {
                write!(f, "phone number must contain ASCII digits only")
            }
        }
    }
}

impl std::error::Error for PduError {}

/// Data coding scheme of the user data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dcs {
    /// GSM 7-bit default alphabet (TP-DCS coding group bits 7..4 = 0000).
    Gsm7Bit,
    /// 8-bit raw octets (TP-DCS 0x04).
    EightBit,
    /// UCS-2 (UTF-16BE), TP-DCS 0x08.
    Ucs2,
}

impl Dcs {
    fn from_tp_dcs(v: u8) -> Dcs {
        // General data coding group (bits 7..4 == 0000): alphabet sits in
        // bits 3..2 (TS 23.038 §4): 00 GSM 7-bit, 01 8-bit, 10 UCS-2.
        match (v >> 2) & 0x03 {
            0x01 => Dcs::EightBit,
            0x02 => Dcs::Ucs2,
            _ => Dcs::Gsm7Bit,
        }
    }

    fn to_tp_dcs(self) -> u8 {
        match self {
            Dcs::Gsm7Bit => 0x00,
            Dcs::EightBit => 0x04,
            Dcs::Ucs2 => 0x08,
        }
    }
}

/// Service centre time stamp (SCTS), decoded from the 7-octet semi-octet field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timestamp {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    /// Time zone offset in quarter-hours (15-minute steps).
    ///
    /// Per the SCTS timezone octet: the low nibble carries the offset count
    /// and bit 0x80 (bit 3 of the high nibble) marks a negative offset.
    pub timezone_quarters: i8,
}

/// Long-SMS concatenation header (TP-UDHI, IEI 0x00/0x08).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Udh {
    pub ref_num: u16,
    pub total_parts: u8,
    pub part_num: u8,
}

/// A decoded SMS-DELIVER (SM-RP-DUD).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmsDeliver {
    /// Short message service centre number, `None` when SMSC length is 0.
    pub smsc: Option<String>,
    /// Sender number (`+`-prefixed when TOA says international).
    pub sender: String,
    pub pid: u8,
    pub dcs: Dcs,
    pub scts: Timestamp,
    /// Concatenation header, `Some` when TP-UDHI is set and a concat IE exists.
    pub udh: Option<Udh>,
    /// Decoded text (UTF-8). For [`Dcs::EightBit`] this is a Latin-1 style
    /// rendering of the raw octets.
    pub text: String,
    /// Raw user-data octets (UDH removed).
    pub ud: Vec<u8>,
}

// ---------------------------------------------------------------------------
// GSM 7-bit default alphabet (3GPP TS 23.038 §6.2.1)
// ---------------------------------------------------------------------------

const GSM7_EXT_ESCAPE: u8 = 0x1b;

/// Septet value -> Unicode scalar, basic table. `None` for the escape slot.
fn gsm7_basic_char(septet: u8) -> Option<char> {
    match septet {
        0x00 => Some('@'),
        0x01 => Some('£'),
        0x02 => Some('$'),
        0x03 => Some('¥'),
        0x04 => Some('è'),
        0x05 => Some('é'),
        0x06 => Some('ù'),
        0x07 => Some('ì'),
        0x08 => Some('ò'),
        0x09 => Some('Ç'),
        0x0a => Some('\n'),
        0x0b => Some('Ø'),
        0x0c => Some('ø'),
        0x0d => Some('\r'),
        0x0e => Some('Å'),
        0x0f => Some('å'),
        0x10 => Some('Δ'),
        0x11 => Some('_'),
        0x12 => Some('Φ'),
        0x13 => Some('Γ'),
        0x14 => Some('Λ'),
        0x15 => Some('Ω'),
        0x16 => Some('Π'),
        0x17 => Some('Ψ'),
        0x18 => Some('Σ'),
        0x19 => Some('Θ'),
        0x1a => Some('Ξ'),
        0x1b => None, // escape
        0x1c => Some('Æ'),
        0x1d => Some('æ'),
        0x1e => Some('ß'),
        0x1f => Some('É'),
        0x20 => Some(' '),
        0x21..=0x3f => Some(septet as char),
        0x40 => Some('¡'),
        0x41..=0x5a => Some(septet as char),
        0x5b => Some('Ä'),
        0x5c => Some('Ö'),
        0x5d => Some('Ñ'),
        0x5e => Some('Ü'),
        0x5f => Some('§'),
        0x60 => Some('¿'),
        0x61..=0x7a => Some(septet as char),
        0x7b => Some('ä'),
        0x7c => Some('ö'),
        0x7d => Some('ñ'),
        0x7e => Some('ü'),
        0x7f => Some('à'),
        _ => None,
    }
}

/// Extended (escape) table: septet following 0x1b -> char.
fn gsm7_ext_char(septet: u8) -> Option<char> {
    match septet {
        0x0a => Some('\u{0c}'), // form feed
        0x14 => Some('^'),
        0x28 => Some('{'),
        0x29 => Some('}'),
        0x2f => Some('\\'),
        0x3c => Some('['),
        0x3d => Some('~'),
        0x3e => Some(']'),
        0x40 => Some('|'),
        0x65 => Some('€'),
        _ => None,
    }
}

/// Char -> septet(s): 1 basic septet, or 2 (escape + ext) septets.
/// Unsupported characters map to `?` (the C version's NPC).
fn char_to_septets(c: char) -> Vec<u8> {
    for s in 0u8..128 {
        if s != GSM7_EXT_ESCAPE && gsm7_basic_char(s) == Some(c) {
            return vec![s];
        }
    }
    for s in 0u8..128 {
        if s != GSM7_EXT_ESCAPE && gsm7_ext_char(s) == Some(c) {
            return vec![GSM7_EXT_ESCAPE, s];
        }
    }
    vec![0x3f] // '?'
}

// ---------------------------------------------------------------------------
// Bit packing
// ---------------------------------------------------------------------------

/// Pack 7-bit septets into 8-bit octets (LSB-first bit order).
fn pack_septets(septets: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity((septets.len() * 7).div_ceil(8));
    let mut acc: u32 = 0;
    let mut nbits: u32 = 0;
    for &s in septets {
        acc |= u32::from(s & 0x7f) << nbits;
        nbits += 7;
        while nbits >= 8 {
            out.push((acc & 0xff) as u8);
            acc >>= 8;
            nbits -= 8;
        }
    }
    if nbits > 0 {
        out.push((acc & 0xff) as u8);
    }
    out
}

/// Unpack `count` septets from octets; trailing filler bits are ignored.
fn unpack_septets(data: &[u8], count: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(count);
    let mut acc: u32 = 0;
    let mut nbits: u32 = 0;
    for &b in data {
        if out.len() >= count {
            break;
        }
        acc |= u32::from(b) << nbits;
        nbits += 8;
        while nbits >= 7 && out.len() < count {
            out.push((acc & 0x7f) as u8);
            acc >>= 7;
            nbits -= 7;
        }
    }
    out
}

/// Septets -> String, resolving escape sequences.
fn septets_to_string(septets: &[u8]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < septets.len() {
        let s = septets[i];
        if s == GSM7_EXT_ESCAPE {
            i += 1;
            let ext = if i < septets.len() { septets[i] } else { 0x3f };
            out.push(gsm7_ext_char(ext).unwrap_or('?'));
        } else {
            out.push(gsm7_basic_char(s).unwrap_or('?'));
        }
        i += 1;
    }
    out
}

// ---------------------------------------------------------------------------
// Number helpers (semi-octet coding)
// ---------------------------------------------------------------------------

/// Decode `digits` semi-octet digits starting at `buf[pos..]`.
fn decode_semi_digits(buf: &[u8], pos: usize, digits: usize) -> Result<String, PduError> {
    let octets = digits.div_ceil(2);
    if buf.len() < pos + octets {
        return Err(PduError::Truncated);
    }
    let mut out = String::with_capacity(digits);
    for i in 0..digits {
        let byte = buf[pos + i / 2];
        let d = if i % 2 == 0 { byte & 0x0f } else { byte >> 4 };
        if d > 9 {
            return Err(PduError::Invalid("bad semi-octet digit"));
        }
        out.push((b'0' + d) as char);
    }
    Ok(out)
}

/// Like [`decode_semi_digits`] but tolerates a trailing 0xF padding
/// half-octet (SMSC fields encode (octets-1)*2 half-octet slots, so odd
/// digit counts always carry one).
fn decode_semi_digits_lenient(buf: &[u8], pos: usize, digits: usize) -> Result<String, PduError> {
    let octets = digits.div_ceil(2);
    if buf.len() < pos + octets {
        return Err(PduError::Truncated);
    }
    let mut out = String::with_capacity(digits);
    for i in 0..digits {
        let byte = buf[pos + i / 2];
        let d = if i % 2 == 0 { byte & 0x0f } else { byte >> 4 };
        if d > 9 {
            if d == 0x0f && i + 1 == digits {
                break; // padding
            }
            return Err(PduError::Invalid("bad semi-octet digit"));
        }
        out.push((b'0' + d) as char);
    }
    Ok(out)
}

/// Encode an ASCII-digit string into semi-octets; odd length pads with 0xF.
fn encode_semi_digits(digits: &str) -> Vec<u8> {
    let n = digits.len();
    let mut out = Vec::with_capacity(n.div_ceil(2));
    for (i, b) in digits.bytes().enumerate() {
        let d = b - b'0';
        if i % 2 == 0 {
            out.push(d & 0x0f);
        } else {
            let last = out.len() - 1;
            out[last] |= d << 4;
        }
    }
    if n % 2 == 1 {
        let last = out.len() - 1;
        out[last] |= 0xf0;
    }
    out
}

/// Decode an address value field (SMSC or sender).
///
/// `pos` points at the type-of-address octet; `digits` is the length octet
/// (number of semi-octet digits, or septets for alphanumeric TOA).
fn decode_address(buf: &[u8], pos: usize, digits: u8) -> Result<String, PduError> {
    let toa = *buf.get(pos).ok_or(PduError::Truncated)?;
    if toa & 0x80 == 0 {
        return Err(PduError::Invalid("bad type-of-address"));
    }
    let ton = (toa >> 4) & 0x07;
    let octets = (digits as usize).div_ceil(2);
    if buf.len() < pos + 1 + octets {
        return Err(PduError::Truncated);
    }
    if ton == 5 {
        // Alphanumeric: the value field carries GSM 7-bit septets.
        let septets = unpack_septets(&buf[pos + 1..pos + 1 + octets], digits as usize);
        return Ok(septets_to_string(&septets));
    }
    let number = decode_semi_digits(buf, pos + 1, digits as usize)?;
    Ok(if ton == 1 { format!("+{number}") } else { number })
}

// ---------------------------------------------------------------------------
// SCTS
// ---------------------------------------------------------------------------

fn swap_decimal_nibble(x: u8) -> u8 {
    (x / 16) + (x % 16) * 10
}

fn parse_scts(buf: &[u8], pos: usize) -> Result<Timestamp, PduError> {
    if buf.len() < pos + 7 {
        return Err(PduError::Truncated);
    }
    let year = 2000 + swap_decimal_nibble(buf[pos]) as u16;
    let month = swap_decimal_nibble(buf[pos + 1]);
    let day = swap_decimal_nibble(buf[pos + 2]);
    let hour = swap_decimal_nibble(buf[pos + 3]);
    let minute = swap_decimal_nibble(buf[pos + 4]);
    let second = swap_decimal_nibble(buf[pos + 5]);
    let tz_byte = buf[pos + 6];
    let count = (tz_byte & 0x0f) as i8;
    let tzq = if tz_byte & 0x80 != 0 { -count } else { count };
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return Err(PduError::Invalid("bad scts field"));
    }
    Ok(Timestamp {
        year,
        month,
        day,
        hour,
        minute,
        second,
        timezone_quarters: tzq,
    })
}

pub fn encode_scts(ts: &Timestamp) -> [u8; 7] {
    // Semi-octet: tens digit in the LOW nibble, ones in the HIGH nibble.
    let bcd = |v: u8| ((v % 10) << 4) | (v / 10);
    let tz_count = ts.timezone_quarters.unsigned_abs() & 0x0f;
    let tz_flag = if ts.timezone_quarters < 0 { 0x80 } else { 0x00 };
    [
        bcd((ts.year % 100) as u8),
        bcd(ts.month),
        bcd(ts.day),
        bcd(ts.hour),
        bcd(ts.minute),
        bcd(ts.second),
        tz_flag | tz_count,
    ]
}

/// Convert UTF-16BE octets to a UTF-8 String, validating surrogate pairs.
fn decode_ucs2(raw: Vec<u8>) -> Result<(String, Vec<u8>), PduError> {
    if !raw.len().is_multiple_of(2) {
        return Err(PduError::Invalid("odd ucs2 length"));
    }
    let mut text = String::with_capacity(raw.len() / 2);
    let mut pending_high: Option<u16> = None;
    for pair in raw.chunks_exact(2) {
        let unit = u16::from_be_bytes([pair[0], pair[1]]);
        match (pending_high.take(), unit) {
            (None, u) if (0xd800..0xdc00).contains(&u) => pending_high = Some(u),
            (None, u) if (0xdc00..0xe000).contains(&u) => {
                return Err(PduError::Invalid("lone low surrogate"));
            }
            (None, u) => {
                text.push(char::from_u32(u32::from(u)).ok_or(PduError::Invalid("bad ucs2 unit"))?)
            }
            (Some(hi), u) if (0xdc00..0xe000).contains(&u) => {
                let cp =
                    0x10000 + ((u32::from(hi) - 0xd800) << 10) + (u32::from(u) - 0xdc00);
                text.push(char::from_u32(cp).ok_or(PduError::Invalid("bad surrogate"))?);
            }
            (Some(_), _) => return Err(PduError::Invalid("lone high surrogate")),
        }
    }
    if pending_high.is_some() {
        return Err(PduError::Invalid("lone high surrogate"));
    }
    Ok((text, raw))
}

// ---------------------------------------------------------------------------
// Public API: decode
// ---------------------------------------------------------------------------

/// Decode an SMS-DELIVER PDU (SM-RP-DUD as read from `AT+CMGR`).
///
/// All field parsing is bounds-checked; short input yields
/// [`PduError::Truncated`] and the function never panics.
pub fn decode(buf: &[u8]) -> Result<SmsDeliver, PduError> {
    if buf.is_empty() {
        return Err(PduError::Truncated);
    }
    // SMSC length octet counts OCTETS including the type-of-address octet
    // (TS 24.011 §10.5.1 / 23.040 §9.1.2.5).
    let smsc_field = buf[0] as usize;

    let smsc = if smsc_field == 0 {
        None
    } else {
        if buf.len() < 1 + smsc_field {
            return Err(PduError::Truncated);
        }
        let digits = (smsc_field - 1) * 2;
        Some(decode_semi_digits_lenient(buf, 2, digits)?)
    };
    let tp_start = 1 + smsc_field;

    let first_tp = *buf.get(tp_start).ok_or(PduError::Truncated)?;
    if first_tp & 0x03 != 0x00 {
        return Err(PduError::Invalid("not SMS-DELIVER"));
    }
    let has_udhi = first_tp & 0x40 != 0;

    // Sender address: length octet, TOA, digits.
    let sender_len = *buf.get(tp_start + 1).ok_or(PduError::Truncated)?;
    let sender = decode_address(buf, tp_start + 2, sender_len)?;
    let mut pos = tp_start + 2 + 1 + (sender_len as usize).div_ceil(2);

    let pid = *buf.get(pos).ok_or(PduError::Truncated)?;
    pos += 1;
    let tp_dcs = *buf.get(pos).ok_or(PduError::Truncated)?;
    let dcs = Dcs::from_tp_dcs(tp_dcs);
    pos += 1;

    let scts = parse_scts(buf, pos)?;
    pos += 7;

    let udl = *buf.get(pos).ok_or(PduError::Truncated)? as usize;
    pos += 1;
    let ud_full = buf.get(pos..).ok_or(PduError::Truncated)?;

    // Concatenation UDH (IEI 0x00: 8-bit ref, IEI 0x08: 16-bit ref).
    let mut udh = None;
    let mut payload = ud_full;
    let mut udh_octets = 0usize;
    if has_udhi {
        let udhl = *ud_full.first().ok_or(PduError::Truncated)? as usize;
        udh_octets = 1 + udhl;
        if ud_full.len() < udh_octets {
            return Err(PduError::Truncated);
        }
        let mut ie = 1usize;
        while ie + 1 < udh_octets {
            let iei = ud_full[ie];
            let iedl = ud_full[ie + 1] as usize;
            if ie + 2 + iedl > udh_octets {
                return Err(PduError::Invalid("udh IE overflow"));
            }
            match (iei, iedl) {
                (0x00, 3) => {
                    udh = Some(Udh {
                        ref_num: u16::from(ud_full[ie + 2]),
                        total_parts: ud_full[ie + 3],
                        part_num: ud_full[ie + 4],
                    });
                }
                (0x08, 4) => {
                    udh = Some(Udh {
                        ref_num: (u16::from(ud_full[ie + 2]) << 8) | u16::from(ud_full[ie + 3]),
                        total_parts: ud_full[ie + 4],
                        part_num: ud_full[ie + 5],
                    });
                }
                _ => {}
            }
            ie += 2 + iedl;
        }
        payload = &ud_full[udh_octets..];
    }

    let (text, raw_ud) = match dcs {
        Dcs::Gsm7Bit => {
            // TP-UDL counts septets including the UDH filler septets; the
            // filler bits live in the payload octets after the UDH.
            let needed_octets = (udl * 7).div_ceil(8);
            if payload.len() < needed_octets {
                return Err(PduError::Truncated);
            }
            let septets = unpack_septets(payload, udl);
            let skip = if has_udhi {
                // Septets occupied by the UDH octets: ceil(udh_octets*8/7).
                (udh_octets * 8).div_ceil(7)
            } else {
                0
            };
            if skip > septets.len() {
                return Err(PduError::Invalid("udh septets overflow"));
            }
            (septets_to_string(&septets[skip..]), pack_septets(&septets[skip..]))
        }
        Dcs::EightBit | Dcs::Ucs2 => {
            // TP-UDL counts octets including the UDH when present.
            let data_len = if has_udhi {
                udl.saturating_sub(udh_octets)
            } else {
                udl
            };
            if payload.len() < data_len {
                return Err(PduError::Truncated);
            }
            let raw = payload[..data_len].to_vec();
            if matches!(dcs, Dcs::EightBit) {
                let text = raw.iter().map(|&b| b as char).collect();
                (text, raw)
            } else {
                decode_ucs2(raw)?
            }
        }
    };

    Ok(SmsDeliver {
        smsc,
        sender,
        pid,
        dcs,
        scts,
        udh,
        text,
        ud: raw_ud,
    })
}

// ---------------------------------------------------------------------------
// Public API: encode SMS-SUBMIT
// ---------------------------------------------------------------------------

/// Encode an SMS-SUBMIT PDU (no SMSC field; length byte for `AT+CMGS=<len>`
/// is `pdu.len() - 1`).
///
/// * `phone_number` must be non-empty ASCII digits only — this actually
///   rejects non-digits, unlike the always-false check at pdu.c:335;
/// * 7-bit text is limited to 160 septets, UCS-2 to 70 UTF-16 units and
///   8-bit to 140 octets, replacing the fixed-stack-buffer overflow risk
///   of pdu.c:418-421 with an explicit [`PduError::TooLong`].
pub fn encode_sms_submit(phone_number: &str, text: &str, dcs: Dcs) -> Result<Vec<u8>, PduError> {
    validate_digits(phone_number)?;

    let mut out: Vec<u8> = Vec::with_capacity(64);
    out.push(0x00); // SMSC length: none.
    out.push(0x11); // TP-MTI SMS-SUBMIT, relative TP-VP format.
    out.push(0x00); // TP-MR.

    // TP-DA.
    out.push(phone_number.len() as u8);
    out.push(if phone_number.len() < 6 { 0x81 } else { 0x91 });
    out.extend_from_slice(&encode_semi_digits(phone_number));

    out.push(0x00); // TP-PID.
    out.push(dcs.to_tp_dcs()); // TP-DCS.
    out.push(0xb0); // TP-VP: 10 days.

    match dcs {
        Dcs::Gsm7Bit => {
            let mut septets = Vec::with_capacity(text.len());
            for c in text.chars() {
                septets.extend_from_slice(&char_to_septets(c));
            }
            if septets.len() > 160 {
                return Err(PduError::TooLong);
            }
            out.push(septets.len() as u8); // TP-UDL in septets.
            out.extend_from_slice(&pack_septets(&septets));
        }
        Dcs::Ucs2 => {
            let mut units: Vec<u16> = Vec::new();
            for c in text.chars() {
                let mut buf = [0u16; 2];
                units.extend_from_slice(c.encode_utf16(&mut buf));
            }
            if units.len() > 70 {
                return Err(PduError::TooLong);
            }
            out.push((units.len() * 2) as u8); // TP-UDL in octets.
            for u in units {
                out.extend_from_slice(&u.to_be_bytes());
            }
        }
        Dcs::EightBit => {
            if text.chars().count() > 140 {
                return Err(PduError::TooLong);
            }
            let raw: Vec<u8> = text
                .chars()
                .map(|c| if c as u32 > 0xff { b'?' } else { c as u8 })
                .collect();
            out.push(raw.len() as u8);
            out.extend_from_slice(&raw);
        }
    }
    Ok(out)
}

fn validate_digits(phone: &str) -> Result<(), PduError> {
    if phone.is_empty() || !phone.bytes().all(|b| b.is_ascii_digit()) {
        return Err(PduError::InvalidNumber);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        (0..s.len() / 2)
            .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap())
            .collect()
    }

    /// Cross-validation vector 1: UCS-2 "你好", sender +13818906569.
    /// Constructed by hand from the C decoder's field layout
    /// (pdu.c:534-630): SMSC(08 91...) + TP-MTI(04) + sender(0B 91...)
    /// + PID/DCS(00 08) + SCTS(2017-12-08 15:00:32 +8q) + UDL(04)
    /// + UD(4F60 597D).
    #[test]
    fn cross_vector_ucs2_nihao() {
        let buf = hex("0891683108101505F0040B913118986065F9000871218051002308044F60597D");
        let m = decode(&buf).expect("decode");
        assert_eq!(m.smsc.as_deref(), Some("8613800151500"));
        assert_eq!(m.sender, "+13818906569");
        assert_eq!(m.dcs, Dcs::Ucs2);
        assert_eq!(m.pid, 0);
        assert_eq!(m.text, "你好");
        assert_eq!(m.ud, hex("4F60597D"));
        assert!(m.udh.is_none());
        assert_eq!(
            m.scts,
            Timestamp {
                year: 2017,
                month: 12,
                day: 8,
                hour: 15,
                minute: 0,
                second: 32,
                timezone_quarters: 8,
            }
        );
    }

    /// Cross-validation vector 2: GSM 7-bit "hellohello" (the classic
    /// dreamfabric bit-layout example), 5-digit national sender "10086"
    /// with unknown TOA (0x81, no `+` prefix), timezone -8 quarters
    /// (0x88: count 8, sign bit 0x80).
    #[test]
    fn cross_vector_7bit_hellohello() {
        let buf = hex("0891683108101505F00405810180F60000321051900300880AE8329BFD4697D9EC37");
        let m = decode(&buf).expect("decode");
        assert_eq!(m.sender, "10086");
        assert_eq!(m.dcs, Dcs::Gsm7Bit);
        assert_eq!(m.text, "hellohello");
        assert_eq!(m.ud, hex("E8329BFD4697D9EC37"));
        assert_eq!(m.scts.year, 2023);
        assert_eq!(m.scts.month, 1);
        assert_eq!(m.scts.day, 15);
        assert_eq!(m.scts.hour, 9);
        assert_eq!(m.scts.minute, 30);
        assert_eq!(m.scts.second, 0);
        assert_eq!(m.scts.timezone_quarters, -8);
    }

    /// Cross-validation vector 3: UCS-2 with UDH (concat ref 0xAB, 3
    /// parts, part 2), TP-UDHI set in the first TP octet (0x44).
    #[test]
    fn cross_vector_udh_ucs2() {
        let buf = hex("0891683108101505F0440B913118986065F9000871218051002308 08 050003AB03024F60");
        let m = decode(&buf).expect("decode");
        assert_eq!(
            m.udh,
            Some(Udh {
                ref_num: 0xab,
                total_parts: 3,
                part_num: 2,
            })
        );
        assert_eq!(m.text, "你");
        assert_eq!(m.ud, hex("4F60"));
    }

    #[test]
    fn scts_timezone_polarity() {
        // 0x88: count 8 + sign bit => -8 quarters (-2h).
        // Date 2023-01-15 09:30:00 in semi-octet coding:
        // tens digit sits in the LOW nibble of each octet.
        let ts = parse_scts(&[0x32, 0x10, 0x51, 0x90, 0x03, 0x00, 0x88], 0).unwrap();
        assert_eq!(ts.timezone_quarters, -8);
        // Round-trip through the encoder.
        assert_eq!(encode_scts(&ts), [0x32, 0x10, 0x51, 0x90, 0x03, 0x00, 0x88]);
        // 0x08: positive 8 quarters (+2h).
        let pos = parse_scts(&[0x71, 0x21, 0x80, 0x51, 0x00, 0x23, 0x08], 0).unwrap();
        assert_eq!(pos.timezone_quarters, 8);
        assert_eq!(
            pos,
            Timestamp {
                year: 2017,
                month: 12,
                day: 8,
                hour: 15,
                minute: 0,
                second: 32,
                timezone_quarters: 8,
            }
        );
        // Invalid calendar fields must be rejected (month byte 0x31 => 13).
        assert_eq!(
            parse_scts(&[0x32, 0x31, 0x51, 0x90, 0x03, 0x00, 0x08], 0),
            Err(PduError::Invalid("bad scts field"))
        );
    }

    #[test]
    fn seven_bit_alignment() {
        // "hellohello" must match the published dreamfabric bit layout.
        let septets: Vec<u8> = "hellohello".bytes().collect();
        assert_eq!(pack_septets(&septets), hex("E8329BFD4697D9EC37"));
        assert_eq!(unpack_septets(&hex("E8329BFD4697D9EC37"), 10), septets);

        // Escape sequences: '{' costs two septets and round-trips.
        let s = "a{b}";
        let mut septs = Vec::new();
        for c in s.chars() {
            septs.extend_from_slice(&char_to_septets(c));
        }
        assert_eq!(septs.len(), 6); // '{' and '}' cost 2 septets each
        assert_eq!(septets_to_string(&septs), s);
        // '@' is septet 0x00 and must not be confused with filler.
        let mut septs0 = Vec::new();
        for c in "@\n\r€".chars() {
            septs0.extend_from_slice(&char_to_septets(c));
        }
        assert_eq!(septets_to_string(&septs0), "@\n\r€");
    }

    #[test]
    fn seven_bit_with_udh_filler() {
        // 7-bit long SMS: UDH is 6 octets => ceil(6*8/7) = 7 filler septets.
        let mut all = vec![0u8; 7];
        all.extend_from_slice("world".as_bytes());
        let packed = pack_septets(&all);
        let mut ud = vec![5u8, 0x00, 0x03, 0x01, 0x03, 0x02]; // UDHL + concat IE
        ud.extend_from_slice(&packed);
        let mut buf = vec![
            0x00, // SMSC length 0
            0x04, // SMS-DELIVER + UDHI? no: 0x04 only; UDHI below
        ];
        buf[1] = 0x44; // MTI 00 + UDHI 0x40 + MMS 0x04
        buf.extend_from_slice(&[0x01, 0x81, 0xf1]); // sender "1", unknown TOA
        buf.extend_from_slice(&[0x00, 0x00]); // PID, DCS 7-bit
        buf.extend_from_slice(&[0x23, 0x01, 0x51, 0x90, 0x03, 0x00, 0x08]); // SCTS
        buf.push(12); // TP-UDL in septets: 7 filler + 5 text
        buf.extend_from_slice(&ud);
        let m = decode(&buf).expect("decode");
        assert_eq!(
            m.udh,
            Some(Udh {
                ref_num: 1,
                total_parts: 3,
                part_num: 2,
            })
        );
        assert_eq!(m.text, "world");
    }

    #[test]
    fn ucs2_to_utf8_surrogate_pair() {
        // U+1F600 encoded as surrogate pair D83A DE00.
        let mut buf = vec![0x00, 0x04, 0x01, 0x81, 0xf1, 0x00, 0x08];
        buf.extend_from_slice(&[0x23, 0x01, 0x51, 0x90, 0x03, 0x00, 0x08]);
        buf.push(0x04);
        buf.extend_from_slice(&[0xd8, 0x3d, 0xde, 0x00]);
        let m = decode(&buf).expect("decode");
        assert_eq!(m.text, "\u{1f600}");
        // Lone high surrogate must be rejected.
        let mut bad = buf.clone();
        bad.pop();
        bad.pop();
        bad.push(0x02);
        bad.extend_from_slice(&[0xd8, 0x3a]);
        assert_eq!(decode(&bad), Err(PduError::Invalid("lone high surrogate")));
    }

    #[test]
    fn eight_bit_passthrough() {
        let mut buf = vec![0x00, 0x04, 0x01, 0x81, 0xf1, 0x00, 0x04];
        buf.extend_from_slice(&[0x23, 0x01, 0x51, 0x90, 0x03, 0x00, 0x08]);
        buf.push(0x03);
        buf.extend_from_slice(&[0x01, 0xff, 0x80]);
        let m = decode(&buf).expect("decode");
        assert_eq!(m.dcs, Dcs::EightBit);
        assert_eq!(m.ud, vec![0x01, 0xff, 0x80]);
        assert_eq!(m.text, "\u{1}\u{ff}\u{80}");
    }

    #[test]
    fn encode_phone_number_odd_even() {
        let even = encode_sms_submit("13800138000", "hi", Dcs::Gsm7Bit).unwrap();
        assert_eq!(even[1], 0x11);
        assert_eq!(even[3], 11); // address length in digits
        assert_eq!(even[4], 0x91); // >= 6 digits => international
        assert_eq!(&even[5..11], &[0x31, 0x08, 0x10, 0x83, 0x00, 0xf0]);
        // Odd digit count pads the last nibble with F; short number => TOA 0x81.
        let odd = encode_sms_submit("12345", "a", Dcs::Gsm7Bit).unwrap();
        assert_eq!(odd[3], 5);
        assert_eq!(odd[4], 0x81);
        assert_eq!(&odd[5..8], &[0x21, 0x43, 0xf5]);
    }

    #[test]
    fn encode_sms_submit_vector() {
        // Full SMS-SUBMIT for "hello" to 10086; user data matches the
        // published 7-bit packing E8 32 9B FD 06.
        let pdu = encode_sms_submit("10086", "hello", Dcs::Gsm7Bit).unwrap();
        assert_eq!(pdu, hex("00110005810180F60000B005E8329BFD06"));
        assert_eq!(&pdu[pdu.len() - 5..], &hex("E8329BFD06")[..]);
    }

    #[test]
    fn encode_rejects_bad_number() {
        assert_eq!(
            encode_sms_submit("12a4", "x", Dcs::Gsm7Bit).unwrap_err(),
            PduError::InvalidNumber
        );
        assert_eq!(
            encode_sms_submit("+8613800138000", "x", Dcs::Gsm7Bit).unwrap_err(),
            PduError::InvalidNumber
        );
        assert_eq!(
            encode_sms_submit("", "x", Dcs::Gsm7Bit).unwrap_err(),
            PduError::InvalidNumber
        );
    }

    #[test]
    fn encode_length_limits() {
        // 7-bit: exactly 160 septets OK, 161 rejected.
        let ok7 = encode_sms_submit("10086", &"a".repeat(160), Dcs::Gsm7Bit).unwrap();
        assert_eq!(ok7[ok7.len() - 141], 160);
        assert_eq!(
            encode_sms_submit("10086", &"a".repeat(161), Dcs::Gsm7Bit).unwrap_err(),
            PduError::TooLong
        );
        // UCS-2: 70 units OK, 71 rejected.
        let ok_u = encode_sms_submit("10086", &"你".repeat(70), Dcs::Ucs2).unwrap();
        assert_eq!(ok_u[ok_u.len() - 141], 140);
        assert_eq!(
            encode_sms_submit("10086", &"你".repeat(71), Dcs::Ucs2).unwrap_err(),
            PduError::TooLong
        );
    }

    #[test]
    fn truncated_input_never_panics() {
        let vectors = [
            "0891683108101505F0040B913118986065F9000871218051002308044F60597D",
            "0891683108101505F00405810180F60000321051900300880AE8329BFD4697D9EC37",
            "0891683108101505F0440B913118986065F900087121805100230808050003AB03024F60",
            "00",
            "0F91",
        ];
        for v in vectors {
            let full = hex(v);
            for i in 0..full.len() {
                assert!(
                    decode(&full[..i]).is_err(),
                    "prefix length {i} of {v} must fail"
                );
            }
        }
        assert_eq!(decode(&[]), Err(PduError::Truncated));
    }

    #[test]
    fn garbage_is_invalid_not_panic() {
        // Absurd SMSC length vs buffer size.
        assert_eq!(decode(&[0xff, 0x91]), Err(PduError::Truncated));
        // Wrong TP-MTI.
        assert_eq!(
            decode(&hex("000301")),
            Err(PduError::Invalid("not SMS-DELIVER"))
        );
        // Bad type-of-address (extension bit clear).
        assert!(decode(&hex("0204910F0000")).is_err());
        // Bad semi-octet digit.
        assert!(decode(&hex("000401810F100080")).is_err());
        // Odd UCS-2 payload length.
        let mut bad = vec![0x00, 0x04, 0x01, 0x81, 0xf1, 0x00, 0x08];
        bad.extend_from_slice(&[0x23, 0x01, 0x51, 0x90, 0x03, 0x00, 0x08, 0x03]);
        bad.extend_from_slice(&[0x4f, 0x60, 0x01]);
        assert_eq!(decode(&bad), Err(PduError::Invalid("odd ucs2 length")));
    }
}
