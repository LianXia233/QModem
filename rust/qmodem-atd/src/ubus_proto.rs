//! Pure-Rust blobmsg codec (libubox-compatible wire format) plus a mock
//! ubus transport for hosts without libubus (zero external crates).
//!
//! ## Wire format (mirrors libubox/blob.h + blobmsg.h)
//!
//! Every attribute is a TLV, 4-byte aligned:
//!
//! ```text
//! u32 header (native endian): (type << 28) | data_len
//! [named attrs only] blobmsg_hdr: u16 namelen, name bytes, NUL, pad4
//! data_len bytes of payload (strings include the trailing NUL;
//! nested payloads are concatenated 4-byte-aligned child attributes)
//! ```
//!
//! `data_len` covers the blobmsg header (if any) plus the payload, exactly
//! like `blob_len()` in libubox. Header/payload use native endianness so a
//! buffer produced here can be handed to the C side as a `blob_attr*`
//! (and vice versa) on the little-endian OpenWrt targets.
//!
//! Because the C handler receives a `blob_attr*` that is byte-identical to
//! what [`encode_msg`] produces, the feature-`ubus` FFI handler and the
//! mock transport share one decoder ([`decode_msg`]) — same semantics on
//! both paths.
//!
//! ## Mock transport ([`mock`])
//!
//! [`mock::MockUbusServer`] dispatches named methods over encoded blobmsg
//! bodies, mirroring `ubus_handler_t` (bytes in, bytes out, ubus status
//! code on error). The `sendat`/`list` dispatch lives in
//! [`mock::dispatch_sendat`]/[`mock::dispatch_list`] and is reused verbatim
//! by the C FFI handler, so `cargo test` (default features) validates the
//! exact code path that runs behind `--features ubus`.

use std::fmt;

/// blobmsg attribute types (libubox/blob-msg.h numeric values).
pub mod types {
    pub const BLOBMSG_TYPE_UNSPEC: u8 = 0;
    pub const BLOBMSG_TYPE_ARRAY: u8 = 1;
    pub const BLOBMSG_TYPE_TABLE: u8 = 2;
    pub const BLOBMSG_TYPE_STRING: u8 = 3;
    pub const BLOBMSG_TYPE_INT64: u8 = 4;
    pub const BLOBMSG_TYPE_INT32: u8 = 5;
    pub const BLOBMSG_TYPE_INT16: u8 = 6;
    pub const BLOBMSG_TYPE_INT8: u8 = 7;
    pub const BLOBMSG_TYPE_DOUBLE: u8 = 8;
}

const BLOB_ATTR_ALIGN: usize = 4;
const LEN_MASK: u32 = 0x0fff_ffff;

/// Codec error. Every failure mode is recoverable (no panics on hostile
/// input — the C `blobmsg_parse` callers rely on libubox for this).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Buffer ends inside a header/payload/padding region.
    Truncated,
    /// Attribute length exceeds the enclosing buffer.
    LengthOverflow,
    /// blobmsg header namelen lies or the name is not NUL-terminated.
    BadName,
    /// String payload is missing its trailing NUL or is not UTF-8.
    BadString,
    /// Integer payload length does not match the declared type.
    BadInt,
    /// Attribute type not supported by this minimal codec.
    UnsupportedType(u8),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated => write!(f, "truncated blobmsg buffer"),
            Error::LengthOverflow => write!(f, "attribute length overflows buffer"),
            Error::BadName => write!(f, "bad blobmsg attribute name"),
            Error::BadString => write!(f, "bad blobmsg string payload"),
            Error::BadInt => write!(f, "bad blobmsg integer payload"),
            Error::UnsupportedType(t) => write!(f, "unsupported blobmsg type {t}"),
        }
    }
}

impl std::error::Error for Error {}

/// Payload of one attribute. This minimal codec covers the types the atd
/// ubus object needs (`sendat`/`list`); extend as methods are ported.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    U32(u32),
    U64(u64),
    /// TABLE (named children) or ARRAY (unnamed children), keyed by the
    /// enclosing attribute's type.
    Nested(Vec<Attr>),
}

/// One blobmsg attribute: type, optional name, payload.
#[derive(Debug, Clone, PartialEq)]
pub struct Attr {
    pub id: u8,
    pub name: Option<String>,
    pub value: Value,
}

impl Attr {
    pub fn string(name: &str, v: impl Into<String>) -> Attr {
        Attr { id: types::BLOBMSG_TYPE_STRING, name: Some(name.to_string()), value: Value::String(v.into()) }
    }

    pub fn u32(name: &str, v: u32) -> Attr {
        Attr { id: types::BLOBMSG_TYPE_INT32, name: Some(name.to_string()), value: Value::U32(v) }
    }

    pub fn u64(name: &str, v: u64) -> Attr {
        Attr { id: types::BLOBMSG_TYPE_INT64, name: Some(name.to_string()), value: Value::U64(v) }
    }

    /// TABLE element: named children.
    pub fn table(name: &str, children: Vec<Attr>) -> Attr {
        Attr { id: types::BLOBMSG_TYPE_TABLE, name: Some(name.to_string()), value: Value::Nested(children) }
    }

    /// ARRAY element: unnamed children.
    pub fn array(name: &str, children: Vec<Attr>) -> Attr {
        Attr { id: types::BLOBMSG_TYPE_ARRAY, name: Some(name.to_string()), value: Value::Nested(children) }
    }

    /// Unnamed attribute (element of an array).
    pub fn item(v: impl Into<String>) -> Attr {
        Attr { id: types::BLOBMSG_TYPE_STRING, name: None, value: Value::String(v.into()) }
    }

    pub fn as_str(&self) -> Option<&str> {
        match &self.value {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        match &self.value {
            Value::U32(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_children(&self) -> Option<&[Attr]> {
        match &self.value {
            Value::Nested(c) => Some(c),
            _ => None,
        }
    }
}

fn pad4(n: usize) -> usize {
    n.div_ceil(BLOB_ATTR_ALIGN) * BLOB_ATTR_ALIGN
}

/// Encode a top-level attribute list (the body of a ubus message: a table
/// of named fields). Result is 4-byte aligned, ready to be passed to the
/// C side as `blob_attr` data (skip the first 4 header bytes when handing
/// it over as an attribute payload).
pub fn encode_msg(attrs: &[Attr]) -> Vec<u8> {
    let mut buf = Vec::new();
    for a in attrs {
        encode_attr_into(&mut buf, a);
    }
    buf
}

/// Encode one attribute (header + data, padded). Used for byte-level
/// golden tests and for nesting.
pub fn encode_attr(attr: &Attr) -> Vec<u8> {
    let mut buf = Vec::new();
    encode_attr_into(&mut buf, attr);
    buf
}

fn encode_attr_into(buf: &mut Vec<u8>, attr: &Attr) {
    let mut payload: Vec<u8> = Vec::new();
    if let Some(name) = &attr.name {
        let n = name.as_bytes();
        debug_assert!(n.len() <= u16::MAX as usize, "blobmsg name too long");
        let mut hdr = Vec::with_capacity(n.len() + 3);
        hdr.extend((n.len() as u16).to_ne_bytes());
        hdr.extend(n);
        hdr.push(0);
        hdr.resize(pad4(hdr.len()), 0);
        payload.extend(hdr);
    }
    match &attr.value {
        Value::String(s) => {
            payload.extend(s.as_bytes());
            payload.push(0);
        }
        Value::U32(v) => payload.extend(v.to_ne_bytes()),
        Value::U64(v) => payload.extend(v.to_ne_bytes()),
        Value::Nested(children) => {
            for c in children {
                encode_attr_into(&mut payload, c);
            }
        }
    }
    let len = payload.len();
    assert!(len <= LEN_MASK as usize, "blobmsg payload too large");
    let word = ((attr.id as u32) << 28) | len as u32;
    buf.extend(word.to_ne_bytes());
    buf.extend(payload);
    let pad = pad4(len) - len;
    buf.resize(buf.len() + pad, 0);
}

/// Decode a top-level ubus message body (named fields, like the `msg`
/// attribute a `ubus_handler_t` receives).
pub fn decode_msg(data: &[u8]) -> Result<Vec<Attr>, Error> {
    decode_attrs(data, true)
}

/// Decode raw attribute data where children are unnamed (array payload).
pub fn decode_array_items(data: &[u8]) -> Result<Vec<Attr>, Error> {
    decode_attrs(data, false)
}

fn decode_attrs(mut data: &[u8], named: bool) -> Result<Vec<Attr>, Error> {
    let mut out = Vec::new();
    while !data.is_empty() {
        if data.len() < BLOB_ATTR_ALIGN {
            return Err(Error::Truncated);
        }
        let word = u32::from_ne_bytes([data[0], data[1], data[2], data[3]]);
        // A zero header word (UNSPEC with len 0) acts as an end marker,
        // mirroring the libubox convention of skipping such attributes.
        if word == 0 {
            break;
        }
        let id = (word >> 28) as u8;
        let len = (word & LEN_MASK) as usize;
        data = &data[BLOB_ATTR_ALIGN..];
        let padded = pad4(len);
        if data.len() < len {
            return Err(Error::Truncated);
        }
        if data.len() < padded {
            return Err(Error::LengthOverflow);
        }
        let payload = &data[..len];
        data = &data[padded..];
        out.push(decode_attr(id, payload, named)?);
    }
    Ok(out)
}

fn decode_attr(id: u8, payload: &[u8], named: bool) -> Result<Attr, Error> {
    let (name, body) = if named {
        let (name, rest) = split_name(payload)?;
        (Some(name), rest)
    } else {
        (None, payload)
    };
    let value = match id {
        types::BLOBMSG_TYPE_STRING => {
            let Some(stripped) = body.strip_suffix(&[0][..]) else {
                return Err(Error::BadString);
            };
            Value::String(
                std::str::from_utf8(stripped)
                    .map_err(|_| Error::BadString)?
                    .to_string(),
            )
        }
        types::BLOBMSG_TYPE_INT32 => {
            if body.len() != 4 {
                return Err(Error::BadInt);
            }
            Value::U32(u32::from_ne_bytes([body[0], body[1], body[2], body[3]]))
        }
        types::BLOBMSG_TYPE_INT64 => {
            if body.len() != 8 {
                return Err(Error::BadInt);
            }
            let mut b = [0u8; 8];
            b.copy_from_slice(body);
            Value::U64(u64::from_ne_bytes(b))
        }
        types::BLOBMSG_TYPE_TABLE => Value::Nested(decode_attrs(body, true)?),
        types::BLOBMSG_TYPE_ARRAY => Value::Nested(decode_attrs(body, false)?),
        types::BLOBMSG_TYPE_UNSPEC if body.is_empty() => Value::Nested(Vec::new()),
        other => return Err(Error::UnsupportedType(other)),
    };
    Ok(Attr { id, name, value })
}

/// Strip the blobmsg header (u16 namelen + name + NUL, padded to 4) and
/// return (name, payload-after-header).
fn split_name(payload: &[u8]) -> Result<(String, &[u8]), Error> {
    if payload.len() < 3 {
        return Err(Error::BadName);
    }
    let namelen = u16::from_ne_bytes([payload[0], payload[1]]) as usize;
    let hdrlen = pad4(namelen + 3); // u16 namelen + name + NUL
    if payload.len() < hdrlen || payload.len() < 2 + namelen + 1 {
        return Err(Error::BadName);
    }
    if payload[2 + namelen] != 0 {
        return Err(Error::BadName);
    }
    let name = std::str::from_utf8(&payload[2..2 + namelen])
        .map_err(|_| Error::BadName)?
        .to_string();
    Ok((name, &payload[hdrlen..]))
}

// ---------------------------------------------------------------------------
// Mock ubus transport (same semantics as libubus, no C dependency)
// ---------------------------------------------------------------------------

pub mod mock {
    use super::{decode_msg, encode_msg, Attr};
    use crate::port::{AtHandle, ResponseStatus};
    use crate::registry::Registry;
    use std::collections::HashMap;
    use std::sync::Arc;

    /// ubus status codes (ubus.h). Only the ones the atd object uses.
    pub const UBUS_STATUS_OK: i32 = 0;
    pub const UBUS_STATUS_INVALID_ARGUMENT: i32 = 2;
    pub const UBUS_STATUS_METHOD_NOT_FOUND: i32 = 3;
    pub const UBUS_STATUS_NOT_FOUND: i32 = 4;
    pub const UBUS_STATUS_CONNECTION_FAILED: i32 = 10;

    /// A registered method: encoded blobmsg body in, encoded reply (or a
    /// ubus status code) out — the same contract as `ubus_handler_t`.
    pub type MethodHandler = Box<dyn Fn(&[u8]) -> Result<Vec<u8>, i32> + Send + Sync>;

    /// In-process stand-in for a ubus connection: named methods dispatching
    /// blobmsg bodies. Used by tests and by hosts without libubus; the
    /// feature-`ubus` FFI path reuses [`dispatch_sendat`]/[`dispatch_list`]
    /// inside its C handler, so both transports behave identically.
    #[derive(Default)]
    pub struct MockUbusServer {
        methods: HashMap<String, MethodHandler>,
    }

    impl MockUbusServer {
        pub fn new() -> MockUbusServer {
            MockUbusServer::default()
        }

        pub fn add_method(&mut self, name: &str, handler: MethodHandler) {
            self.methods.insert(name.to_string(), handler);
        }

        /// `ubus call <object> <method> '<json>'` equivalent.
        pub fn call(&self, method: &str, body: &[u8]) -> Result<Vec<u8>, i32> {
            match self.methods.get(method) {
                Some(h) => h(body),
                None => Err(UBUS_STATUS_METHOD_NOT_FOUND),
            }
        }
    }

    /// Decode a sendat request ({port, at, timeout_ms, end_flag?}), run it
    /// through the registry (blocking call — ubus handlers are synchronous,
    /// same shape as the C daemon) and encode the reply.
    pub fn dispatch_sendat(registry: &Registry, body: &[u8]) -> Result<Vec<u8>, i32> {
        let attrs = decode_msg(body).map_err(|_| UBUS_STATUS_INVALID_ARGUMENT)?;
        let mut port = None;
        let mut at = None;
        let mut timeout_ms: u64 = 3000;
        let mut end_flag = None;
        for a in &attrs {
            match (a.name.as_deref(), a) {
                (Some("port"), v) => port = v.as_str().map(str::to_string),
                (Some("at"), v) => at = v.as_str().map(str::to_string),
                (Some("timeout_ms"), v) => timeout_ms = v.as_u32().unwrap_or(3000) as u64,
                (Some("end_flag"), v) => end_flag = v.as_str().map(str::to_string),
                _ => {}
            }
        }
        let (Some(port), Some(at)) = (port, at) else {
            return Err(UBUS_STATUS_INVALID_ARGUMENT);
        };
        let Some(handle) = registry.get(&port) else {
            return Err(UBUS_STATUS_NOT_FOUND);
        };
        encode_sendat_result(&handle, &at, timeout_ms, end_flag.as_deref())
    }

    /// Shared by the mock server and the feature-`ubus` C handler.
    pub fn encode_sendat_result(
        handle: &AtHandle,
        at: &str,
        timeout_ms: u64,
        end_flag: Option<&str>,
    ) -> Result<Vec<u8>, i32> {
        match handle.send_at(at, timeout_ms, end_flag) {
            Ok(resp) => {
                let lines: Vec<Attr> = resp.lines.iter().map(|l| Attr::item(l.clone())).collect();
                let status = match resp.status {
                    ResponseStatus::Ok => "ok",
                    ResponseStatus::Timeout => "timeout",
                };
                Ok(encode_msg(&[
                    Attr::string("status", status),
                    Attr::array("lines", lines),
                    Attr::string("end_flag", resp.end_flag.clone()),
                    Attr::u32("elapsed_ms", resp.elapsed_ms as u32),
                ]))
            }
            Err(_) => Err(UBUS_STATUS_CONNECTION_FAILED),
        }
    }

    /// `list` method: sorted port names.
    pub fn dispatch_list(registry: &Registry) -> Result<Vec<u8>, i32> {
        let ports: Vec<Attr> = registry.list().into_iter().map(Attr::item).collect();
        Ok(encode_msg(&[Attr::array("ports", ports)]))
    }

    /// Wire the atd method table (`sendat`, `list`; ubus_at_daemon.h:269-293)
    /// onto a mock server.
    pub fn register_atd_methods(server: &mut MockUbusServer, registry: Arc<Registry>) {
        let r = registry.clone();
        server.add_method(
            "sendat",
            Box::new(move |body| dispatch_sendat(&r, body)),
        );
        let r = registry.clone();
        server.add_method("list", Box::new(move |_body| dispatch_list(&r)));
    }

    /// Client-side helpers: build the `ubus call sendat` request body.
    pub fn encode_sendat_request(
        port: &str,
        at: &str,
        timeout_ms: u64,
        end_flag: Option<&str>,
    ) -> Vec<u8> {
        let mut attrs = vec![
            Attr::string("port", port),
            Attr::string("at", at),
            Attr::u32("timeout_ms", timeout_ms as u32),
        ];
        if let Some(f) = end_flag {
            attrs.push(Attr::string("end_flag", f));
        }
        encode_msg(&attrs)
    }

    /// Decoded `sendat` reply.
    #[derive(Debug, Clone, PartialEq)]
    pub struct SendatReply {
        pub status: String,
        pub lines: Vec<String>,
        pub end_flag: String,
        pub elapsed_ms: u32,
    }

    /// Client-side helper: parse a sendat reply body.
    pub fn decode_sendat_reply(body: &[u8]) -> Result<SendatReply, super::Error> {
        let attrs = decode_msg(body)?;
        let mut reply = SendatReply {
            status: String::new(),
            lines: Vec::new(),
            end_flag: String::new(),
            elapsed_ms: 0,
        };
        for a in &attrs {
            match (a.name.as_deref(), a) {
                (Some("status"), v) => reply.status = v.as_str().unwrap_or_default().to_string(),
                (Some("lines"), v) => {
                    reply.lines = v
                        .as_children()
                        .unwrap_or(&[])
                        .iter()
                        .filter_map(|c| c.as_str().map(str::to_string))
                        .collect();
                }
                (Some("end_flag"), v) => reply.end_flag = v.as_str().unwrap_or_default().to_string(),
                (Some("elapsed_ms"), v) => reply.elapsed_ms = v.as_u32().unwrap_or(0),
                _ => {}
            }
        }
        Ok(reply)
    }

    /// Client-side helper: parse a `list` reply body.
    pub fn decode_list_reply(body: &[u8]) -> Result<Vec<String>, super::Error> {
        let attrs = decode_msg(body)?;
        Ok(attrs
            .iter()
            .find(|a| a.name.as_deref() == Some("ports"))
            .and_then(|a| a.as_children().map(|c| c.to_vec()))
            .unwrap_or_default()
            .iter()
            .filter_map(|c| c.as_str().map(str::to_string))
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Unit tests: TLV roundtrip + hostile input
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::mock::*;
    use super::types::*;
    use super::*;
    use crate::registry::Registry;
    use std::sync::Arc;

    /// Byte-level golden: {port: "usb2"} — locks the libubox layout
    /// (header word, blobmsg hdr with u16 namelen, NUL-terminated string,
    /// 4-byte alignment). Endianness follows the host, like libubox.
    #[test]
    fn golden_bytes_named_string_attr() {
        let got = encode_attr(&Attr::string("port", "usb2"));
        let mut want = Vec::new();
        // header: type 3, data_len = pad4(2+4+1) + (4+1) = 8 + 5 = 13
        want.extend((0x3000_000Du32).to_ne_bytes());
        // blobmsg hdr: namelen=4, "port\0", padded to 8 (pad4(7)=8 -> 1 pad)
        want.extend(4u16.to_ne_bytes());
        want.extend(b"port");
        want.push(0);
        want.extend([0u8; 1]);
        // payload: "usb2\0" padded to 8
        want.extend(b"usb2");
        want.push(0);
        want.extend([0u8; 3]);
        assert_eq!(got, want);
    }

    #[test]
    fn scalar_fields_roundtrip() {
        let msg = [Attr::string("port", "/dev/ttyUSB2"), Attr::u32("timeout_ms", 3000), Attr::u64("seq", 0x1234_5678_9abc_def0)];
        let bytes = encode_msg(&msg);
        let back = decode_msg(&bytes).unwrap();
        assert_eq!(back, msg.to_vec());
    }

    #[test]
    fn nested_table_and_array_roundtrip() {
        let msg = [Attr::table(
            "meta",
            vec![
                Attr::u32("epoch", 7),
                Attr::array("lines", vec![Attr::item("+CSQ: 24,99"), Attr::item("OK")]),
            ],
        )];
        let bytes = encode_msg(&msg);
        let back = decode_msg(&bytes).unwrap();
        assert_eq!(back, msg.to_vec());
        // Descend manually to verify structure.
        let meta = &back[0];
        assert_eq!(meta.id, BLOBMSG_TYPE_TABLE);
        let children = meta.as_children().unwrap();
        assert_eq!(children[0].as_u32(), Some(7));
        let arr = &children[1];
        assert_eq!(arr.id, BLOBMSG_TYPE_ARRAY);
        assert_eq!(arr.name.as_deref(), Some("lines"));
        let items = arr.as_children().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].as_str(), Some("+CSQ: 24,99"));
        assert!(items[0].name.is_none(), "array children are unnamed");
    }

    #[test]
    fn array_of_tables_roundtrip() {
        // Convention (mirrors blobmsg_open_array + blobmsg_open_table):
        // children of an ARRAY are unnamed; children of a TABLE are named.
        let unnamed_table = |name: &str, epoch: u32| Attr {
            id: BLOBMSG_TYPE_TABLE,
            name: None,
            value: Value::Nested(vec![Attr::string("name", name), Attr::u32("epoch", epoch)]),
        };
        let msg = [Attr {
            id: BLOBMSG_TYPE_ARRAY,
            name: Some("ports".to_string()),
            value: Value::Nested(vec![unnamed_table("ttyUSB0", 1), unnamed_table("ttyUSB1", 2)]),
        }];
        let bytes = encode_msg(&msg);
        let back = decode_msg(&bytes).unwrap();
        assert_eq!(back, msg.to_vec());
        let items = back[0].as_children().unwrap();
        assert_eq!(items[1].as_children().unwrap()[0].as_str(), Some("ttyUSB1"));
    }

    #[test]
    fn empty_containers_roundtrip() {
        let msg = [Attr::array("empty", vec![]), Attr::table("nothing", vec![])];
        let bytes = encode_msg(&msg);
        let back = decode_msg(&bytes).unwrap();
        assert_eq!(back, msg.to_vec());
    }

    #[test]
    fn empty_message_and_zero_word_terminator() {
        assert!(decode_msg(&[]).unwrap().is_empty());
        let mut bytes = encode_msg(&[Attr::string("a", "b")]);
        bytes.extend([0u8; 4]); // zero header word = end marker, must be ignored
        let back = decode_msg(&bytes).unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].as_str(), Some("b"));
    }

    #[test]
    fn multi_pad_boundaries() {
        // Names/strings whose lengths land on every padding residue.
        for n in 0..12usize {
            let name = "n".repeat(n);
            let value = "v".repeat(n * 5 + 1);
            let attr = Attr::string(&name, &value);
            let bytes = encode_msg(std::slice::from_ref(&attr));
            assert_eq!(bytes.len() % 4, 0, "alignment broken at name len {n}");
            assert_eq!(decode_msg(&bytes).unwrap(), vec![attr]);
        }
    }

    /// No prefix of an encoded message may panic; it must either decode to
    /// a shorter-but-valid message or return an error (libubox robustness).
    #[test]
    fn truncated_input_never_panics() {
        let full = encode_msg(&[
            Attr::string("port", "/dev/ttyUSB2"),
            Attr::u32("timeout_ms", 3000),
            Attr::table("meta", vec![Attr::array("lines", vec![Attr::item("OK")])]),
        ]);
        for cut in 0..full.len() {
            let _ = decode_msg(&full[..cut]);
        }
        // Specific truncations must error, not silently succeed.
        assert_eq!(decode_msg(&full[..2]), Err(Error::Truncated));
        assert_eq!(decode_msg(&full[..6]), Err(Error::Truncated));
    }

    #[test]
    fn lying_lengths_and_bad_names_rejected() {
        // Attr::string("ab", "c") layout: header(4) + name hdr(8:
        // u16 namelen=2, "ab", NUL, 3 pad) + payload "c\0"(2) + pad(2).
        // Absolute offsets: 0 header word; 4 namelen; 6..8 "ab"; 8 NUL of
        // name; 12 'c'; 13 string NUL.
        let base = || encode_msg(&[Attr::string("ab", "c")]);

        // Declared length beyond the buffer (corrupt the header word).
        let mut bytes = base();
        bytes[0..4].copy_from_slice(&((3u32 << 28) | 0x0100).to_ne_bytes());
        assert_eq!(decode_msg(&bytes), Err(Error::Truncated));

        // Declared length fits the buffer but its 4-byte padding does not.
        // Full buffer is 16 B: header(4) + name hdr(8) + "c\0"(2) + pad(2).
        // Truncate to 15 B and claim len=11: 11 raw bytes present, but
        // pad4(11)=12 > 11 remaining -> LengthOverflow.
        let mut bytes = base();
        bytes.truncate(15);
        bytes[0..4].copy_from_slice(&((3u32 << 28) | 11).to_ne_bytes());
        assert_eq!(decode_msg(&bytes), Err(Error::LengthOverflow));

        // Header word claims more payload than remains.
        let mut bytes = base();
        bytes.truncate(4 + 6); // keep only header + 6 payload bytes (of 10)
        bytes[0..4].copy_from_slice(&((3u32 << 28) | 7).to_ne_bytes());
        assert_eq!(decode_msg(&bytes), Err(Error::Truncated));

        // namelen larger than the actual name.
        let mut bytes = base();
        bytes[4] = 9;
        assert_eq!(decode_msg(&bytes), Err(Error::BadName));

        // Name not NUL-terminated.
        let mut bytes = base();
        bytes[8] = b'x'; // NUL after name -> 'x'
        assert_eq!(decode_msg(&bytes), Err(Error::BadName));

        // String payload without trailing NUL.
        let mut bytes = base();
        bytes[13] = b'!';
        assert_eq!(decode_msg(&bytes), Err(Error::BadString));

        // Non-UTF-8 string.
        let mut bytes = base();
        bytes[12] = 0xff;
        assert_eq!(decode_msg(&bytes), Err(Error::BadString));

        // Wrong integer width.
        let mut bytes = encode_msg(&[Attr::u32("n", 1)]);
        bytes.truncate(bytes.len() - 1); // 4-byte payload cut to 3 (+pad gone)
        assert!(decode_msg(&bytes).is_err());

        // Unsupported type id.
        let word = (12u32 << 28) | 4;
        let mut bytes = word.to_ne_bytes().to_vec();
        bytes.extend(1u32.to_ne_bytes());
        assert_eq!(decode_msg(&bytes), Err(Error::UnsupportedType(12)));
    }

    #[test]
    fn mock_unknown_method() {
        let server = MockUbusServer::new();
        assert_eq!(server.call("sendat", &[]), Err(UBUS_STATUS_METHOD_NOT_FOUND));
    }

    #[test]
    fn mock_dispatch_sendat_roundtrip() {
        use crate::port::{spawn_port, AtPortConfig};
        use crate::transport::{LoopbackPort, SerialPort};

        let lp = LoopbackPort::new();
        let h = lp.handle();
        h.set_responder(Box::new(|cmd| vec![format!("ACK[{cmd}]"), "OK".to_string()]));
        let boxed: Box<dyn SerialPort> = Box::new(lp);
        let port = spawn_port("usb0", boxed, 0, AtPortConfig::default());
        let registry = Arc::new(Registry::new());
        registry.insert("usb0", port);

        let mut server = MockUbusServer::new();
        register_atd_methods(&mut server, registry.clone());

        // ok path
        let req = encode_sendat_request("usb0", "AT+CSQ", 1000, None);
        let reply = server.call("sendat", &req).unwrap();
        let r = decode_sendat_reply(&reply).unwrap();
        assert_eq!(r.status, "ok");
        assert_eq!(r.end_flag, "OK");
        assert_eq!(r.lines, vec!["ACK[AT+CSQ]", "OK"]);
        assert!(r.elapsed_ms <= 1000);

        // unknown port -> UBUS_STATUS_NOT_FOUND
        let req = encode_sendat_request("nope", "AT", 1000, None);
        assert_eq!(server.call("sendat", &req), Err(UBUS_STATUS_NOT_FOUND));

        // missing "at" -> INVALID_ARGUMENT
        let req = encode_msg(&[Attr::string("port", "usb0")]);
        assert_eq!(server.call("sendat", &req), Err(UBUS_STATUS_INVALID_ARGUMENT));

        // list
        let reply = server.call("list", &[]).unwrap();
        assert_eq!(decode_list_reply(&reply).unwrap(), vec!["usb0".to_string()]);
    }

    #[test]
    fn mock_dispatch_sendat_timeout() {
        use crate::port::AtPortConfig;
        use crate::transport::{LoopbackPort, SerialPort};

        let lp = LoopbackPort::new();
        let h = lp.handle();
        h.set_responder(Box::new(|_cmd| vec!["+CSQ: 21,99".to_string()]));
        let boxed: Box<dyn SerialPort> = Box::new(lp);
        let port = crate::port::spawn_port("usb1", boxed, 0, AtPortConfig::default());
        let registry = Arc::new(Registry::new());
        registry.insert("usb1", port);

        let mut server = MockUbusServer::new();
        register_atd_methods(&mut server, registry);

        let req = encode_sendat_request("usb1", "AT+CSQ", 80, None);
        let reply = server.call("sendat", &req).unwrap();
        let r = decode_sendat_reply(&reply).unwrap();
        assert_eq!(r.status, "timeout");
        assert_eq!(r.lines, vec!["+CSQ: 21,99"]);
        assert_eq!(r.end_flag, "");
    }
}
