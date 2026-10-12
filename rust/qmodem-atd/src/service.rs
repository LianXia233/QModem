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
//!
//! Implementation: `std::os::unix::net::UnixListener` with one thread per
//! connection; JSON via the built-in [`crate::json`] module.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Arc;
use std::thread;

use crate::json::{self, Value};
use crate::line::{LineHub, RecvError};
use crate::port::{AtError, ResponseStatus};
use crate::registry::Registry;

/// Accept loop; runs until the listener errors. Blocks the calling thread.
pub fn serve(listener: UnixListener, registry: Arc<Registry>, hub: Arc<LineHub>) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let registry = registry.clone();
                let hub = hub.clone();
                let _ = thread::Builder::new()
                    .name("atd-conn".to_string())
                    .spawn(move || handle_conn(stream, registry, hub));
            }
            Err(_) => return,
        }
    }
}

fn write_line(w: &mut UnixStream, v: &Value) -> bool {
    writeln!(w, "{v}").is_ok() && w.flush().is_ok()
}

fn handle_conn(stream: UnixStream, registry: Arc<Registry>, hub: Arc<LineHub>) {
    let Ok(mut writer) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let req: Value = match json::parse(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let resp = crate::json!({"status": "error", "error": format!("bad json: {e}")});
                if !write_line(&mut writer, &resp) {
                    return;
                }
                continue;
            }
        };
        match req.get("cmd").and_then(Value::as_str) {
            Some("list") => {
                let resp = crate::json!({"status": "ok", "ports": registry.list()});
                if !write_line(&mut writer, &resp) {
                    return;
                }
            }
            Some("sendat") => {
                let resp = dispatch_sendat(&registry, &req);
                if !write_line(&mut writer, &resp) {
                    return;
                }
            }
            Some("subscribe") => {
                let rx = hub.subscribe();
                loop {
                    match rx.recv() {
                        Ok(l) => {
                            let ev = crate::json!({
                                "event": "line",
                                "port": l.port,
                                "line": l.line,
                                "seq": l.seq,
                                "epoch": l.epoch,
                            });
                            if !write_line(&mut writer, &ev) {
                                return;
                            }
                        }
                        Err(RecvError::Lagged(n)) => {
                            let ev = crate::json!({"event": "lagged", "dropped": n});
                            if !write_line(&mut writer, &ev) {
                                return;
                            }
                        }
                        Err(RecvError::Closed) => return,
                    }
                }
            }
            _ => {
                let resp = crate::json!({"status": "error", "error": "unknown cmd"});
                if !write_line(&mut writer, &resp) {
                    return;
                }
            }
        }
    }
}

fn dispatch_sendat(registry: &Registry, req: &Value) -> Value {
    let port = match req.get("port").and_then(Value::as_str) {
        Some(p) => p.to_string(),
        None => return crate::json!({"status": "error", "error": "missing port"}),
    };
    let at = match req.get("at").and_then(Value::as_str) {
        Some(a) => a.to_string(),
        None => return crate::json!({"status": "error", "error": "missing at"}),
    };
    let timeout_ms = req
        .get("timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(3000);
    let end_flag = req.get("end_flag").and_then(Value::as_str).map(str::to_string);

    let handle = match registry.get(&port) {
        Some(h) => h,
        None => return crate::json!({"status": "error", "error": "unknown port"}),
    };
    match handle.send_at(&at, timeout_ms, end_flag.as_deref()) {
        Ok(resp) => crate::json!({
            "status": match resp.status {
                ResponseStatus::Ok => "ok",
                ResponseStatus::Timeout => "timeout",
            },
            "lines": resp.lines,
            "end_flag": resp.end_flag,
            "elapsed_ms": resp.elapsed_ms,
        }),
        Err(AtError::PortClosed) => crate::json!({"status": "error", "error": "port_closed"}),
    }
}
