//! End-to-end tests for the async AT daemon core and its JSON service.

use std::time::Duration;

use qmodem_atd::port::{spawn_port, AtError, AtPortConfig, ResponseStatus};
use qmodem_atd::registry::Registry;
use qmodem_atd::transport::{LoopbackPort, SerialPort};

/// Sequential sendat: each command gets "OK" and the echo is suppressed.
#[tokio::test(flavor = "multi_thread")]
async fn sequential_sendat() {
    let lp = LoopbackPort::new();
    let h = lp.handle();
    h.set_responder(Box::new(|cmd| {
        vec![format!("{cmd} ECHO"), "OK".to_string()]
    }));
    let port = spawn_port("lb0", Box::new(lp), 0, AtPortConfig::default());

    for i in 0..5 {
        let r = port.send_at(&format!("AT+T{i}"), 1000, None).await.unwrap();
        assert_eq!(r.status, ResponseStatus::Ok);
        assert_eq!(r.end_flag, "OK");
        assert_eq!(r.lines, vec![format!("AT+T{i} ECHO"), "OK".to_string()]);
    }
}

/// Two tasks send concurrently; the actor serializes them and each caller
/// receives its OWN response (the C single `current_response` slot would
/// let the second caller steal the first caller's answer).
#[tokio::test(flavor = "multi_thread")]
async fn concurrent_sendat_no_crosstalk() {
    let lp = LoopbackPort::new();
    let h = lp.handle();
    h.set_responder(Box::new(|cmd| {
        vec![format!("RESP[{cmd}]"), "OK".to_string()]
    }));
    let port = spawn_port("lb1", Box::new(lp), 0, AtPortConfig::default());

    let p1 = port.clone();
    let p2 = port.clone();
    let (r1, r2) = tokio::join!(
        p1.send_at("AT+FIRST", 3000, None),
        p2.send_at("AT+SECOND", 3000, None),
    );
    let r1 = r1.unwrap();
    let r2 = r2.unwrap();
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
#[tokio::test(flavor = "multi_thread")]
async fn timeout_returns_partial() {
    let lp = LoopbackPort::new();
    let h = lp.handle();
    // Responder emits one info line then stays silent (no end flag).
    h.set_responder(Box::new(|_cmd| vec!["+CSQ: 21,99".to_string()]));
    let port = spawn_port("lb2", Box::new(lp), 0, AtPortConfig::default());

    let t0 = std::time::Instant::now();
    let r = port.send_at("AT+CSQ", 150, None).await.unwrap();
    let elapsed = t0.elapsed();
    assert_eq!(r.status, ResponseStatus::Timeout);
    assert_eq!(r.lines, vec!["+CSQ: 21,99".to_string()]);
    assert!(elapsed >= Duration::from_millis(140));
    assert!(elapsed < Duration::from_millis(2000));
}

/// Custom end flag (e.g. the SMS `>` prompt style) terminates early.
#[tokio::test(flavor = "multi_thread")]
async fn custom_end_flag() {
    let lp = LoopbackPort::new();
    let h = lp.handle();
    h.set_responder(Box::new(|_cmd| vec!["> ".to_string()]));
    let port = spawn_port("lb3", Box::new(lp), 0, AtPortConfig::default());
    let r = port.send_at("AT+CMGS", 1000, Some(">")).await.unwrap();
    assert_eq!(r.status, ResponseStatus::Ok);
    assert_eq!(r.end_flag, ">");
}

/// URC lines are broadcast to subscribers with seq/epoch metadata.
#[tokio::test(flavor = "multi_thread")]
async fn urc_broadcast() {
    let lp = LoopbackPort::new();
    let h = lp.handle();
    let port = spawn_port("lb4", Box::new(lp), 0, AtPortConfig::default());
    let mut rx = port.subscribe();

    h.push_urc("RING");
    let ev = rx.recv().await.unwrap();
    assert_eq!(ev.line, "RING");
    assert_eq!(ev.port, "lb4");
    assert_eq!(ev.epoch, 0);
    assert!(ev.seq >= 1);
}

/// Closing the port fails in-flight and subsequent requests with
/// PortClosed instead of panicking or hanging.
#[tokio::test(flavor = "multi_thread")]
async fn port_closed_returns_error() {
    let lp = LoopbackPort::new();
    let h = lp.handle();
    h.set_response(vec!["OK".to_string()]);
    let port = spawn_port("lb5", Box::new(lp), 0, AtPortConfig::default());

    // Sanity: works while open.
    assert_eq!(port.send_at("AT", 1000, None).await.unwrap().status, ResponseStatus::Ok);

    h.close();
    // In-flight request (responder now silent) must fail or time out
    // gracefully; then new requests get PortClosed.
    tokio::time::sleep(Duration::from_millis(50)).await;
    match port.send_at("AT+ANY", 100, None).await {
        Ok(r) => assert_eq!(r.status, ResponseStatus::Timeout),
        Err(AtError::PortClosed) => {}
    }
}

// ---------------------------------------------------------------------------
// JSON service integration
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn json_service_sendat_and_list() {
    use qmodem_atd::service;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::{UnixListener, UnixStream};

    let lp = LoopbackPort::new();
    let h = lp.handle();
    h.set_responder(Box::new(|cmd| {
        vec![format!("ACK[{cmd}]"), "OK".to_string()]
    }));

    let registry = Registry::new();
    registry.insert("loop0", spawn_port("loop0", Box::new(lp), 0, AtPortConfig::default()));
    let registry = std::sync::Arc::new(registry);

    let dir = std::env::temp_dir().join(format!("qmodem-atd-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let sock_path = dir.join("atd.sock");
    let _ = std::fs::remove_file(&sock_path);
    let listener = UnixListener::bind(&sock_path).unwrap();
    let (hub, _) = tokio::sync::broadcast::channel(64);
    let hub_tx = hub.clone();
    tokio::spawn(async move { service::serve(listener, registry, hub_tx).await });

    let stream = UnixStream::connect(&sock_path).await.unwrap();
    let (rd, mut wr) = stream.into_split();
    let mut reader = BufReader::new(rd);

    async fn roundtrip(
        reader: &mut BufReader<tokio::net::unix::OwnedReadHalf>,
        wr: &mut tokio::net::unix::OwnedWriteHalf,
        req: serde_json::Value,
    ) -> serde_json::Value {
        wr.write_all(format!("{req}\n").as_bytes()).await.unwrap();
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        serde_json::from_str(&line).unwrap()
    }

    // sendat round trip.
    let resp = roundtrip(
        &mut reader,
        &mut wr,
        serde_json::json!({"cmd": "sendat", "port": "loop0", "at": "AT+CSQ", "timeout_ms": 1000}),
    )
    .await;
    assert_eq!(resp["status"], "ok");
    assert_eq!(resp["end_flag"], "OK");
    assert_eq!(resp["lines"][0], "ACK[AT+CSQ]");

    // list round trip.
    let resp = roundtrip(&mut reader, &mut wr, serde_json::json!({"cmd": "list"})).await;
    assert_eq!(resp["status"], "ok");
    assert_eq!(resp["ports"], serde_json::json!(["loop0"]));

    // unknown port.
    let resp = roundtrip(
        &mut reader,
        &mut wr,
        serde_json::json!({"cmd": "sendat", "port": "nope", "at": "AT"}),
    )
    .await;
    assert_eq!(resp["status"], "error");

    // subscribe on a second connection: forward the next broadcast line.
    let sub_stream = UnixStream::connect(&sock_path).await.unwrap();
    let (sub_rd, mut sub_wr) = sub_stream.into_split();
    let mut sub_reader = BufReader::new(sub_rd);
    sub_wr
        .write_all(b"{\"cmd\":\"subscribe\"}\n")
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    hub.send(qmodem_atd::line::AtLine {
        port: "loop0".into(),
        line: "+URC: 1".into(),
        seq: 1,
        epoch: 0,
    })
    .unwrap();
    let mut line = String::new();
    sub_reader.read_line(&mut line).await.unwrap();
    let ev: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(ev["event"], "line");
    assert_eq!(ev["line"], "+URC: 1");
    assert_eq!(ev["seq"], 1);
    let _ = std::fs::remove_file(&sock_path);
}

/// Feature-gated transports must still compile into the default build
/// path: verify the SerialPort impl object is spawnable via the same API.
#[tokio::test(flavor = "multi_thread")]
async fn boxed_transport_dispatch() {
    let lp = LoopbackPort::new();
    let h = lp.handle();
    h.set_response(vec!["OK".to_string()]);
    let boxed: Box<dyn SerialPort> = Box::new(lp);
    let port = spawn_port("boxed", boxed, 7, AtPortConfig::default());
    let r = port.send_at("ATI", 1000, None).await.unwrap();
    assert_eq!(r.status, ResponseStatus::Ok);
}
