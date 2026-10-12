//! Encode -> decode round trips: `encode_sms_submit` produces an
//! SMS-SUBMIT; the test converts it into an SMS-DELIVER (same address /
//! PID / DCS / user-data fields, TP-MTI swapped to 00) and asserts the
//! decoder recovers the exact original text, digits, and timestamp.

mod common;

use common::{deliver, hex, sample_scts};
use qmodem_pdu::{decode, encode_sms_submit, encode_scts, Dcs, Timestamp};

/// Convert an SMS-SUBMIT PDU from `encode_sms_submit` into an SMS-DELIVER
/// with the same address/PID/DCS/user data (TP-MTI 0x11 -> 0x04).
fn submit_to_deliver(submit: &[u8]) -> Vec<u8> {
    // SUBMIT layout (encode_sms_submit): [00][11][00][alen][toa][addr..]
    // [pid][dcs][vp][udl][ud..]
    assert_eq!(submit[0], 0x00, "encode_sms_submit emits an empty SMSC");
    assert_eq!(submit[1], 0x11);
    let alen = submit[3] as usize;
    let addr_octets = alen.div_ceil(2);
    let fields = 5 + addr_octets; // end of address value field
    let mut out = vec![0x00, 0x04]; // no SMSC; SMS-DELIVER + MMS
    out.extend_from_slice(&submit[3..fields]); // alen, TOA, address
    out.push(submit[fields]); // TP-PID
    out.push(submit[fields + 1]); // TP-DCS
    out.extend_from_slice(&encode_scts(&sample_scts()));
    out.extend_from_slice(&submit[fields + 3..]); // TP-UDL + user data
    out
}

fn roundtrip(text: &str, dcs: Dcs) {
    let submit = encode_sms_submit("10086", text, dcs).expect("encode");
    let deliver_pdu = submit_to_deliver(&submit);
    let m = decode(&deliver_pdu).unwrap_or_else(|e| panic!("decode failed: {e:?}"));
    assert_eq!(m.sender, "10086");
    assert_eq!(m.dcs, dcs);
    assert_eq!(m.scts, sample_scts());
    assert_eq!(m.text, text, "round trip text mismatch for {dcs:?}");
    assert!(m.udh.is_none());
}

#[test]
fn roundtrip_7bit_ascii() {
    roundtrip("hello world", Dcs::Gsm7Bit);
}

/// Escape-table characters cost two septets each; the round trip must
/// survive the septet-boundary shifts they cause.
#[test]
fn roundtrip_7bit_escape_chars() {
    roundtrip("{[}]~\\^|€", Dcs::Gsm7Bit);
}

/// Full single-SMS 7-bit capacity: exactly 160 septets.
#[test]
fn roundtrip_7bit_160_septets() {
    let text = "a".repeat(160);
    let submit = encode_sms_submit("10086", &text, Dcs::Gsm7Bit).expect("160 septets fit");
    let deliver_pdu = submit_to_deliver(&submit);
    let m = decode(&deliver_pdu).expect("decode");
    assert_eq!(m.text, text);
    assert_eq!(m.ud.len(), 140); // ceil(160*7/8)
}

/// Default-alphabet specials (septet 0x00 '@' must not be confused with
/// filler bits) plus CJK-free UCS-2 punctuation.
#[test]
fn roundtrip_7bit_septet_zero() {
    roundtrip("@\n\r_§ÄÖÜñüà", Dcs::Gsm7Bit);
}

#[test]
fn roundtrip_ucs2_cjk() {
    roundtrip("你好，世界！", Dcs::Ucs2);
}

/// Full single-SMS UCS-2 capacity: exactly 70 UTF-16 units.
#[test]
fn roundtrip_ucs2_70_units() {
    let text = "你".repeat(70);
    let submit = encode_sms_submit("10086", &text, Dcs::Ucs2).expect("70 units fit");
    let deliver_pdu = submit_to_deliver(&submit);
    let m = decode(&deliver_pdu).expect("decode");
    assert_eq!(m.text, text);
    assert_eq!(m.ud.len(), 140);
}

/// 8-bit binary data (chars <= U+00FF) survives as Latin-1 style text.
#[test]
fn roundtrip_8bit_binary() {
    roundtrip("\u{1}\u{7f}\u{ff}Ab9", Dcs::EightBit);
}

/// The SCTS encoder/decoder are inverses for every legal quarter-hour
/// offset in ±19:45, verified through a full PDU decode.
#[test]
fn roundtrip_scts_all_offsets() {
    for q in -79i8..=79 {
        let ts = Timestamp {
            timezone_quarters: q,
            ..sample_scts()
        };
        let buf = deliver(
            None,
            "1",
            false,
            0,
            Dcs::EightBit,
            &ts,
            1,
            &[0x41],
        );
        let m = decode(&buf).unwrap_or_else(|e| panic!("offset {q}: {e:?}"));
        assert_eq!(m.scts.timezone_quarters, q, "offset {q}");
        assert_eq!(m.scts.year, 2024);
        assert_eq!(m.scts.month, 2);
        assert_eq!(m.scts.day, 29);
        assert_eq!(m.scts.hour, 23);
        assert_eq!(m.scts.minute, 59);
        assert_eq!(m.scts.second, 59);
    }
}

/// Address round trip: odd/even digit counts and the TOA-driven `+`
/// prefix behave identically in encode and decode.
#[test]
fn roundtrip_address_odd_even() {
    // 5 digits -> unknown TOA (0x81), no "+" on decode.
    let submit = encode_sms_submit("10086", "x", Dcs::Gsm7Bit).unwrap();
    let m = decode(&submit_to_deliver(&submit)).expect("decode");
    assert_eq!(m.sender, "10086");
    assert_eq!(&submit[4], &0x81);

    // 6 digits -> international TOA (0x91), "+" on decode.
    let submit = encode_sms_submit("100860", "x", Dcs::Gsm7Bit).unwrap();
    let m = decode(&submit_to_deliver(&submit)).expect("decode");
    assert_eq!(m.sender, "+100860");
    assert_eq!(&submit[4], &0x91);

    // 11 digits with odd length: 0xF padding nibble round-trips.
    let submit = encode_sms_submit("13800138000", "x", Dcs::Gsm7Bit).unwrap();
    let m = decode(&submit_to_deliver(&submit)).expect("decode");
    assert_eq!(m.sender, "+13800138000");
    assert_eq!(&submit[5..11], &hex("3108108300F0")[..]);
}
