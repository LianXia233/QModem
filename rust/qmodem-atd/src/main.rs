//! qmodem-atd entry point.
//!
//! Usage:
//! ```text
//! qmodem-atd --socket /var/run/qmodem-atd.sock [--loopback NAME]...
//!            [--port /dev/ttyUSB2]...      # requires feature `tty`
//! ```
//!
//! Each `--loopback NAME` spawns an in-process loopback modem (handy for
//! smoke tests); each `--port PATH` opens a real tty (feature `tty`).

use std::sync::Arc;

use qmodem_atd::port::{spawn_port, AtPortConfig};
use qmodem_atd::registry::Registry;
use qmodem_atd::transport::LoopbackPort;
#[cfg(feature = "tty")]
use qmodem_atd::transport::SerialPort;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut socket: Option<String> = None;
    let mut loopbacks: Vec<String> = Vec::new();
    let mut ports: Vec<String> = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--socket" => {
                i += 1;
                socket = args.get(i).cloned();
            }
            "--loopback" => {
                i += 1;
                if let Some(n) = args.get(i) {
                    loopbacks.push(n.clone());
                }
            }
            "--port" => {
                i += 1;
                if let Some(p) = args.get(i) {
                    ports.push(p.clone());
                }
            }
            other => {
                eprintln!("unknown arg: {other}");
                std::process::exit(2);
            }
        }
        i += 1;
    }

    let socket = socket.unwrap_or_else(|| {
        eprintln!("--socket is required");
        std::process::exit(2);
    });

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main(socket, loopbacks, ports));
}

async fn async_main(socket: String, loopbacks: Vec<String>, ports: Vec<String>) {
    let registry = Arc::new(Registry::new());
    let (hub, _) = tokio::sync::broadcast::channel::<qmodem_atd::AtLine>(256);

    for name in &loopbacks {
        let port = LoopbackPort::new();
        let handle = spawn_port(name.clone(), Box::new(port), 0, AtPortConfig::default());
        registry.insert(name.clone(), handle);
        println!("registered loopback port {name}");
    }

    #[cfg(feature = "tty")]
    for path in &ports {
        match qmodem_atd::transport::tty::TtyPort::open(path) {
            Ok(tty) => {
                let boxed: Box<dyn SerialPort> = Box::new(tty);
                let handle = spawn_port(path.clone(), boxed, 0, AtPortConfig::default());
                registry.insert(path.clone(), handle);
                println!("registered tty port {path}");
            }
            Err(e) => {
                eprintln!("failed to open {path}: {e}");
                std::process::exit(1);
            }
        }
    }
    #[cfg(not(feature = "tty"))]
    {
        if !ports.is_empty() {
            eprintln!("--port requires building with --features tty");
            std::process::exit(2);
        }
        let _ = &ports;
    }

    #[cfg(feature = "service")]
    {
        // Bind the unix socket, removing any stale file first.
        let _ = std::fs::remove_file(&socket);
        let listener = tokio::net::UnixListener::bind(&socket).expect("bind unix socket");
        println!("qmodem-atd listening on {socket}");
        qmodem_atd::service::serve(listener, registry, hub).await;
    }
    #[cfg(not(feature = "service"))]
    {
        let _ = (socket, registry, hub);
        println!("built without the service feature; nothing to serve");
    }
}
