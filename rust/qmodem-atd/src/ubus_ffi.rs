//! Minimal libubus/libubox C FFI for the atd ubus object (feature `ubus`,
//! default off) — declared by hand, no bindgen, zero external crates.
//!
//! ## What is declared
//!
//! The subset of libubus/libubox needed to expose the daemon's method
//! table (`open`/`close`/`sendat`/`list`, ubus_at_daemon.h:269-293) over
//! real ubus:
//!
//! * [`ubus_connect`]/[`ubus_free`] — connection lifetime ([`UbusConnection`]
//!   wraps it in RAII);
//! * [`ubus_lookup`]/[`ubus_lookup_id`] — object discovery;
//! * [`ubus_add_object`] — publish the method table ([`register_atd_object`]
//!   builds the `repr(C)` [`ubus_method`] table for `sendat`/`list`);
//! * [`ubus_send_reply`] — per-call reply;
//! * [`blobmsg_parse`] — the one libubox decoder the C path needs.
//!
//! ## Why the C handler can reuse the pure-Rust codec
//!
//! libubox serializes to exactly the wire format implemented in
//! [`crate::ubus_proto`] (4-byte-aligned TLV, `type<<28 | len` header,
//! blobmsg name header, native endianness). The `sendat` handler below
//! slices the incoming `blob_attr` and decodes it with
//! [`crate::ubus_proto::decode_msg`], then runs the *same*
//! [`crate::ubus_proto::mock::dispatch_sendat`] the mock transport uses —
//! one code path, tested on every host, no libubus required for CI.
//!
//! ## Linking
//!
//! Declarations compile standalone (no symbols referenced until a caller
//! uses them), so host builds and `cargo check --features ubus` pass
//! without libubus. For real linking (OpenWrt SDK sysroot):
//!
//! 1. stage libubus/libubox headers+libs,
//! 2. `cargo build --release --features ubus \
//!      --config 'target.<triplet>.linker="<staging>/...-gcc"' \
//!      --config 'rustflags=["-L", "native=<staging>/usr/lib", "-lubus", "-lubox"]'`
//! 3. a host smoke check can link /tmp stub libraries instead (see
//!    tests/ubus_ffi_smoke.rs and docs/refactor/journal-ffi-ubus.md).
//!
//! ## Design notes
//!
//! The C daemon marshals ubus calls into the per-port queue; the Rust
//! daemon keeps that shape — the handler formats the request, blocks on
//! `AtHandle::send_at`'s oneshot reply (ubus handlers are synchronous),
//! and encodes the reply; it never touches transport state directly.
//! Rust state is reached via the C-idiomatic embedding pattern: the
//! daemon owns an [`AtdUbusObject`] (struct with a leading
//! [`ubus_object`]) and recovers it from the handler argument by
//! `offset_of!`/`container_of`.

#![allow(non_camel_case_types)]

use std::ffi::{c_char, CStr, CString};

use crate::registry::Registry;
use crate::ubus_proto::{self, mock};

/// Opaque ubus context (libubus.h).
pub enum ubus_context {}
/// Opaque ubus object id request data (libubus.h).
pub enum ubus_request_data {}
/// Opaque blob attribute — the wire format is implemented in
/// [`crate::ubus_proto`]; this type exists for pointer typing only.
#[repr(C)]
pub struct blob_attr {
    /// `(type << 28) | data_len`, native endian (libubox `id_len`).
    pub id_len: u32,
    /// Flexible array member; accessed by raw pointer arithmetic.
    pub data: [u8; 0],
}

pub const UBUS_STATUS_OK: i32 = 0;
pub const UBUS_STATUS_INVALID_ARGUMENT: i32 = 2;
pub const UBUS_STATUS_METHOD_NOT_FOUND: i32 = 3;
pub const UBUS_STATUS_NOT_FOUND: i32 = 4;

/// blobmsg attribute types (libubox blobmsg.h).
pub const BLOBMSG_TYPE_STRING: u32 = 3;
pub const BLOBMSG_TYPE_INT32: u32 = 5;

/// `ubus_handler_t` (libubus.h): receives the connection, the object, the
/// request slot, the method name and the message blob.
pub type ubus_handler_t = Option<
    unsafe extern "C" fn(
        ctx: *mut ubus_context,
        obj: *mut ubus_object,
        req: *mut ubus_request_data,
        method: *const c_char,
        msg: *const blob_attr,
    ) -> i32,
>;

/// `ubus_lookup_handler_t` (libubus.h).
pub type ubus_lookup_handler_t = Option<
    unsafe extern "C" fn(
        ctx: *mut ubus_context,
        obj: *mut ubus_object,
        priv_: *mut std::ffi::c_void,
    ),
>;

/// blobmsg field policy (libubox blobmsg.h `struct blobmsg_policy`).
#[repr(C)]
pub struct blobmsg_policy {
    pub name: *const c_char,
    pub msg_type: u32,
}

/// ubus method descriptor (libubus.h `struct ubus_method`).
#[repr(C)]
pub struct ubus_method {
    pub name: *const c_char,
    pub handler: ubus_handler_t,
    pub policy: *const blobmsg_policy,
    pub n_policy: u8,
}

/// ubus object type (libubus.h `struct ubus_object_type`).
#[repr(C)]
pub struct ubus_object_type {
    pub name: *const c_char,
    pub id: u32,
    pub methods: *const ubus_method,
    pub n_methods: i32,
}

/// ubus object (libubus.h `struct ubus_object`; the subscriber list heads
/// are opaque here — libubus only ever initializes/reads them itself).
#[repr(C)]
pub struct ubus_object {
    pub name: *const c_char,
    pub id: u32,
    pub obj_type: *mut ubus_object_type,
    pub refcount: i32,
    pub has_subscribers: bool,
    /// `struct list_head subscribers` (opaque, libubus-owned).
    pub subscribers: [u8; 16],
    /// `struct list_head auto_delete` (opaque, libubus-owned).
    pub auto_delete: [u8; 16],
}

extern "C" {
    /// `ubus_connect(path: Option<&CStr>) -> *mut ubus_context`
    pub fn ubus_connect(path: *const c_char) -> *mut ubus_context;
    pub fn ubus_free(ctx: *mut ubus_context);
    pub fn ubus_lookup(
        ctx: *mut ubus_context,
        path: *const c_char,
        cb: ubus_lookup_handler_t,
        priv_: *mut std::ffi::c_void,
    ) -> i32;
    pub fn ubus_lookup_id(ctx: *mut ubus_context, path: *const c_char, id: *mut u32) -> i32;
    pub fn ubus_add_object(ctx: *mut ubus_context, obj: *mut ubus_object) -> i32;
    pub fn ubus_send_reply(
        ctx: *mut ubus_context,
        req: *mut ubus_request_data,
        msg: *const blob_attr,
    ) -> i32;
    pub fn blobmsg_parse(
        policy: *const blobmsg_policy,
        policy_len: i32,
        tb: *mut *const blob_attr,
        data: *const blob_attr,
        len: usize,
    ) -> i32;
}

/// blob_len(attr): payload length from the header word (libubox blob.h).
///
/// # Safety
/// `attr` must point to a valid libubox blob_attr.
pub unsafe fn blob_len(attr: *const blob_attr) -> usize {
    (unsafe { std::ptr::read(attr) }.id_len & 0x0fff_ffff) as usize
}

/// blob_type(attr): attribute type from the header word.
///
/// # Safety
/// `attr` must point to a valid libubox blob_attr.
pub unsafe fn blob_type(attr: *const blob_attr) -> u32 {
    unsafe { std::ptr::read(attr) }.id_len >> 28
}

/// RAII ubus connection (ubus_connect/ubus_free).
pub struct UbusConnection {
    ctx: *mut ubus_context,
}

// The ubus context is used from the single ubus poll thread; the daemon
// keeps all calls on that thread (same ownership discipline as the C
// daemon's ubus thread).
unsafe impl Send for UbusConnection {}

impl UbusConnection {
    /// Connect to the given ubus socket path (None = default /var/run/ubus).
    ///
    /// # Safety
    /// Must not race with libubus calls from other threads.
    pub unsafe fn connect(path: Option<&CStr>) -> Option<UbusConnection> {
        let p = match path {
            Some(s) => s.as_ptr(),
            None => std::ptr::null(),
        };
        let ctx = unsafe { ubus_connect(p) };
        if ctx.is_null() {
            None
        } else {
            Some(UbusConnection { ctx })
        }
    }

    pub fn ctx(&self) -> *mut ubus_context {
        self.ctx
    }
}

impl Drop for UbusConnection {
    fn drop(&mut self) {
        unsafe { ubus_free(self.ctx) };
    }
}

// ---------------------------------------------------------------------------
// atd object: method table + sendat handler
// ---------------------------------------------------------------------------

// Method/field names for the atd ubus object. c-string literals (const
// stable) keep these usable without lazy init.
mod names {
    use std::ffi::CStr;
    pub const OBJ: &CStr = c"qmodem.atd";
    pub const SENDAT: &CStr = c"sendat";
    pub const LIST: &CStr = c"list";
    pub const F_PORT: &CStr = c"port";
    pub const F_AT: &CStr = c"at";
    pub const F_TIMEOUT: &CStr = c"timeout_ms";
    pub const F_END_FLAG: &CStr = c"end_flag";
}

// Policy for the `sendat` method (mirrors the C daemon's sendat policy);
// built at registration time (raw-pointer statics would need unsafe Sync).
const SENDAT_POLICY_TYPES: [(&CStr, u32); 4] = [
    (names::F_PORT, BLOBMSG_TYPE_STRING),
    (names::F_AT, BLOBMSG_TYPE_STRING),
    (names::F_TIMEOUT, BLOBMSG_TYPE_INT32),
    (names::F_END_FLAG, BLOBMSG_TYPE_STRING),
];

/// Daemon-side state embedded behind the ubus object (C container_of
/// pattern). Must be heap- or static-pinned: libubus keeps raw pointers
/// into it after [`ubus_add_object`].
pub struct AtdUbusObject {
    pub obj: ubus_object,
    pub registry: *const Registry,
}

const OBJ_FIELD_OFFSET: usize = std::mem::offset_of!(AtdUbusObject, obj);

/// `unsafe extern "C" fn` handler for the `sendat` method. Decodes the
/// request blob with the pure-Rust codec, blocks on the per-port command
/// queue (ubus handlers are synchronous, same shape as the C daemon) and
/// encodes the reply blob.
unsafe extern "C" fn atd_sendat_handler(
    _ctx: *mut ubus_context,
    obj: *mut ubus_object,
    req: *mut ubus_request_data,
    _method: *const c_char,
    msg: *const blob_attr,
) -> i32 {
    let state_ptr =
        unsafe { (obj as *mut u8).sub(OBJ_FIELD_OFFSET).cast::<AtdUbusObject>().as_ref() };
    let Some(state) = state_ptr else {
        return UBUS_STATUS_INVALID_ARGUMENT;
    };
    let registry_ref = unsafe { state.registry.as_ref() };
    let Some(registry) = registry_ref else {
        return UBUS_STATUS_INVALID_ARGUMENT;
    };

    // The msg attribute is a TABLE whose payload is the message body —
    // slice it and decode with the shared pure-Rust codec.
    let len = unsafe { blob_len(msg) };
    let data_ptr = unsafe { std::ptr::addr_of!((*msg).data) };
    let body = unsafe { std::slice::from_raw_parts(data_ptr as *const u8, len) };
    let Ok(attrs) = ubus_proto::decode_msg(body) else {
        return UBUS_STATUS_INVALID_ARGUMENT;
    };

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
        return UBUS_STATUS_INVALID_ARGUMENT;
    };
    let Some(handle) = registry.get(&port) else {
        return UBUS_STATUS_NOT_FOUND;
    };

    // Reuse the exact mock-path reply encoder, then hand the buffer to
    // libubox as a blob_attr (identical wire format, native endian).
    match mock::encode_sendat_result(&handle, &at, timeout_ms, end_flag.as_deref()) {
        Ok(reply) => {
            let msg_attr = wrap_as_blob_attr(&reply);
            unsafe { ubus_send_reply(_ctx, req, msg_attr.as_ptr() as *const blob_attr) }
        }
        Err(status) => status,
    }
}

/// Wrap an encoded message body into a standalone TABLE `blob_attr`
/// (4-byte header word + body), ready for [`ubus_send_reply`].
fn wrap_as_blob_attr(msg_body: &[u8]) -> Vec<u8> {
    let word = ((ubus_proto::types::BLOBMSG_TYPE_TABLE as u32) << 28)
        | msg_body.len() as u32;
    let mut out = Vec::with_capacity(4 + msg_body.len());
    out.extend(word.to_ne_bytes());
    out.extend(msg_body);
    out
}

/// Build and register the `qmodem.atd` ubus object exposing `sendat` and
/// `list`. All C-reachable memory (name strings, method table, the object
/// itself) is intentionally leaked — libubus keeps pointers to it for the
/// lifetime of the connection, mirroring the C daemon's static tables.
///
/// # Safety
/// - `registry` must outlive the ubus connection;
/// - the returned object must never be freed while the connection lives;
/// - all libubus calls must stay on one thread.
pub unsafe fn register_atd_object(
    ctx: *mut ubus_context,
    registry: &Registry,
) -> Result<&'static mut AtdUbusObject, i32> {
    let sendat_policy: Box<[blobmsg_policy]> = SENDAT_POLICY_TYPES
        .iter()
        .map(|(name, ty)| blobmsg_policy { name: name.as_ptr(), msg_type: *ty })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let methods: Box<[ubus_method]> = vec![
        ubus_method {
            name: names::SENDAT.as_ptr(),
            handler: Some(atd_sendat_handler),
            policy: sendat_policy.as_ptr(),
            n_policy: sendat_policy.len() as u8,
        },
        ubus_method {
            name: names::LIST.as_ptr(),
            handler: Some(atd_list_handler),
            policy: std::ptr::null(),
            n_policy: 0,
        },
    ]
    .into_boxed_slice();
    let obj_type: Box<ubus_object_type> = Box::new(ubus_object_type {
        name: names::OBJ.as_ptr(),
        id: 0, // assigned by ubusd on add_object
        methods: methods.as_ptr(),
        n_methods: methods.len() as i32,
    });
    let obj_name: CString = names::OBJ.to_owned();
    let state: Box<AtdUbusObject> = Box::new(AtdUbusObject {
        obj: ubus_object {
            name: obj_name.as_ptr(),
            id: 0,
            obj_type: Box::into_raw(obj_type),
            refcount: 0,
            has_subscribers: false,
            subscribers: [0u8; 16],
            auto_delete: [0u8; 16],
        },
        registry,
    });

    // Leak everything libubus will keep pointing at.
    Box::leak(sendat_policy);
    Box::leak(methods);
    Box::leak(Box::new(obj_name));
    let state = Box::leak(state);

    let status = unsafe { ubus_add_object(ctx, &mut state.obj) };
    if status == UBUS_STATUS_OK {
        Ok(state)
    } else {
        Err(status)
    }
}

/// `list` method handler: sorted port names.
unsafe extern "C" fn atd_list_handler(
    _ctx: *mut ubus_context,
    obj: *mut ubus_object,
    req: *mut ubus_request_data,
    _method: *const c_char,
    _msg: *const blob_attr,
) -> i32 {
    let state_ptr =
        unsafe { (obj as *mut u8).sub(OBJ_FIELD_OFFSET).cast::<AtdUbusObject>().as_ref() };
    let Some(state) = state_ptr else {
        return UBUS_STATUS_INVALID_ARGUMENT;
    };
    let registry_ref = unsafe { state.registry.as_ref() };
    let Some(registry) = registry_ref else {
        return UBUS_STATUS_INVALID_ARGUMENT;
    };
    match mock::dispatch_list(registry) {
        Ok(reply) => {
            let msg_attr = wrap_as_blob_attr(&reply);
            unsafe { ubus_send_reply(_ctx, req, msg_attr.as_ptr() as *const blob_attr) }
        }
        Err(status) => status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The wire format is byte-identical across the FFI/mock boundary:
    /// wrap_as_blob_attr must produce a header word + body that
    /// ubus_proto::decode round-trips.
    #[test]
    fn wrap_as_blob_attr_roundtrip() {
        let body = ubus_proto::encode_msg(&[ubus_proto::Attr::string("status", "ok")]);
        let wrapped = wrap_as_blob_attr(&body);
        assert_eq!(wrapped.len() % 4, 0);
        let word = u32::from_ne_bytes([wrapped[0], wrapped[1], wrapped[2], wrapped[3]]);
        assert_eq!(word >> 28, ubus_proto::types::BLOBMSG_TYPE_TABLE as u32);
        assert_eq!((word & 0x0fff_ffff) as usize, body.len());
        let back = ubus_proto::decode_msg(&wrapped[4..]).unwrap();
        assert_eq!(back[0].as_str(), Some("ok"));
    }

    /// Registered names must match the C daemon's method table spelling
    /// (ubus_at_daemon.h:269-293) — locked here to catch accidental edits.
    #[test]
    fn method_table_names() {
        assert_eq!(names::OBJ, c"qmodem.atd");
        assert_eq!(names::SENDAT, c"sendat");
        assert_eq!(names::LIST, c"list");
        assert_eq!(names::F_PORT, c"port");
        assert_eq!(names::F_AT, c"at");
        assert_eq!(names::F_TIMEOUT, c"timeout_ms");
        assert_eq!(names::F_END_FLAG, c"end_flag");
    }
}
