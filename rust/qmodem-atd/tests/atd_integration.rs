//! End-to-end tests for the thread-based AT daemon core and its JSON service.

use std::thread;
use std::time::Duration;

#[cfg(feature = "service")]
use std::io::{BufRead, BufReader, Write};
#[cfg(feature = "service")]
use std::os::unix::net::{UnixListener, UnixStream};
#[cfg(feature = "service")]
use std::sync::Arc;

#[cfg(feature = "service")]
use qmodem_atd::json;
#[cfg(feature = "service")]
use qmodem_atd::json::Value;
#[cfg(feature = "service")]
use qmodem_atd::line::LineHub;
use qmodem_atd::port::{spawn_port, AtError, AtPortConfig, ResponseStatus};
#[cfg(feature = "service")]
use qmodem_atd::registry::Registry;
use qmodem_atd::transport::{LoopbackPort, SerialPort};

/// Sequential sendat: each command gets "OK" and the echo is suppressed.
#[test]
fn sequential_sendat() {
    let lp = LoopbackPort::new();
    let h = lp.handle();
    h.set_responder(Box::new(|cmd| {
        vec![format!("{cmd} ECHO"), "OK".to_string()]
    }));
    let port = spawn_port("lb0", Box::new(lp), 0, AtPortConfig::default());

    for i in 0..5 {
        let r = port.send_at(&format!("AT+T{i}"), 1000, None).unwrap();
        assert_eq!(r.status, ResponseStatus::Ok);
        assert_eq!(r.end_flag, "OK");
        assert_eq!(r.lines, vec![format!("AT+T{i} ECHO"), "OK".to_string()]);
    }
}

/// Two threads send concurrently; the actor serializes them and each caller
/// receives its OWN response (the C single `current_response` slot would
/// let the second caller steal the first caller's answer).
#[test]
fn concurrent_sendat_no_crosstalk() {
    let lp = LoopbackPort::new();
    let h = lp.handle();
    h.set_responder(Box::new(|cmd| {
        vec![format!("RESP[{cmd}]"), "OK".to_string()]
    }));
    let port = spawn_port("lb1", Box::new(lp), 0, AtPortConfig::default());

    let p1 = port.clone();
    let p2 = port.clone();
    let t1 = thread::spawn(move || p1.send_at("AT+FIRST", 3000, None));
    let t2 = thread::spawn(move || p2.send_at("AT+SECOND", 3000, None));
    let r1 = t1.join().unwrap().unwrap();
    let r2 = t2.join().unwrap().unwrap();
    assert_eq!(r1.status, ResponseStatus::Ok);
    assert_eq!(r2.status, ResponseStatus::Ok);
    // Each response must contain its own command's marker.
    assert!(r1.lines.iter().any(|l| l.contains("RESP[AT+FIRST")));
    assert!(r2.lines.iter().any(|l| l.contains("RESP[AT+SECOND")));
    assert!(!r1.lines.iter().any(|l| l.contains("SECOND")));
    assert!(!r2.lines.iter().any(|l| l.contains("FIRST")));
}

/// Port never answers: caller gets Timeout plus the partial lines that
/// did arrive before the monotonic deadline (C used CLOCK_REALTIME and
/// could hang or jump on wall-clock steps).
#[test]
fn timeout_returns_partial() {
    let lp = LoopbackPort::new();
    let h = lp.handle();
    // Responder emits one info line then stays silent (no end flag).
    h.set_responder(Box::new(|_cmd| vec!["+CSQ: 21,99".to_string()]));
    let port = spawn_port("lb2", Box::new(lp), 0, AtPortConfig::default());

    let t0 = std::time::Instant::now();
    let r = port.send_at("AT+CSQ", 150, None).unwrap();
    let elapsed = t0.elapsed();
    assert_eq!(r.status, ResponseStatus::Timeout);
    assert_eq!(r.lines, vec!["+CSQ: 21,99".to_string()]);
    assert!(elapsed >= Duration::from_millis(140));
    assert!(elapsed < Duration::from_millis(2000));
}

/// Custom end flag (e.g. the SMS `>` prompt style) terminates early.
#[test]
fn custom_end_flag() {
    let lp = LoopbackPort::new();
    let h = lp.handle();
    h.set_responder(Box::new(|_cmd| vec!["> ".to_string()]));
    let port = spawn_port("lb3", Box::new(lp), 0, AtPortConfig::default());
    let r = port.send_at("AT+CMGS", 1000, Some(">")).unwrap();
    assert_eq!(r.status, ResponseStatus::Ok);
    assert_eq!(r.end_flag, ">");
}

/// URC lines are broadcast to subscribers with seq/epoch metadata.
#[test]
fn urc_broadcast() {
    let lp = LoopbackPort::new();
    let h = lp.handle();
    let port = spawn_port("lb4", Box::new(lp), 0, AtPortConfig::default());
    let rx = port.subscribe();

    h.push_urc("RING");
    let ev = rx.recv().unwrap();
    assert_eq!(ev.line, "RING");
    assert_eq!(ev.port, "lb4");
    assert_eq!(ev.epoch, 0);
    assert!(ev.seq >= 1);
}

/// Closing the port fails in-flight and subsequent requests with
/// PortClosed instead of panicking or hanging.
#[test]
fn port_closed_returns_error() {
    let lp = LoopbackPort::new();
    let h = lp.handle();
    h.set_response(vec!["OK".to_string()]);
    let port = spawn_port("lb5", Box::new(lp), 0, AtPortConfig::default());

    // Sanity: works while open.
    assert_eq!(port.send_at("AT", 1000, None).unwrap().status, ResponseStatus::Ok);

    h.close();
    // In-flight request (responder now silent) must fail or time out
    // gracefully; then new requests get PortClosed.
    thread::sleep(Duration::from_millis(50));
    match port.send_at("AT+ANY", 100, None) {
        Ok(r) => assert_eq!(r.status, ResponseStatus::Timeout),
        Err(AtError::PortClosed) => {}
    }
}

// ---------------------------------------------------------------------------
// JSON service integration (feature `service`)
// ---------------------------------------------------------------------------

#[cfg(feature = "service")]
fn roundtrip(
    reader: &mut BufReader<UnixStream>,
    writer: &mut UnixStream,
    req: &Value,
) -> Value {
    writeln!(writer, "{req}").unwrap();
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    json::parse(line.trim()).unwrap()
}

#[cfg(feature = "service")]
#[test]
fn json_service_sendat_and_list() {
    use qmodem_atd::service;

    let lp = LoopbackPort::new();
    let h = lp.handle();
    h.set_responder(Box::new(|cmd| {
        vec![format!("ACK[{cmd}]"), "OK".to_string()]
    }));

    let registry = Registry::new();
    registry.insert("loop0", spawn_port("loop0", Box::new(lp), 0, AtPortConfig::default()));
    let registry = Arc::new(registry);

    let dir = std::env::temp_dir().join(format!("qmodem-atd-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let sock_path = dir.join("atd.sock");
    let _ = std::fs::remove_file(&sock_path);
    let listener = UnixListener::bind(&sock_path).unwrap();
    let hub = Arc::new(LineHub::with_capacity(64));
    let hub_tx = hub.clone();
    thread::spawn(move || service::serve(listener, registry, hub_tx));

    let stream = UnixStream::connect(&sock_path).unwrap();
    let mut writer = stream.try_clone().unwrap();
    let mut reader = BufReader::new(stream);

    // sendat round trip.
    let resp = roundtrip(
        &mut reader,
        &mut writer,
        &json!({"cmd": "sendat", "port": "loop0", "at": "AT+CSQ", "timeout_ms": 1000}),
    );
    assert_eq!(resp["status"], "ok");
    assert_eq!(resp["end_flag"], "OK");
    assert_eq!(resp["lines"][0], "ACK[AT+CSQ]");
    assert!(resp["elapsed_ms"].as_u64().is_some());

    // list round trip.
    let resp = roundtrip(&mut reader, &mut writer, &json!({"cmd": "list"}));
    assert_eq!(resp["status"], "ok");
    assert_eq!(resp["ports"], json!(["loop0"]));

    // unknown port.
    let resp = roundtrip(
        &mut reader,
        &mut writer,
        &json!({"cmd": "sendat", "port": "nope", "at": "AT"}),
    );
    assert_eq!(resp["status"], "error");

    // bad json -> error response, connection stays usable.
    writeln!(writer, "{{not json").unwrap();
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let resp = json::parse(line.trim()).unwrap();
    assert_eq!(resp["status"], "error");
    assert!(resp["error"].as_str().unwrap().contains("bad json"));

    // subscribe on a second connection: forward the next broadcast line.
    let sub_stream = UnixStream::connect(&sock_path).unwrap();
    let mut sub_writer = sub_stream.try_clone().unwrap();
    let mut sub_reader = BufReader::new(sub_stream);
    sub_writer.write_all(b"{\"cmd\":\"subscribe\"}\n").unwrap();
    thread::sleep(Duration::from_millis(50));
    hub.send(qmodem_atd::line::AtLine {
        port: "loop0".into(),
        line: "+URC: 1".into(),
        seq: 1,
        epoch: 0,
    });
    let mut line = String::new();
    sub_reader.read_line(&mut line).unwrap();
    let ev = json::parse(line.trim()).unwrap();
    assert_eq!(ev["event"], "line");
    assert_eq!(ev["line"], "+URC: 1");
    assert_eq!(ev["seq"], 1);
    let _ = std::fs::remove_file(&sock_path);
}

/// Feature-gated transports must still compile into the default build
/// path: verify the SerialPort impl object is spawnable via the same API.
#[test]
fn boxed_transport_dispatch() {
    let lp = LoopbackPort::new();
    let h = lp.handle();
    h.set_response(vec!["OK".to_string()]);
    let boxed: Box<dyn SerialPort> = Box::new(lp);
    let port = spawn_port("boxed", boxed, 7, AtPortConfig::default());
    let r = port.send_at("ATI", 1000, None).unwrap();
    assert_eq!(r.status, ResponseStatus::Ok);
    // The per-port epoch flows into broadcast events.
    let rx = port.subscribe();
    h.push_urc("EVT");
    assert_eq!(rx.recv().unwrap().epoch, 7);
}
