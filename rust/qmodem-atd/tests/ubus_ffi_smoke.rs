//! FFI smoke tests (feature `ubus`). These exercise the hand-declared
//! libubus/libubox surface against minimal stub libraries that mirror the
//! real signatures — on hosts without libubus they link via:
//!
//! ```text
//! RUSTFLAGS="-L native=<stubdir> -lubus -lubox" \
//!   cargo test -p qmodem-atd --features ubus
//! ```
//!
//! What they validate:
//! 1. the declaration/linkage layer (symbols resolve, RAII connect/free
//!    round trip works);
//! 2. the wire-format bridge: a buffer produced by the pure-Rust codec
//!    (`ubus_proto`) parses correctly through the C-side
//!    `blobmsg_parse` with a policy table — i.e. both sides agree on the
//!    libubox TLV layout, which is what lets the real handler reuse the
//!    Rust decoder;
//! 3. object registration plumbing (`register_atd_object` with the stub
//!    `ubus_add_object`).
//!
//! The sendat/list call *semantics* (request decode -> AtHandle ->
//! reply encode) are covered transport-independently by
//! tests/ubus_mock.rs, since both transports share that code.
#![cfg(feature = "ubus")]

use std::ffi::{CStr, CString};

use qmodem_atd::registry::Registry;
use qmodem_atd::ubus_ffi::*;
use qmodem_atd::ubus_proto;

/// Read one parsed named field the way a C consumer would: header word at
/// the attr, blobmsg name header inside the data, payload after it.
/// Returns (type, name-payload) — mirrors blobmsg_data/blobmsg_data_len.
unsafe fn field_payload(attr: *const blob_attr) -> (u32, Vec<u8>) {
    unsafe {
        let word = std::ptr::read(attr as *const u32);
        let total_len = (word & 0x0fff_ffff) as usize;
        let data = std::ptr::addr_of!((*attr).data) as *const u8;
        let namelen = std::ptr::read(data as *const u16) as usize;
        let hdrlen = (namelen + 3 + 3) & !3; // pad4(2 + namelen + 1)
        let payload = std::slice::from_raw_parts(data.add(hdrlen), total_len - hdrlen);
        (word >> 28, payload.to_vec())
    }
}

/// Symbol resolution + RAII lifetime through the stub libubus.
#[test]
fn connect_and_free_roundtrip() {
    unsafe {
        let conn = UbusConnection::connect(None).expect("stub ubus_connect must succeed");
        assert!(!conn.ctx().is_null());
        let mut id: u32 = 0;
        let path = CString::new("qmodem.atd").unwrap();
        assert_eq!(ubus_lookup_id(conn.ctx(), path.as_ptr(), &mut id), 0);
        assert_eq!(id, 1);
        drop(conn); // Drop -> ubus_free
    }
}

/// Register the atd object through the stub: the method table is reachable
/// and shaped as declared (names, policy count).
#[test]
fn register_atd_object_via_stub() {
    unsafe {
        let conn = UbusConnection::connect(None).unwrap();
        let registry = Registry::new();
        let state = register_atd_object(conn.ctx(), &registry)
            .expect("register_atd_object must return OK");
        assert!(!state.obj.name.is_null());
        let t = state.obj.obj_type.as_ref().expect("obj_type set");
        assert_eq!(t.n_methods, 2);
        let methods = std::slice::from_raw_parts(t.methods, 2);
        assert_eq!(CStr::from_ptr(methods[0].name), c"sendat");
        assert_eq!(methods[0].n_policy, 4);
        assert!(methods[0].handler.is_some());
        assert_eq!(CStr::from_ptr(methods[1].name), c"list");
        assert_eq!(methods[1].n_policy, 0);
    }
}

/// The core bridge claim: Rust-encoded blobmsg == libubox blobmsg.
/// Encode the sendat request with the pure-Rust codec and parse it through
/// the C stub's `blobmsg_parse` with a policy table; compare what C sees
/// with what the Rust decoder sees.
#[test]
fn rust_encoded_blob_parses_via_c_blobmsg_parse() {
    let body = ubus_proto::encode_msg(&[
            ubus_proto::Attr::string("port", "/dev/ttyUSB2"),
            ubus_proto::Attr::string("at", "AT+CSQ"),
            ubus_proto::Attr::u32("timeout_ms", 3000),
            ubus_proto::Attr::string("end_flag", "OK"),
        ]);
        // Wrap the message body into a TABLE blob_attr, exactly like
        // ubus_ffi::wrap_as_blob_attr does for ubus_send_reply.
        let word = (2u32 << 28) | body.len() as u32;
        let mut wrapped = Vec::with_capacity(4 + body.len());
        wrapped.extend(word.to_ne_bytes());
        wrapped.extend(&body);
        let blob = wrapped.as_ptr() as *const blob_attr;
        // Message body = payload after the 4-byte header (what a real
        // handler passes as blobmsg_data(msg)).
        let data = unsafe {
            std::slice::from_raw_parts(
                std::ptr::addr_of!((*blob).data) as *const u8,
                body.len(),
            )
        };

        let policy = [
            blobmsg_policy { name: c"port".as_ptr(), msg_type: BLOBMSG_TYPE_STRING },
            blobmsg_policy { name: c"at".as_ptr(), msg_type: BLOBMSG_TYPE_STRING },
            blobmsg_policy { name: c"timeout_ms".as_ptr(), msg_type: BLOBMSG_TYPE_INT32 },
            blobmsg_policy { name: c"end_flag".as_ptr(), msg_type: BLOBMSG_TYPE_STRING },
        ];
        let mut tb: [*const blob_attr; 4] = [std::ptr::null(); 4];
        let rc = unsafe {
            blobmsg_parse(
                policy.as_ptr(),
                policy.len() as i32,
                tb.as_mut_ptr(),
                data.as_ptr() as *const blob_attr,
                body.len(),
            )
        };
        assert_eq!(rc, 0, "C blobmsg_parse must accept the Rust-encoded blob");
        for slot in tb.iter() {
            assert!(!slot.is_null(), "all policy fields must match");
        }

        // C-side view: payload bytes and types match the request.
        let (ty, payload) = unsafe { field_payload(tb[0]) };
        assert_eq!(ty, BLOBMSG_TYPE_STRING);
        assert_eq!(&payload, b"/dev/ttyUSB2\0");
        let (_, payload) = unsafe { field_payload(tb[1]) };
        assert_eq!(&payload, b"AT+CSQ\0");
        let (ty, payload) = unsafe { field_payload(tb[2]) };
        assert_eq!(ty, BLOBMSG_TYPE_INT32);
        assert_eq!(payload.len(), 4);
        assert_eq!(
            u32::from_ne_bytes([payload[0], payload[1], payload[2], payload[3]]),
            3000
        );
        let (_, payload) = unsafe { field_payload(tb[3]) };
        assert_eq!(&payload, b"OK\0");

        // Rust-side view of the same bytes agrees with the C parse.
        let rust_view = ubus_proto::decode_msg(data).unwrap();
        assert_eq!(rust_view[0].name.as_deref(), Some("port"));
        assert_eq!(rust_view[0].as_str(), Some("/dev/ttyUSB2"));
        assert_eq!(rust_view[2].as_u32(), Some(3000));
        assert_eq!(rust_view[3].as_str(), Some("OK"));

        // blob_type/blob_len helpers on the wrapper attr.
        assert_eq!(unsafe { blob_type(blob) }, 2); // TABLE
        assert_eq!(unsafe { blob_len(blob) }, body.len());
}
