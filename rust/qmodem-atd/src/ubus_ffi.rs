//! libubus C FFI skeleton (feature `ubus`, default off).
//!
//! ## Why it is a skeleton
//!
//! The daemon's production transport on OpenWrt is ubus, reached today via
//! the C daemon's `ubus_sendat_method` etc. (ubus_at_daemon.h:269-293).
//! Porting it requires linking `libubus`/`libubox`, which host build
//! environments (including CI) do not provide, so this module is feature
//! gated and contains declarations + wiring notes only. It compiles
//! standalone without the library present (no symbol references).
//!
//! ## Enabling for cross-compilation
//!
//! 1. Build OpenWrt SDK sysroot with libubus/libubox staging.
//! 2. `cargo build --release --features ubus \
//!      --config 'target.xxx.linker="staging_dir/...-gcc"' \
//!      --config 'rustflags=["-L", "staging_dir/usr/lib", ...]'`
//! 3. Implement `register_ubus_object` using the declared FFI prototypes,
//!    mirroring the C methods: open/close/sendat/list/event_callback*.

#![allow(non_camel_case_types)]

/// Opaque ubus context (libubus.h).
pub enum ubus_context {}
/// Opaque ubus object (libubus.h).
pub enum ubus_object {}
/// Opaque request data (libubus.h).
pub enum ubus_request_data {}
/// Opaque blob attribute (libubox/blobmsg.h).
pub enum blob_attr {}

pub const UBUS_STATUS_OK: i32 = 0;

pub type ubus_handler_t =
    Option<unsafe extern "C" fn(ctx: *mut ubus_context, obj: *mut ubus_object,
        req: *mut ubus_request_data, method: *const i8, msg: *mut blob_attr) -> i32>;

extern "C" {
    /// ubus_connect(path: Option<&c_char>) -> *mut ubus_context
    pub fn ubus_connect(path: *const i8) -> *mut ubus_context;
    pub fn ubus_free(ctx: *mut ubus_context);
    pub fn ubus_add_object(ctx: *mut ubus_context, obj: *mut ubus_object) -> i32;
    pub fn ubus_send_reply(ctx: *mut ubus_context, req: *mut ubus_request_data,
        msg: *mut blob_attr) -> i32;
    pub fn blobmsg_parse(attr: *const blob_attr, max: usize, tb: *mut *mut blob_attr,
        data: *const blob_attr, len: usize) -> i32;
}

/// Attach a ubus object exposing `sendat` / `list` on top of the async
/// port registry. Wire-up sketch (not compiled until `--features ubus`):
///
/// ```ignore
/// pub unsafe fn register_ubus_object(ctx: *mut ubus_context) -> i32 {
///     // 1. build ubus_object with method table:
///     //    "sendat" -> handler that forwards to the per-port std mpsc
///     //    queue (ubus handlers are not async).
///     // 2. ubus_add_object(ctx, &mut obj)
///     UBUS_STATUS_OK
/// }
/// ```
///
/// Design note: the C daemon marshals ubus calls into the per-port queue;
/// the Rust daemon keeps that shape — the ubus handler only formats an
/// [`crate::port::AtRequest`] and blocks on the request's oneshot reply
/// channel, never touching transport state directly.
pub struct UbusSkeleton;

impl UbusSkeleton {
    /// Placeholder documenting the intended integration point.
    pub fn describe() -> &'static str {
        "ubus FFI skeleton: enable with --features ubus when libubus is available"
    }
}
