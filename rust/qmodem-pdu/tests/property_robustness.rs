//! Property-style robustness tests: the decoder must never panic on any
//! input — truncated prefixes, random bytes, mutated fields, inconsistent
//! UDLs, absurd SMSC lengths — it may only return Ok or Err.

mod common;

use common::{deliver, deliver_with_udh, hex, sample_scts, Lcg};
use qmodem_pdu::{decode, Dcs, PduError};

/// Base PDUs covering every DCS, UDH presence, and SMSC layout.
fn base_pdus() -> Vec<Vec<u8>> {
    vec![
        deliver(
            None,
            "10086",
            false,
            0,
            Dcs::Gsm7Bit,
            &sample_scts(),
            10,
            &hex("E8329BFD4697D9EC37"),
        ),
        deliver(
            Some(("8613800151500", true)),
            "13818906569",
            true,
            0,
            Dcs::Ucs2,
            &sample_scts(),
            4,
            &hex("4F60597D"),
        ),
        deliver_with_udh(
            "1",
            false,
            Dcs::Gsm7Bit,
            &sample_scts(),
            &hex("060804ABCD0302"),
            b"world",
        ),
        deliver(
            Some(("10086", false)),
            "1",
            false,
            0,
            Dcs::EightBit,
            &sample_scts(),
            3,
            &[0x01, 0xFF, 0x80],
        ),
    ]
}

/// Every strict prefix of every base PDU must decode to an Err (the PDU
/// structure cannot be validly truncated) and never panic.
#[test]
fn all_prefixes_are_errors_never_panic() {
    for base in base_pdus() {
        for len in 0..base.len() {
            let res = decode(&base[..len]);
            assert!(res.is_err(), "prefix len {len} of {:02X?} decoded ok", &base[..len]);
        }
    }
    assert_eq!(decode(&[]), Err(PduError::Truncated));
}

/// Pseudo-random byte buffers (zero-dep xorshift64*): decode may return
/// Ok or Err but must not panic. 5000 iterations.
#[test]
fn random_bytes_never_panic() {
    let mut rng = Lcg::new(0xC0FFEE);
    for _ in 0..5000 {
        let len = rng.below(64);
        let buf: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        let _ = decode(&buf);
    }
}

/// Single-byte mutations of valid PDUs: every byte replaced with every
/// corner value must not panic (0x00: zero-prefix length, 0xFF: absurd
/// lengths, 0x08/0x88: sign-flag extremes, 0x0F/0xF0: BCD padding).
#[test]
fn single_byte_mutation_never_panic() {
    let corners = [0x00u8, 0x01, 0x08, 0x0F, 0x80, 0x88, 0xF0, 0xFF];
    for base in base_pdus() {
        for pos in 0..base.len() {
            for &c in &corners {
                let mut buf = base.clone();
                buf[pos] = c;
                let _ = decode(&buf);
            }
        }
    }
}

/// TP-UDL = 0 with payload present: valid, yields an empty text.
#[test]
fn udl_zero_with_payload() {
    let buf = deliver(
        None,
        "1",
        false,
        0,
        Dcs::Gsm7Bit,
        &sample_scts(),
        0,
        &[0x41, 0x42],
    );
    let m = decode(&buf).expect("UDL 0 is a valid empty message");
    assert_eq!(m.text, "");
    assert!(m.ud.is_empty());
}

/// TP-UDL far exceeding the actual payload must yield Truncated, not a
/// panic, for all three DCS variants (UDL 0xFF = 255 septets/octets).
#[test]
fn udl_exceeds_payload_is_truncated() {
    for dcs in [Dcs::Gsm7Bit, Dcs::EightBit, Dcs::Ucs2] {
        let buf = deliver(None, "1", false, 0, dcs, &sample_scts(), 0xFF, &[0x41, 0x42]);
        assert_eq!(decode(&buf), Err(PduError::Truncated), "{dcs:?}");
    }
    // Boundary: 7-bit UDL exactly one septet short of the payload bits.
    let buf = deliver(None, "1", false, 0, Dcs::Gsm7Bit, &sample_scts(), 9, &[0x41; 7]);
    assert_eq!(decode(&buf), Err(PduError::Truncated));
}

/// TP-UDL smaller than the payload: trailing octets are ignored and the
/// message decodes from exactly UDL units.
#[test]
fn udl_below_payload_ignores_tail() {
    // "hi" = 2 septets (68 69) -> 2 octets E8 34; trailing 0xAA ignored.
    let buf = deliver(
        None,
        "1",
        false,
        0,
        Dcs::Gsm7Bit,
        &sample_scts(),
        2,
        &[0xE8, 0x34, 0xAA],
    );
    let m = decode(&buf).expect("decode");
    assert_eq!(m.text, "hi");
    assert_eq!(m.ud, vec![0xE8, 0x34]);

    // UCS-2: UDL 2 octets = one unit; trailing unit ignored.
    let buf = deliver(
        None,
        "1",
        false,
        0,
        Dcs::Ucs2,
        &sample_scts(),
        2,
        &[0x00, 0x41, 0x00, 0x42],
    );
    let m = decode(&buf).expect("decode");
    assert_eq!(m.text, "A");
}

/// Pathological SMSC length octets: 0 (absent), 1 (TOA-only, zero digits),
/// and absurd values beyond the buffer (0xFF, 0x7F) must all be handled
/// without panic.
#[test]
fn smsc_len_pathologies() {
    // SMSC len 0 -> None.
    let buf = deliver(
        None,
        "1",
        false,
        0,
        Dcs::EightBit,
        &sample_scts(),
        1,
        &[0x41],
    );
    let m = decode(&buf).expect("decode");
    assert_eq!(m.smsc, None);

    // SMSC len 1: only the TOA octet, zero digits -> Some("").
    let mut one = buf.clone();
    one[0] = 0x01;
    one.insert(1, 0x81);
    let m = decode(&one).expect("decode");
    assert_eq!(m.smsc.as_deref(), Some(""));

    // Absurd SMSC lengths overrun the buffer -> Truncated.
    for len in [0x7Fu8, 0xFF] {
        let mut big = buf.clone();
        big[0] = len;
        assert_eq!(decode(&big), Err(PduError::Truncated), "smsc len {len}");
        assert_eq!(decode(&[len]), Err(PduError::Truncated));
    }
}

/// Pathological UDH: UDHI with empty UD, UDHL beyond the UD, UDHL=0.
#[test]
fn udh_pathologies() {
    // UDHI set but no user data at all -> Truncated (no UDHL byte).
    let mut buf = deliver(
        None,
        "1",
        false,
        0,
        Dcs::Ucs2,
        &sample_scts(),
        0,
        &[],
    );
    buf[1] = 0x44; // set TP-UDHI
    assert_eq!(decode(&buf), Err(PduError::Truncated));

    // UDHL claims more octets than the UD carries -> Truncated.
    let mut buf = deliver(
        None,
        "1",
        false,
        0,
        Dcs::Ucs2,
        &sample_scts(),
        2,
        &[0x05, 0x00], // UDHL 5 but only 1 further octet
    );
    buf[1] = 0x44;
    assert_eq!(decode(&buf), Err(PduError::Truncated));

    // UDHL = 0: no IEs, udh stays None, whole UD is payload.
    let mut buf = deliver(
        None,
        "1",
        false,
        0,
        Dcs::Ucs2,
        &sample_scts(),
        3,
        &[0x00, 0x4F, 0x60],
    );
    buf[1] = 0x44;
    let m = decode(&buf).expect("decode");
    assert!(m.udh.is_none());
    assert_eq!(m.text, "你");
}

/// Random bytes spliced into the SCTS field: invalid calendar/BCD values
/// must surface as Invalid, never a panic.
#[test]
fn random_scts_never_panic() {
    let mut rng = Lcg::new(0x5EED5EED);
    for _ in 0..2000 {
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
        for slot in buf.iter_mut().take(14).skip(7) {
            *slot = rng.byte();
        }
        let _ = decode(&buf);
    }
}

/// Wrong TP-MTI (bits 1..0 != 00) is rejected as "not SMS-DELIVER".
#[test]
fn wrong_tp_mti_rejected() {
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
    buf[1] = 0x05; // SMS-STATUS-REPORT style MTI bits
    assert_eq!(decode(&buf), Err(PduError::Invalid("not SMS-DELIVER")));
}
