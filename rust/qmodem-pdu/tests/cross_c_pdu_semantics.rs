//! Cross-validation of the Rust codec against the semantics of the C
//! reference implementation `application/qmodem/src/libqmodem-sms/src/pdu.c`.
//!
//! Every test cites the exact pdu.c lines its vector is derived from.
//! Where the Rust rewrite intentionally diverges from C (documented in
//! the crate header), the divergence is asserted explicitly.

mod common;

use common::{deliver, deliver_with_udh, hex, pack7, sample_scts, semi};
use qmodem_pdu::{decode, encode_sms_submit, Dcs, Udh};

// ---------------------------------------------------------------------------
// 1. 7-bit packing — pdu.c:44-55 (dreamfabric "hellohello" worked example)
//    and EncodePDUMessage at pdu.c:59-85.
//    The comment block pins the exact octet layout:
//    E8 32 9B FD 46 97 D9 EC 37 for the 10 septets "hellohello".
// ---------------------------------------------------------------------------

#[test]
fn c_7bit_packing_hellohello() {
    let septets: Vec<u8> = b"hellohello".to_vec();
    assert_eq!(pack7(&septets), hex("E8329BFD4697D9EC37"));
    // Same data through a complete SMS-DELIVER (pdu.c:540-649 layout).
    let buf = deliver(
        None,
        "10086",
        false,
        0,
        Dcs::Gsm7Bit,
        &sample_scts(),
        10,
        &hex("E8329BFD4697D9EC37"),
    );
    let m = decode(&buf).expect("decode");
    assert_eq!(m.text, "hellohello");
    assert_eq!(m.sender, "10086");
}

// ---------------------------------------------------------------------------
// 2. Septet reconstruction across octet boundaries — DecodePDUMessage_GSM_7bit
//    pdu.c:89-119: the carry_on_bits loop (pdu.c:99-113) reassembles septets
//    from consecutive octets; the remainder is taken at pdu.c:114-115.
// ---------------------------------------------------------------------------

#[test]
fn c_7bit_unpack_boundaries() {
    // 8 septets -> exactly 7 octets (no trailing partial octet).
    let septets8: Vec<u8> = b"12345678".to_vec();
    let octets = pack7(&septets8);
    assert_eq!(octets.len(), 7);
    let buf = deliver(
        None,
        "1",
        false,
        0,
        Dcs::Gsm7Bit,
        &sample_scts(),
        8,
        &octets,
    );
    assert_eq!(decode(&buf).unwrap().text, "12345678");

    // 3 septets -> 3 octets with 4 filler bits (pdu.c:63: (len*7+7)/8).
    let septets3: Vec<u8> = b"abc".to_vec();
    let buf = deliver(
        None,
        "1",
        false,
        0,
        Dcs::Gsm7Bit,
        &sample_scts(),
        3,
        &pack7(&septets3),
    );
    assert_eq!(decode(&buf).unwrap().text, "abc");
}

// ---------------------------------------------------------------------------
// 3. Basic alphabet -> Latin-1 mapping — gsm7bits_to_latin1, pdu.c:123-132.
//    Row 0: '@' 0xa3 '$' 0xa5 0xe8 ...; 0x0a '\n', 0x0d '\r';
//    0x1c 0xc6 'Æ', 0x1d 0xe6 'æ', 0x1e 0xdf 'ß'.
// ---------------------------------------------------------------------------

#[test]
fn c_gsm7_latin1_table() {
    let sept = [0x00u8, 0x01, 0x02, 0x04, 0x0a, 0x0d, 0x1c, 0x1d, 0x1e];
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
    assert_eq!(m.text, "@£$è\n\rÆæß");
}

// ---------------------------------------------------------------------------
// 4. Escape table — gsm7bits_extend_to_latin1, pdu.c:134-143: 0x0a->\f,
//    0x14->^, 0x28->{, 0x29->}, 0x2f->\, 0x3c->[, 0x3d->~, 0x3e->],
//    0x40->|. G7bitToAscii resolves escape pairs at pdu.c:255-263.
//    Divergence (documented, lib.rs gsm7_ext_char): septet 0x65 maps to
//    '€' in the Rust table but is 0/NPC in the C table (pdu.c:139-142).
// ---------------------------------------------------------------------------

#[test]
fn c_gsm7_extend_table() {
    let pairs: &[(u8, char)] = &[
        (0x0a, '\u{0c}'),
        (0x14, '^'),
        (0x28, '{'),
        (0x29, '}'),
        (0x2f, '\\'),
        (0x3c, '['),
        (0x3d, '~'),
        (0x3e, ']'),
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
    assert_eq!(decode(&buf).unwrap().text, expected);
}

// ---------------------------------------------------------------------------
// 5. UDH concat IE 8-bit reference — parse_udh, pdu.c:205-208:
//    iei 0x00 && iedl 3 -> ref = buffer[pos+2], total = buffer[pos+3],
//    part = buffer[pos+4].
// ---------------------------------------------------------------------------

#[test]
fn c_parse_udh_concat_ie00() {
    let udh = hex("050003990201"); // ref 0x99, total 2, part 1
    let buf = deliver_with_udh(
        "1",
        false,
        Dcs::Ucs2,
        &sample_scts(),
        &udh,
        &hex("4F60"),
    );
    let m = decode(&buf).expect("decode");
    assert_eq!(
        m.udh,
        Some(Udh {
            ref_num: 0x99,
            total_parts: 2,
            part_num: 1
        })
    );
}

// ---------------------------------------------------------------------------
// 6. UDH concat IE 16-bit reference — parse_udh, pdu.c:209-213:
//    iei 0x08 && iedl 4 -> ref = (buffer[pos+2]<<8)|buffer[pos+3],
//    total = buffer[pos+4], part = buffer[pos+5].
// ---------------------------------------------------------------------------

#[test]
fn c_parse_udh_concat_ie08() {
    let udh = hex("060804DEAD0201");
    let buf = deliver_with_udh(
        "1",
        false,
        Dcs::Ucs2,
        &sample_scts(),
        &udh,
        &hex("4F60"),
    );
    let m = decode(&buf).expect("decode");
    assert_eq!(
        m.udh,
        Some(Udh {
            ref_num: 0xDEAD,
            total_parts: 2,
            part_num: 1
        })
    );
}

// ---------------------------------------------------------------------------
// 7. Concat total <= 1 — parse_udh, pdu.c:216-218 zeroes the concat info
//    when concat_total <= 1. Intentional divergence: the Rust codec keeps
//    the raw IE visible as `Some(Udh { total_parts: 1, .. })` instead of
//    hiding it, so callers can still see the raw header.
// ---------------------------------------------------------------------------

#[test]
fn c_parse_udh_total_le_1_divergence() {
    let udh = hex("050003550101"); // total = 1
    let buf = deliver_with_udh(
        "1",
        false,
        Dcs::Ucs2,
        &sample_scts(),
        &udh,
        &hex("4F60"),
    );
    let m = decode(&buf).expect("decode");
    // C: concat_ref = concat_total = concat_part = 0 (pdu.c:216-218).
    // Rust: raw IE preserved.
    assert_eq!(
        m.udh,
        Some(Udh {
            ref_num: 0x55,
            total_parts: 1,
            part_num: 1
        })
    );
}

// ---------------------------------------------------------------------------
// 8. Sender address semi-octet decoding — DecodePhoneNumber, pdu.c:351-363:
//    even digit index takes buffer[i/2] low nibble, odd takes high nibble.
//    Divergence (lib.rs decode_address): TOA international (0x91) gets a
//    "+" prefix in Rust; the C decoder never adds one (pdu.c:576-577).
// ---------------------------------------------------------------------------

#[test]
fn c_sender_semi_octet_and_plus_prefix() {
    // Even digit count: "123456" national (TOA 0x81 per pdu.c:23).
    let buf = deliver(
        None,
        "123456",
        false,
        0,
        Dcs::EightBit,
        &sample_scts(),
        1,
        &[0x41],
    );
    let m = decode(&buf).expect("decode");
    assert_eq!(m.sender, "123456");

    // Odd digit count: last nibble is 0xF padding (pdu.c:338-345 encodes it).
    let buf = deliver(
        None,
        "9876543",
        true,
        0,
        Dcs::EightBit,
        &sample_scts(),
        1,
        &[0x41],
    );
    let m = decode(&buf).expect("decode");
    assert_eq!(semi("9876543"), vec![0x89, 0x67, 0x45, 0xF3]);
    assert_eq!(m.sender, "+9876543");
}

// ---------------------------------------------------------------------------
// 9. DCS routing — pdu_decode, pdu.c:614: `switch ((*tp_dcs / 4) % 4)`:
//    case 0 -> GSM 7-bit, case 2 -> UCS2 (raw copy, pdu.c:632-636),
//    default -> untouched buffer. Rust Dcs::from_tp_dcs (lib.rs) uses
//    (v >> 2) & 0x03 — the same arithmetic — with 8-bit as the 0x01 case.
// ---------------------------------------------------------------------------

#[test]
fn c_dcs_group_routing() {
    // DCS 0x0C: (0x0C/4)%4 = 0 -> 7-bit in C; (0x0C>>2)&0x03 = 3 -> the
    // Rust default arm is also Gsm7Bit. Text: "ok" septets 6F 6B.
    let buf = deliver(
        None,
        "1",
        false,
        0,
        Dcs::Gsm7Bit,
        &sample_scts(),
        2,
        &pack7(&[0x6F, 0x6B]),
    );
    let mut pdu = buf;
    pdu[6] = 0x0C; // TP-DCS position with empty SMSC
    let m = decode(&pdu).expect("decode");
    assert_eq!(m.dcs, Dcs::Gsm7Bit);
    assert_eq!(m.text, "ok");

    // DCS 0x04: (0x04/4)%4 = 1 -> C leaves the buffer untouched; the Rust
    // codec renders it as Latin-1 style text (lib.rs EightBit arm).
    let buf = deliver(
        None,
        "1",
        false,
        0,
        Dcs::EightBit,
        &sample_scts(),
        3,
        &[0x61, 0x62, 0x63],
    );
    let m = decode(&buf).expect("decode");
    assert_eq!(m.dcs, Dcs::EightBit);
    assert_eq!(m.text, "abc");

    // DCS 0x08: (0x08/4)%4 = 2 -> UCS2. C copies raw UCS-2 bytes
    // (pdu.c:635); Rust converts to UTF-8 (documented divergence,
    // crate header + decode_ucs2, lib.rs).
    let buf = deliver(
        None,
        "1",
        false,
        0,
        Dcs::Ucs2,
        &sample_scts(),
        2,
        &[0x00, 0x41],
    );
    let m = decode(&buf).expect("decode");
    assert_eq!(m.dcs, Dcs::Ucs2);
    assert_eq!(m.text, "A");
}

// ---------------------------------------------------------------------------
// 10. SMS-SUBMIT layout — pdu_encode, pdu.c:366-437:
//     [0]=SMSC len (pdu.c:385-386), 0x11 SMS_SUBMIT (pdu.c:391,29),
//     0x00 MR (pdu.c:392), addr len (pdu.c:395), TOA 0x81 if <6 digits
//     else 0x91 (pdu.c:397-401), EncodePhoneNumber (pdu.c:403),
//     0x00 PID (pdu.c:414), 0x00 DCS (pdu.c:415), 0xB0 VP (pdu.c:416),
//     UDL in septets (pdu.c:428).
// ---------------------------------------------------------------------------

#[test]
fn c_encode_sms_submit_layout() {
    // C always emits DCS 0x00 (7-bit, pdu.c:415); "Hi" -> septets 48 69,
    // packed into 2 octets C8 34, UDL 2 (pdu.c:425-428).
    let pdu = encode_sms_submit("1234567", "Hi", Dcs::Gsm7Bit).unwrap();
    assert_eq!(pdu, hex("0011000791214365F70000B002C834"));
    // Field-by-field, with pdu.c line references:
    assert_eq!(pdu[0], 0x00); // SMSC length 0            (pdu.c:385)
    assert_eq!(pdu[1], 0x11); // SMS_SUBMIT               (pdu.c:391)
    assert_eq!(pdu[2], 0x00); // TP-MR                    (pdu.c:392)
    assert_eq!(pdu[3], 7); // addr len in digits         (pdu.c:395)
    assert_eq!(pdu[4], 0x91); // >= 6 digits -> intl TOA  (pdu.c:397-401)
    assert_eq!(&pdu[5..9], &hex("214365F7")); // semi-octet digits (pdu.c:403)
    assert_eq!(pdu[9], 0x00); // TP-PID                   (pdu.c:414)
    assert_eq!(pdu[10], 0x00); // TP-DCS                  (pdu.c:415)
    assert_eq!(pdu[11], 0xB0); // TP-VP 10 days           (pdu.c:416)
    assert_eq!(pdu[12], 2); // TP-UDL in septets          (pdu.c:428)
    assert_eq!(&pdu[13..], &hex("C834"));

    // Short number (< 6 digits) -> TYPE_OF_ADDRESS_UNKNOWN 0x81
    // (pdu.c:397-398; also the enum at pdu.c:23).
    let short = encode_sms_submit("1234", "a", Dcs::Gsm7Bit).unwrap();
    assert_eq!(short[4], 0x81);
}

// ---------------------------------------------------------------------------
// 11. UDH filler septets — pdu_decode 7-bit branch, pdu.c:622-624:
//     skip_septets = (skip_bytes * 8 + 6) / 7, i.e. ceil(UDH octets*8/7).
//     A 6-octet UDH occupies 7 septets; the text starts after them.
// ---------------------------------------------------------------------------

#[test]
fn c_udh_skip_septets_formula() {
    // IEI 0x08 concat: 1 (UDHL) + 6 = 7... use a 6-octet IE block:
    // UDHL=6 -> skip_bytes 7 (1+6), skip_septets = (7*8+6)/7 = 8? No:
    // C's skip_bytes = udh.bytes = udhl + 1 (pdu.c:193) = 7, giving
    // (7*8+6)/7 = 8 filler septets. The Rust codec computes
    // ceil(udh_octets*8/7) with udh_octets = 1+udhl = 7 -> 8. Same.
    let udh = hex("06080411220302"); // UDHL 6 -> udh_bytes 7 -> 8 filler septets
    let buf = deliver_with_udh(
        "1",
        false,
        Dcs::Gsm7Bit,
        &sample_scts(),
        &udh,
        b"world",
    );
    let m = decode(&buf).expect("decode");
    assert_eq!(m.text, "world");
    // m.ud is the re-packed 7-bit data (EncodePDUMessage layout), not ASCII.
    assert_eq!(m.ud, hex("F7B79C4D06"));
    // TP-UDL counts all septets: 8 filler + 5 text.
    assert_eq!(buf[14], 13);
}

// ---------------------------------------------------------------------------
// 12. Unmapped characters -> NPC '?' — AsciiToG7bit, pdu.c:274/295-319:
//     latin1_to_gsm7bits holds NPC '?' for unmapped input, and multi-byte
//     UTF-8 input collapses to one NPC (pdu.c:307-314). Rust encodes
//     unsupported chars to septet 0x3F ('?') the same way (lib.rs
//     char_to_septets fallback).
// ---------------------------------------------------------------------------

#[test]
fn c_ascii_to_g7bit_npc() {
    // CJK "中" has no GSM-7 mapping; encode_sms_submit must fall back to
    // '?' per septet: 2 chars -> 2 septets (0x3F 0x3F) -> octets BF 1F.
    let pdu = encode_sms_submit("10086", "中中", Dcs::Gsm7Bit).unwrap();
    assert_eq!(&pdu[pdu.len() - 3..], &hex("02BF1F"));
    // And '?' decodes back as a literal question mark.
    let udl = pdu[pdu.len() - 3];
    let ud = pdu[pdu.len() - 2..].to_vec();
    let buf = deliver(
        None,
        "1",
        false,
        0,
        Dcs::Gsm7Bit,
        &sample_scts(),
        udl,
        &ud,
    );
    assert_eq!(decode(&buf).unwrap().text, "??");
}
