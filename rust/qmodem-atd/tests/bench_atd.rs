//! std-only micro-benchmarks for qmodem-atd, in integration-test form.
//!
//! No criterion and no external crates: each `#[test]` times a fixed
//! workload with `std::time::Instant`, prints p50/p95/max (plus the mean)
//! and asserts only a generous sanity upper bound so CI stays green on
//! slow/loaded machines (no flaky timing assertions).
//!
//! Run: `cargo test -p qmodem-atd --test bench_atd -- --nocapture`

use std::hint::black_box;
use std::thread;
use std::time::{Duration, Instant};

use qmodem_atd::json;
use qmodem_atd::line::{AtLine, LineHub};
use qmodem_atd::port::{spawn_port, AtPortConfig, ResponseStatus};
use qmodem_atd::transport::LoopbackPort;

/// `p`-quantile of the samples (0.5 -> p50, 0.95 -> p95, 1.0 -> max).
fn pct(mut samples: Vec<f64>, p: f64) -> f64 {
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let idx = (((samples.len() as f64) * p).ceil() as usize)
        .saturating_sub(1)
        .min(samples.len() - 1);
    samples[idx]
}

fn mean(samples: &[f64]) -> f64 {
    samples.iter().sum::<f64>() / samples.len() as f64
}

/// 1) Single-port sendat round-trip latency over LoopbackPort.
///
/// The actor picks queued commands on its idle read slice, so the
/// distribution is dominated by the 50 ms idle slice; that is the real
/// standby pickup latency this bench documents.
#[test]
fn bench_sendat_roundtrip_latency() {
    let lp = LoopbackPort::new();
    lp.handle()
        .set_responder(Box::new(|_cmd| vec!["OK".to_string()]));
    let port = spawn_port("bench-lat", Box::new(lp), 0, AtPortConfig::default());

    // Warm-up: thread spawn, first-response path, allocator pools.
    for _ in 0..20 {
        port.send_at("AT", 2000, None).unwrap();
    }

    const N: usize = 1000;
    let mut ms = Vec::with_capacity(N);
    for _ in 0..N {
        let t0 = Instant::now();
        let r = port.send_at("AT+CSQ", 2000, None).unwrap();
        ms.push(t0.elapsed().as_secs_f64() * 1e3);
        assert_eq!(r.status, ResponseStatus::Ok);
        assert_eq!(r.end_flag, "OK");
    }

    let (p50, p95, max, avg) = (
        pct(ms.clone(), 0.5),
        pct(ms.clone(), 0.95),
        pct(ms.clone(), 1.0),
        mean(&ms),
    );
    println!("sendat round-trip latency ({} samples, ms):", N);
    println!("  p50={:.3}  p95={:.3}  max={:.3}  mean={:.3}", p50, p95, max, avg);
    println!("  total={:.1}ms", ms.iter().sum::<f64>());

    // Sanity bound only (see module docs): idle-slice pickup is bounded
    // by 50 ms + scheduling jitter; 100 ms leaves generous headroom.
    assert!(max < 100.0, "round trip {}ms exceeds 100ms sanity bound", max);
}

/// 2) Concurrent queueing throughput: 8 tasks x 500 commands through one
/// port actor (strictly serial execution on the bounded mpsc queue).
#[test]
fn bench_sendat_concurrent_throughput() {
    let lp = LoopbackPort::new();
    lp.handle().set_responder(Box::new(|cmd| {
        vec![format!("ECHO {cmd}"), "OK".to_string()]
    }));
    let port = spawn_port("bench-tput", Box::new(lp), 0, AtPortConfig::default());

    const TASKS: usize = 8;
    const PER_TASK: usize = 500;
    const TOTAL: usize = TASKS * PER_TASK;

    let t0 = Instant::now();
    let workers: Vec<_> = (0..TASKS)
        .map(|_| {
            let p = port.clone();
            thread::spawn(move || {
                for _ in 0..PER_TASK {
                    let r = p.send_at("AT+T", 5000, None).unwrap();
                    assert_eq!(r.status, ResponseStatus::Ok);
                }
            })
        })
        .collect();
    for w in workers {
        w.join().unwrap();
    }
    let total = t0.elapsed();
    let total_ms = total.as_secs_f64() * 1e3;
    println!("concurrent sendat throughput ({} tasks x {}):", TASKS, PER_TASK);
    println!("  total={:.1}ms  throughput={:.0} cmd/s", total_ms, TOTAL as f64 / total.as_secs_f64());

    // Sanity bound only: an always-non-empty queue means the actor never
    // idles between commands; 30 s leaves huge headroom for slow CI.
    assert!(total < Duration::from_secs(30), "throughput bench over sanity bound");
}

/// 3) Broadcast fan-out latency: one hub, 32 subscribers, per-send
/// dispatch time measured over 1000 published lines.
#[test]
fn bench_broadcast_32_subscribers() {
    let hub = LineHub::with_capacity(4096); // no lag during the run
    let rxs: Vec<_> = (0..32).map(|_| hub.subscribe()).collect();

    const N: usize = 1000;
    let mut us = Vec::with_capacity(N);
    for i in 0..N {
        let line = AtLine {
            port: "bench".into(),
            line: format!("+URC: {i}"),
            seq: i as u64,
            epoch: 0,
        };
        let t0 = Instant::now();
        let delivered = hub.send(line);
        us.push(t0.elapsed().as_secs_f64() * 1e6);
        assert_eq!(delivered, 32);
    }

    let (p50, p95, max, avg) = (
        pct(us.clone(), 0.5),
        pct(us.clone(), 0.95),
        pct(us.clone(), 1.0),
        mean(&us),
    );
    println!("broadcast fan-out to 32 subscribers ({} sends, us):", N);
    println!("  p50={:.2}  p95={:.2}  max={:.2}  mean={:.2}", p50, p95, max, avg);
    println!("  total={:.1}ms", us.iter().sum::<f64>() / 1e3);

    // Delivery correctness: every subscriber drained all N lines
    // (Duration::ZERO makes an empty queue report Ok(None) immediately).
    for (si, rx) in rxs.iter().enumerate() {
        let mut got = 0;
        while let Ok(Some(_)) = rx.recv_timeout(Duration::ZERO) {
            got += 1;
        }
        assert_eq!(got, N, "subscriber {si} missing lines");
    }

    // Sanity bound only: fan-out is pure in-process mutex work.
    let total = Duration::from_micros(us.iter().sum::<f64>() as u64);
    assert!(total < Duration::from_secs(5), "broadcast bench over sanity bound");
}

/// 4) JSON parse + serialize on a ~1 KB protocol payload, 1000 ops each.
#[test]
fn bench_json_parse_serialize_1kb() {
    // Deterministic ~1024-byte object shaped like the daemon protocol.
    let base = String::from(
        r##"{"cmd":"sendat","port":"/dev/ttyUSB2","at":"AT+CSQ","timeout_ms":3000,"lines":["+CSQ: 24,99","OK"],"meta":{"epoch":3,"seq":123456789,"note":""##
    );
    let pad = 1024usize.saturating_sub(base.len() + 3);
    let mut payload = base;
    payload.push_str(&"x".repeat(pad));
    payload.push_str(r#""}}"#);
    assert_eq!(payload.len(), 1024, "payload must be ~1KB");

    let value = json::parse(&payload).unwrap();
    // Round-trip correctness gate (cheap, deterministic).
    assert_eq!(json::parse(&value.to_string()).unwrap(), value);

    const N: usize = 1000;
    // Warm-up.
    for _ in 0..50 {
        black_box(json::parse(&payload).unwrap());
        black_box(value.to_string());
    }

    let mut parse_us = Vec::with_capacity(N);
    for _ in 0..N {
        let t0 = Instant::now();
        black_box(json::parse(&payload).unwrap());
        parse_us.push(t0.elapsed().as_secs_f64() * 1e6);
    }

    let mut ser_us = Vec::with_capacity(N);
    for _ in 0..N {
        let t0 = Instant::now();
        black_box(value.to_string());
        ser_us.push(t0.elapsed().as_secs_f64() * 1e6);
    }

    for (label, s) in [("parse", &parse_us), ("serialize", &ser_us)] {
        println!(
            "json {} 1KB ({} ops, us):  p50={:.2}  p95={:.2}  max={:.2}  mean={:.2}  total={:.1}ms",
            label,
            N,
            pct(s.clone(), 0.5),
            pct(s.clone(), 0.95),
            pct(s.clone(), 1.0),
            mean(s),
            s.iter().sum::<f64>() / 1e3,
        );
    }

    // Sanity bound only: parsing 1MB/s would already pass.
    let total: f64 = parse_us.iter().chain(ser_us.iter()).sum();
    assert!(total < 5e6, "json bench over sanity bound: {:.1}ms", total / 1e3);
}
