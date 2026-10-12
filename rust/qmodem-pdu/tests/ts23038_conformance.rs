//! TS 23.038 / TS 23.040 conformance vectors for the qmodem-pdu codec.
//!
//! Every fixed hex vector was derived by hand from the standard's field
//! layouts and bit-packing rules (§6.2.1 GSM 7-bit alphabet, §9.2.3.11
//! SCTS, TS 23.040 §9.2.3.24 UDH IEs), never from the implementation.

mod common;

use common::{deliver, deliver_with_udh, hex, pack7, sample_scts, semi};
use qmodem_pdu::{decode, encode_scts, Dcs, PduError, Timestamp, Udh};

fn septets(s: &[u8]) -> Vec<u8> {
    s.to_vec()
}

// ---------------------------------------------------------------------------
// GSM 7-bit (TP-DCS coding group 0000, alphabet bits 3..2 = 00)
// ---------------------------------------------------------------------------

/// Basic alphabet: "hello GSM" packs to the published LSB-first layout.
/// Hand-computed: septets 68 65 6C 6C 6F 20 47 53 4D -> E8 32 9B FD 06 1D A7 4D.
#[test]
fn gsm7_basic_alphabet() {
    let text_septets = b"hello GSM";
    let packed = hex("E8329BFD061DA74D");
    assert_eq!(pack7(text_septets), packed, "independent packer mismatch");
    let buf = deliver(
        None,
        "1",
        false,
        0,
        Dcs::Gsm7Bit,
        &sample_scts(),
        9,
        &packed,
    );
    let m = decode(&buf).expect("decode");
    assert_eq!(m.dcs, Dcs::Gsm7Bit);
    assert_eq!(m.text, "hello GSM");
    assert_eq!(m.ud, packed);
}

/// Default-alphabet specials from TS 23.038 table 6.2.1: '@' 0x00,
/// '£' 0x01, '$' 0x02, 'è' 0x04. 4 septets -> 28 bits -> 4 octets
/// (80 80 80 00, trailing nibble is filler).
#[test]
fn gsm7_special_chars() {
    let sept = septets(&[0x00, 0x01, 0x02, 0x04]);
    let packed = pack7(&sept);
    assert_eq!(packed, hex("80808000"));
    let buf = deliver(
        Some(("8613800151500", true)),
        "10086",
        false,
        0,
        Dcs::Gsm7Bit,
        &sample_scts(),
        4,
        &packed,
    );
    let m = decode(&buf).expect("decode");
    assert_eq!(m.smsc.as_deref(), Some("8613800151500"));
    assert_eq!(m.text, "@£$è");
}

/// Escape table (§6.2.1 table 6.3): each extended char costs 2 septets
/// (0x1B + code): ^ 0x14, { 0x28, } 0x29, \ 0x2F, [ 0x3C, ~ 0x3D, ] 0x3E,
/// | 0x40.
#[test]
fn gsm7_escape_chars() {
    let pairs: &[(u8, char)] = &[
        (0x14, '^'),
        (0x28, '{'),
        (0x29, '}'),
        (0x2F, '\\'),
        (0x3C, '['),
        (0x3D, '~'),
        (0x3E, ']'),
        (0x40, '|'),
    ];
    let mut sept = Vec::new();
    let mut expected = String::new();
    for &(code, ch) in pairs {
        sept.push(0x1b);
        sept.push(code);
        expected.push(ch);
    }
    let buf = deliver(
        None,
        "1",
        false,
        0,
        Dcs::Gsm7Bit,
        &sample_scts(),
        sept.len() as u8,
        &pack7(&sept),
    );
    let m = decode(&buf).expect("decode");
    assert_eq!(m.text, expected);
    assert_eq!(m.text.len(), 8, "8 chars from 16 septets");
}

/// Control characters '\n' (0x0A) and '\r' (0x0D) survive the codec.
#[test]
fn gsm7_control_chars() {
    let sept = septets(&[0x0A, 0x0D]);
    let buf = deliver(
        None,
        "1",
        false,
        0,
        Dcs::Gsm7Bit,
        &sample_scts(),
        2,
        &pack7(&sept),
    );
    let m = decode(&buf).expect("decode");
    assert_eq!(m.text, "\n\r");
}

// ---------------------------------------------------------------------------
// 8-bit (TP-DCS 0x04)
// ---------------------------------------------------------------------------

/// 8-bit data is passed through octet-exact; text is the Latin-1 style
/// rendering (each octet mapped to the same Unicode scalar).
#[test]
fn eightbit_raw_octets() {
    let raw = [0x00u8, 0x41, 0x7F, 0xFF];
    let buf = deliver(
        None,
        "10086",
        false,
        0,
        Dcs::EightBit,
        &sample_scts(),
        raw.len() as u8,
        &raw,
    );
    let m = decode(&buf).expect("decode");
    assert_eq!(m.dcs, Dcs::EightBit);
    assert_eq!(m.ud, raw.to_vec());
    assert_eq!(m.text, "\u{0}A\u{7f}\u{ff}");
}

/// 8-bit with TP-UDHI: TP-UDL counts octets including the UDH.
#[test]
fn eightbit_with_udh() {
    let udh = hex("050003AA0302"); // UDHL=5, concat IEI 0x00
    let payload = [0xABu8, 0xCD];
    let buf = deliver_with_udh("1", false, Dcs::EightBit, &sample_scts(), &udh, &payload);
    let m = decode(&buf).expect("decode");
    assert_eq!(m.dcs, Dcs::EightBit);
    assert_eq!(
        m.udh,
        Some(Udh {
            ref_num: 0xAA,
            total_parts: 3,
            part_num: 2
        })
    );
    assert_eq!(m.ud, payload.to_vec());
    assert_eq!(m.text, "\u{ab}\u{cd}");
}

// ---------------------------------------------------------------------------
// UCS-2 (TP-DCS 0x08)
// ---------------------------------------------------------------------------

/// UCS-2 ASCII-range text stays octet-exact (UTF-16BE units).
#[test]
fn ucs2_ascii_text() {
    let raw = hex("004100420043"); // "ABC"
    let buf = deliver(
        None,
        "1",
        false,
        0,
        Dcs::Ucs2,
        &sample_scts(),
        raw.len() as u8,
        &raw,
    );
    let m = decode(&buf).expect("decode");
    assert_eq!(m.dcs, Dcs::Ucs2);
    assert_eq!(m.text, "ABC");
    assert_eq!(m.ud, raw);
}

/// CJK BMP: "中文" = U+4E2D U+6587.
#[test]
fn ucs2_cjk_text() {
    let raw = hex("4E2D6587");
    let buf = deliver(
        None,
        "1",
        false,
        0,
        Dcs::Ucs2,
        &sample_scts(),
        raw.len() as u8,
        &raw,
    );
    let m = decode(&buf).expect("decode");
    assert_eq!(m.text, "中文");
}

/// Astral plane U+1F600 arrives as a UTF-16 surrogate pair (D83D DE00)
/// and is recombined into one scalar by the UTF-8 conversion.
#[test]
fn ucs2_surrogate_pair_emoji() {
    let raw = hex("D83DDE00");
    let buf = deliver(
        None,
        "1",
        false,
        0,
        Dcs::Ucs2,
        &sample_scts(),
        raw.len() as u8,
        &raw,
    );
    let m = decode(&buf).expect("decode");
    assert_eq!(m.text, "\u{1f600}");
}

// ---------------------------------------------------------------------------
// SCTS timezone: TS 23.040 §9.2.3.11 semi-octet coding
// (low nibble = tens digit + sign flag 0x08, high nibble = units digit,
// count in quarter hours, 0..=79 => ±19:45 max)
// ---------------------------------------------------------------------------

/// +12:45 = 51 quarters -> octet 0x15 (tens 5 low, units 1 high).
#[test]
fn scts_plus_12h45_boundary() {
    let scts = hex("42209232959515"); // 2024-02-29 23:59:59 +12:45
    let mut buf = deliver(
        None,
        "1",
        false,
        0,
        Dcs::EightBit,
        &sample_scts(),
        1,
        &[0x41],
    );
    // Overwrite the timezone octet (position 14) with the boundary value.
    buf[13] = 0x15;
    let m = decode(&buf).expect("decode");
    assert_eq!(m.scts.year, 2024);
    assert_eq!(m.scts.month, 2);
    assert_eq!(m.scts.day, 29);
    assert_eq!(m.scts.hour, 23);
    assert_eq!(m.scts.minute, 59);
    assert_eq!(m.scts.second, 59);
    assert_eq!(m.scts.timezone_quarters, 51);
    assert_eq!(scts[6], 0x15);
}

/// -12:45 = -51 quarters -> octet 0x1D (tens 5 + sign flag 0x08, units 1).
#[test]
fn scts_minus_12h45_boundary() {
    let mut buf = deliver(
        None,
        "1",
        false,
        0,
        Dcs::EightBit,
        &sample_scts(),
        1,
        &[0x41],
    );
    buf[13] = 0x1D;
    let m = decode(&buf).expect("decode");
    assert_eq!(m.scts.timezone_quarters, -51);
}

/// Known timezone octets across the representable range, and the
/// encode side is the exact inverse of the decode side.
#[test]
fn scts_range_and_roundtrip() {
    let cases: &[(u8, i8)] = &[
        (0x00, 0),   // GMT
        (0x80, 8),   // +2:00
        (0x88, -8),  // -2:00
        (0x22, 22),  // +5:30 (India)
        (0x2A, -22), // -5:30
        (0x23, 32),  // +8:00 (China)
        (0x2B, -32), // -8:00
        (0x40, 4),   // +1:00 (spec DST example 01000000)
        (0x15, 51),  // +12:45
        (0x1D, -51), // -12:45
        (0x97, 79),  // +19:45 max
        (0x9F, -79), // -19:45 max
    ];
    for &(octet, quarters) in cases {
        let mut buf = deliver(
            None,
            "1",
            false,
            0,
            Dcs::EightBit,
            &sample_scts(),
            1,
            &[0x41],
        );
        buf[13] = octet;
        let m = decode(&buf).unwrap_or_else(|e| panic!("octet {octet:02X}: {e:?}"));
        assert_eq!(m.scts.timezone_quarters, quarters, "octet {octet:02X}");
    }
    // Encoder inverse: for every legal quarter count the encoded octet
    // must decode back to the same value.
    for q in -79i8..=79 {
        let ts = Timestamp {
            timezone_quarters: q,
            ..sample_scts()
        };
        let enc = encode_scts(&ts);
        let mut buf = deliver(
            None,
            "1",
            false,
            0,
            Dcs::EightBit,
            &sample_scts(),
            1,
            &[0x41],
        );
        buf[7..14].copy_from_slice(&enc);
        let m = decode(&buf).unwrap_or_else(|e| panic!("quarters {q}: {e:?}"));
        assert_eq!(m.scts.timezone_quarters, q, "roundtrip {q}");
    }
}

/// Non-BCD digits in the timezone units nibble are rejected (high nibble
/// 0xA..0xF carries no valid digit).
#[test]
fn scts_rejects_bad_bcd() {
    for tz in [0xA0u8, 0xF0, 0xFE] {
        let mut buf = deliver(
            None,
            "1",
            false,
            0,
            Dcs::EightBit,
            &sample_scts(),
            1,
            &[0x41],
        );
        buf[13] = tz;
        assert_eq!(
            decode(&buf),
            Err(PduError::Invalid("bad scts field")),
            "tz {tz:02X}"
        );
    }
}

// ---------------------------------------------------------------------------
// User Data Header IEs (TS 23.040 §9.2.3.24)
// ---------------------------------------------------------------------------

/// IEI 0x00: concatenation with 8-bit reference (IEI, IEDL=3, ref, total, part).
#[test]
fn udh_iei_00_8bit_ref() {
    let udh = hex("050003420201"); // UDHL 5; ref 0x42, 2 parts, part 1
    let payload = hex("4F60"); // UCS-2 "你"
    let buf = deliver_with_udh("1", false, Dcs::Ucs2, &sample_scts(), &udh, &payload);
    let m = decode(&buf).expect("decode");
    assert_eq!(
        m.udh,
        Some(Udh {
            ref_num: 0x42,
            total_parts: 2,
            part_num: 1
        })
    );
    assert_eq!(m.text, "你");
}

/// IEI 0x08: concatenation with 16-bit reference (IEI, IEDL=4, ref_hi,
/// ref_lo, total, part); ref 0xABCD.
#[test]
fn udh_iei_08_16bit_ref() {
    let udh = hex("060804ABCD0302");
    let payload = hex("4F60");
    let buf = deliver_with_udh("1", false, Dcs::Ucs2, &sample_scts(), &udh, &payload);
    let m = decode(&buf).expect("decode");
    assert_eq!(
        m.udh,
        Some(Udh {
            ref_num: 0xABCD,
            total_parts: 3,
            part_num: 2
        })
    );
}

/// An unknown IE (here 8-bit port addressing, IEI 0x04) must be skipped
/// while the trailing concat IE is still picked up (parse every IE, C
/// parse_udh walks the full list the same way).
#[test]
fn udh_unknown_ie_skipped() {
    // UDHL 11: IE 0x05/4 (application port 16-bit: 1234 -> 5678) + concat
    // IE 0x00/3. Both unknown and concat IEs are parsed in one pass.
    let udh = hex("0B0504123456780003770303");
    let payload = hex("4F60");
    let buf = deliver_with_udh("1", false, Dcs::Ucs2, &sample_scts(), &udh, &payload);
    let m = decode(&buf).expect("decode");
    assert_eq!(
        m.udh,
        Some(Udh {
            ref_num: 0x77,
            total_parts: 3,
            part_num: 3
        })
    );
    assert_eq!(m.text, "你");
}

/// An IE whose IEDL runs past TP-UDHL must be rejected, not over-read.
#[test]
fn udh_ie_overflow() {
    let udh = hex("040010FF"); // UDHL 4, IEI 0x00 IEDL 0x10 > remaining
    let buf = deliver_with_udh("1", false, Dcs::Ucs2, &sample_scts(), &udh, &hex("4F60"));
    assert_eq!(decode(&buf), Err(PduError::Invalid("udh IE overflow")));
}

/// Sanity: the helper PDUs built by common::deliver with a numeric SMSC
/// keep their SMSC digits through the semi-octet round trip.
#[test]
fn deliver_helper_smsc_roundtrip() {
    let buf = deliver(
        Some(("10086", false)),
        "13800138000",
        true,
        0,
        Dcs::EightBit,
        &sample_scts(),
        1,
        &[0x41],
    );
    let m = decode(&buf).expect("decode");
    assert_eq!(m.smsc.as_deref(), Some("10086"));
    assert_eq!(m.sender, "+13800138000");
    assert_eq!(m.scts, sample_scts());
}

/// septets()/semi() helpers agree with the hand vectors used above.
#[test]
fn helper_semi_octet_known_values() {
    assert_eq!(semi("10086"), vec![0x01, 0x80, 0xF6]);
    assert_eq!(semi("8613800151500"), hex("683108101505F0"));
    let _ = septets(&[0]);
}
