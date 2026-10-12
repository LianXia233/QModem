//! Integration test: a full "ubus call sendat" round trip over the mock
//! transport (default features, no libubus required).
//!
//! The mock path shares its dispatch code (`dispatch_sendat` /
//! `dispatch_list` / `encode_sendat_result`) with the feature-`ubus` C
//! handler, so this validates the exact semantics that run behind
//! `--features ubus` on OpenWrt.

use std::sync::Arc;

use qmodem_atd::port::{spawn_port, AtPortConfig};
use qmodem_atd::registry::Registry;
use qmodem_atd::transport::{LoopbackHandle, LoopbackPort, SerialPort};
use qmodem_atd::ubus_proto::mock::{
    self, decode_list_reply, decode_sendat_reply, encode_sendat_request, MockUbusServer,
    UBUS_STATUS_INVALID_ARGUMENT, UBUS_STATUS_METHOD_NOT_FOUND, UBUS_STATUS_NOT_FOUND,
};
use qmodem_atd::ubus_proto::{encode_msg, types, Attr, Value};

/// Build a registry with one loopback port answering `ACK[<cmd>]` + OK.
/// The handle stays usable after the port is boxed (same pattern as the
/// existing integration tests).
fn mock_daemon() -> (Arc<Registry>, LoopbackHandle) {
    let lp = LoopbackPort::new();
    let h = lp.handle();
    h.set_responder(Box::new(|cmd| vec![format!("ACK[{cmd}]"), "OK".to_string()]));
    let boxed: Box<dyn SerialPort> = Box::new(lp);
    let registry = Arc::new(Registry::new());
    registry.insert("usb0", spawn_port("usb0", boxed, 0, AtPortConfig::default()));
    (registry, h)
}

fn server_with(registry: Arc<Registry>) -> MockUbusServer {
    let mut server = MockUbusServer::new();
    mock::register_atd_methods(&mut server, registry);
    server
}

/// `ubus call qmodem.atd sendat '{"port":"usb0","at":"AT+CSQ",...}'`:
/// encode request blob -> dispatch -> decode reply blob.
#[test]
fn ubus_call_sendat_ok() {
    let (registry, _lp) = mock_daemon();
    let server = server_with(registry);

    let req = encode_sendat_request("usb0", "AT+CSQ", 2000, None);
    let reply = server.call("sendat", &req).expect("sendat must succeed");
    let r = decode_sendat_reply(&reply).expect("reply must decode");

    assert_eq!(r.status, "ok");
    assert_eq!(r.end_flag, "OK");
    assert_eq!(r.lines, vec!["ACK[AT+CSQ]", "OK"]);
    assert!(r.elapsed_ms <= 2000);
}

/// Custom end flag flows through the blob just like the JSON service.
#[test]
fn ubus_call_sendat_custom_end_flag() {
    let (registry, h) = mock_daemon();
    h.set_responder(Box::new(|_cmd| vec!["> ".to_string()]));
    let server = server_with(registry);

    let req = encode_sendat_request("usb0", "AT+CMGS", 1000, Some(">"));
    let r = decode_sendat_reply(&server.call("sendat", &req).unwrap()).unwrap();
    assert_eq!(r.status, "ok");
    assert_eq!(r.end_flag, ">");
}

/// Silent modem: status "timeout" plus the partial lines that arrived.
#[test]
fn ubus_call_sendat_timeout() {
    let lp = LoopbackPort::new();
    lp.handle()
        .set_responder(Box::new(|_cmd| vec!["+CSQ: 21,99".to_string()]));
    let boxed: Box<dyn SerialPort> = Box::new(lp);
    let registry = Arc::new(Registry::new());
    registry.insert("usb0", spawn_port("usb0", boxed, 0, AtPortConfig::default()));
    let server = server_with(registry);

    let req = encode_sendat_request("usb0", "AT+CSQ", 100, None);
    let r = decode_sendat_reply(&server.call("sendat", &req).unwrap()).unwrap();
    assert_eq!(r.status, "timeout");
    assert_eq!(r.lines, vec!["+CSQ: 21,99"]);
    assert_eq!(r.end_flag, "");
}

/// Unknown port -> UBUS_STATUS_NOT_FOUND (4), like ubusd object lookup.
#[test]
fn ubus_call_sendat_unknown_port() {
    let (registry, _lp) = mock_daemon();
    let server = server_with(registry);
    let req = encode_sendat_request("nope", "AT", 1000, None);
    assert_eq!(server.call("sendat", &req), Err(UBUS_STATUS_NOT_FOUND));
}

/// Missing mandatory field -> UBUS_STATUS_INVALID_ARGUMENT (2).
#[test]
fn ubus_call_sendat_missing_at() {
    let (registry, _lp) = mock_daemon();
    let server = server_with(registry);
    let req = encode_msg(&[Attr::string("port", "usb0")]);
    assert_eq!(server.call("sendat", &req), Err(UBUS_STATUS_INVALID_ARGUMENT));

    // Garbage blob (not decodable) is also INVALID_ARGUMENT, never a panic.
    assert_eq!(server.call("sendat", &[0xff, 0x00, 0x01]), Err(UBUS_STATUS_INVALID_ARGUMENT));
}

/// `ubus call qmodem.atd list` -> sorted port names.
#[test]
fn ubus_call_list() {
    let (registry, _lp) = mock_daemon();
    registry.insert(
        "usb1",
        spawn_port(
            "usb1",
            Box::new(LoopbackPort::new()),
            0,
            AtPortConfig::default(),
        ),
    );
    let server = server_with(registry);

    let reply = server.call("list", &[]).expect("list must succeed");
    let ports = decode_list_reply(&reply).expect("list reply must decode");
    assert_eq!(ports, vec!["usb0".to_string(), "usb1".to_string()]);
}

/// Unknown method -> UBUS_STATUS_METHOD_NOT_FOUND (3), like ubusd.
#[test]
fn ubus_call_unknown_method() {
    let (registry, _lp) = mock_daemon();
    let server = server_with(registry);
    assert_eq!(
        server.call("reboot", &[]),
        Err(UBUS_STATUS_METHOD_NOT_FOUND)
    );
}

/// Reply wire format sanity: the sendat reply body is a TABLE of named
/// fields with a named "lines" ARRAY of unnamed STRING items — the exact
/// nesting convention the C handler produces via the shared encoder.
#[test]
fn reply_wire_format_structure() {
    let (registry, _lp) = mock_daemon();
    let server = server_with(registry);

    let req = encode_sendat_request("usb0", "AT", 1000, None);
    let reply = server.call("sendat", &req).unwrap();
    let attrs = qmodem_atd::ubus_proto::decode_msg(&reply).unwrap();

    let mut saw_lines = false;
    for a in &attrs {
        assert!(a.name.is_some(), "top-level reply fields are named");
        if a.name.as_deref() == Some("lines") {
            saw_lines = true;
            assert_eq!(a.id, types::BLOBMSG_TYPE_ARRAY);
            for item in a.as_children().unwrap_or(&[]) {
                assert!(item.name.is_none(), "array items are unnamed");
                assert!(matches!(item.value, Value::String(_)));
            }
        }
    }
    assert!(saw_lines, "reply must contain a lines array");
}
