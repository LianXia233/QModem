//! Unix-socket JSON line protocol service (feature `service`, default on).
//!
//! This is the pre-ubus test/management interface (ubus FFI is gated
//! behind `feature = "ubus"`; see `ubus_ffi.rs`). One JSON object per line:
//!
//! * `{"cmd":"sendat","port":"/dev/ttyUSB2","at":"AT+CSQ","timeout_ms":3000,
//!    "end_flag":"OK"}` -> `{"status":"ok"|"timeout","lines":[...],
//!    "end_flag":"OK","elapsed_ms":12}` or `{"status":"error",...}`;
//! * `{"cmd":"list"}` -> `{"status":"ok","ports":[...]}`;
//! * `{"cmd":"subscribe"}` -> stream of `{"event":"line","port":...,
//!    "line":...,"seq":...,"epoch":...}` objects until disconnect.
//!
//! Note: `subscribe` never answers a request line; it only streams events.

use std::sync::Arc;

use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::broadcast;

use crate::line::AtLine;
use crate::port::{AtError, ResponseStatus};
use crate::registry::Registry;

/// Accept loop; runs until the listener errors.
pub async fn serve(listener: UnixListener, registry: Arc<Registry>, hub: broadcast::Sender<AtLine>) {
    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let registry = registry.clone();
                let hub = hub.clone();
                tokio::spawn(async move {
                    handle_conn(stream, registry, hub).await;
                });
            }
            Err(_) => return,
        }
    }
}

async fn handle_conn(
    stream: UnixStream,
    registry: Arc<Registry>,
    hub: broadcast::Sender<AtLine>,
) {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let req: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let resp = json!({"status": "error", "error": format!("bad json: {e}")});
                if write_half
                    .write_all(format!("{resp}\n").as_bytes())
                    .await
                    .is_err()
                {
                    return;
                }
                continue;
            }
        };
        match req.get("cmd").and_then(|v| v.as_str()) {
            Some("list") => {
                let resp = json!({"status": "ok", "ports": registry.list()});
                if write_half
                    .write_all(format!("{resp}\n").as_bytes())
                    .await
                    .is_err()
                {
                    return;
                }
            }
            Some("sendat") => {
                let resp = dispatch_sendat(&registry, &req).await;
                if write_half
                    .write_all(format!("{resp}\n").as_bytes())
                    .await
                    .is_err()
                {
                    return;
                }
            }
            Some("subscribe") => {
                let mut rx = hub.subscribe();
                loop {
                    match rx.recv().await {
                        Ok(l) => {
                            let ev = json!({
                                "event": "line",
                                "port": l.port,
                                "line": l.line,
                                "seq": l.seq,
                                "epoch": l.epoch,
                            });
                            if write_half
                                .write_all(format!("{ev}\n").as_bytes())
                                .await
                                .is_err()
                            {
                                return;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            let ev = json!({"event": "lagged", "dropped": n});
                            let _ = write_half
                                .write_all(format!("{ev}\n").as_bytes())
                                .await;
                        }
                        Err(broadcast::error::RecvError::Closed) => return,
                    }
                }
            }
            _ => {
                let resp = json!({"status": "error", "error": "unknown cmd"});
                if write_half
                    .write_all(format!("{resp}\n").as_bytes())
                    .await
                    .is_err()
                {
                    return;
                }
            }
        }
    }
}

async fn dispatch_sendat(registry: &Registry, req: &serde_json::Value) -> serde_json::Value {
    let port = match req.get("port").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => return json!({"status": "error", "error": "missing port"}),
    };
    let at = match req.get("at").and_then(|v| v.as_str()) {
        Some(a) => a.to_string(),
        None => return json!({"status": "error", "error": "missing at"}),
    };
    let timeout_ms = req
        .get("timeout_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(3000);
    let end_flag = req.get("end_flag").and_then(|v| v.as_str()).map(str::to_string);

    let handle = match registry.get(&port) {
        Some(h) => h,
        None => return json!({"status": "error", "error": "unknown port"}),
    };
    match handle.send_at(&at, timeout_ms, end_flag.as_deref()).await {
        Ok(resp) => json!({
            "status": match resp.status {
                ResponseStatus::Ok => "ok",
                ResponseStatus::Timeout => "timeout",
            },
            "lines": resp.lines,
            "end_flag": resp.end_flag,
            "elapsed_ms": resp.elapsed_ms,
        }),
        Err(AtError::PortClosed) => json!({"status": "error", "error": "port_closed"}),
    }
}
