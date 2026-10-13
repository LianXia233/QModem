# qmodem-atd Async AT Daemon Architecture (Async Core · Experimental)

[简体中文](async-daemon.zh-cn.md) | English

> Status: **experimental**. `rust/qmodem-atd` is a parallel alternative to `application/qmodem/src/ubus_at_daemon` (the C version) and currently **does not replace the packaging path of any existing component**. The authoritative sources for design notes and fix cross-references are `rust/README.md` and `docs/refactor/journal-rust-core.md`; this document is a developer-oriented architecture tour.

## 1. Thread-actor model

`qmodem-atd` is built on the tokio multi-threaded runtime; the core structure is "**one actor per port**":

```
                Unix socket JSON service (feature "service", enabled by default)
                     │  sendat / list / subscribe
                     ▼
              Registry (port registry, shared via Arc)
                     │  look up port handle
        ┌────────────┼────────────┐
        ▼            ▼            ▼
   ┌─────────┐  ┌─────────┐  ┌─────────┐
   │ AtPort  │  │ AtPort  │  │ AtPort  │   ← one actor task per port
   │ (ttyUSB0)  │ (ttyUSB1)  │ (ttyUSB2)  │      exclusive Box<dyn SerialPort>
   │ mpsc command queue (serial execution)          │
   │ per-request private oneshot reply            │
   │ tokio::time::sleep monotonic timeout        │
   └────┬────┘  └────┬────┘  └────┬────┘
        └────────────┼────────────┘
                     ▼
          broadcast::channel (URC broadcast, capacity 256)
          AtLine { port, line, seq, epoch }
```

Key points:

| Mechanism | Implementation | Location (rust/qmodem-atd/src/) |
|---|---|---|
| Port ownership | `Box<dyn SerialPort>` is owned exclusively by the actor; zero sharing on the read/write paths | `transport.rs` / `port.rs` |
| Command serialization | per-port mpsc queue; concurrent requests each hold their own oneshot, responses never cross | `port.rs` |
| Timeout | `tokio::select!` + `tokio::time::sleep` (**monotonic clock**); on timeout, returns the partially received lines + `Timeout` | `port.rs` |
| URC broadcast | `tokio::sync::broadcast`, `AtLine{port,line,seq,epoch}` preserves the C version's seq/restart-epoch semantics; lagging subscribers get the dropped-line count reported via `RecvError::Lagged` | `service.rs` |
| Port shutdown | On actor exit it drains: all pending and queued requests receive `AtError::PortClosed`, no panic, no hang | `port.rs` |
| End-flag matching | `Pending::matches_end_flag` ports the C version's exact/prefix/colon special-case rules; the default 5 end flags are identical | `port.rs` |
| Echo suppression | **Intentionally different from C**: exact match against the most recently written command text (the C version drops any line with an `AT` prefix, which also kills legitimate response lines starting with `AT+...`, see section 3 of journal-rust-core.md) | `port.rs` |

The default build (the `service` feature) does not touch serial ports or libubus and compiles and tests fully green on dev machines/CI; `--loopback NAME` starts an in-process loopback modem for smoke tests, and `--port PATH` requires `--features tty`.

## 2. Comparison with the C version ubus_at_daemon

C sources: `application/qmodem/src/ubus_at_daemon/src/`. The per-item "problem → how Rust eliminates it" material is taken from section 3 of `docs/refactor/journal-rust-core.md`:

| # | C problem | C location | How the Rust design eliminates it |
|---|---|---|---|
| A1 | Per-port pthread reader; the write path uses the same `port->fd` concurrently with the reader | at_handler.c:193-195, 307-320 | The `AtPort` actor's `tokio::select!` loop owns the `SerialPort` exclusively; the ownership model eliminates the UAF root cause directly |
| A2 | Single-slot `current_response` + `waiting_for_response`; concurrent sendat calls trample each other and responses cross | at_handler.c:138-158, 322-334 | per-port mpsc command queue executes serially with a private oneshot per request; concurrency tests assert that the two requests each get their own response with zero content mismatch |
| A3 | `CLOCK_REALTIME` absolute timeouts; wall-clock jumps cause early/late timeouts | at_handler.c:208-215 | `tokio::time::sleep` monotonic clock; on timeout, returns the partially received lines + `Timeout` (tests assert the elapsed time hugs the deadline) |
| A4 | Fixed ring queue for line_events + drop_count + notify_pipe | line_events.c, ubus_at_daemon.h:78-89 | `broadcast::channel` preserves the seq/restart_epoch semantics; lagging is reported explicitly via `RecvError::Lagged` |
| A5 | After a port closes, waiters hang on a condvar that no one signals | reader_thread_func exit path | The actor drains on exit: all waiting requests get `AtError::PortClosed` (test-covered) |
| A6 | End-flag matching rules scattered and hand-written | at_handler.c:79-121, 28-77 | Centralized in `Pending::matches_end_flag`; the default end flags match C |
| A7 | Hard dependency on ubus; host builds need the QMODEM_HOST_TEST shim | ubus_at_daemon.h:15-23 | `feature = "service"` (default) uses a Unix socket JSON line protocol as the test interface; `feature = "ubus"` reserves an FFI skeleton, the default build is unaffected |

For memory-safety fixes on the C side (uninitialized response, fd leaks, refcount races, etc.), see `docs/refactor/journal-c-core.md`.

## 3. Unix socket JSON test protocol

Startup (requires `cargo build --release` first, see `rust/README.md`):

```sh
cd rust
cargo build --release
./target/release/qmodem-atd --socket /var/run/qmodem/atd.sock --loopback loop0
```

The protocol is **one JSON object per line**, with three commands: `sendat`, `list`, and `subscribe` (`subscribe` only streams events continuously and does not reply to the request line).

### sendat

```sh
curl --unix-socket /var/run/qmodem/atd.sock \
     -d '{"cmd":"sendat","port":"/dev/ttyUSB2","at":"AT+CSQ","timeout_ms":3000,"end_flag":"OK"}' \
     http://localhost/
```

Response:

```json
{"status":"ok","lines":["+CSQ: 25,99","OK"],"end_flag":"OK","elapsed_ms":12}
```

`status` is `ok` / `timeout` / `error`; on `timeout`, `lines` carries the partially received lines; a nonexistent port returns `{"status":"error","error":"unknown port"}`.

### list

```sh
curl --unix-socket /var/run/qmodem/atd.sock \
     -d '{"cmd":"list"}' http://localhost/
# {"status":"ok","ports":["/dev/ttyUSB2","loop0"]}
```

### subscribe (URC stream)

```sh
curl -N --unix-socket /var/run/qmodem/atd.sock \
     -d '{"cmd":"subscribe"}' http://localhost/
# {"event":"line","port":"/dev/ttyUSB2","line":"+CMTI: \"SM\",3","seq":7,"epoch":1}
# If the consumer lags behind: {"event":"lagged","dropped":n}
```

## 4. Cross-compilation (aarch64 / x86_64 musl)

The default feature set (`service`) depends only on tokio / serde / serde_json / thiserror (plus libc/termios gated behind the `tty` feature) and **does not link any system C library** (no libubus/libubox), so musl targets can produce fully static binaries:

```sh
rustup target add aarch64-unknown-linux-musl x86_64-unknown-linux-musl

# aarch64 (common router target)
cargo build --release -p qmodem-atd --target aarch64-unknown-linux-musl
# x86_64
cargo build --release -p qmodem-atd --target x86_64-unknown-linux-musl

file target/aarch64-unknown-linux-musl/release/qmodem-atd
# statically linked → can be copied directly into the firmware and run
```

Notes (consistent with the downgrade path in `rust/README.md`):

- `qmodem-pdu` depends only on thiserror; in extreme scenarios it can degrade to zero dependencies (hand-written Display impls);
- `qmodem-atd` relies on tokio's `select!` semantics, which cannot be equivalently replaced by `std::thread` by design, so tokio is the minimal, non-degradable set of external crates;
- Without a musl toolchain, you can use the nightly `-Z build-std` approach or the OpenWrt SDK sysroot instead; the artifacts are equivalent.

## 5. ubus FFI roadmap

`src/ubus_ffi.rs` already contains an FFI skeleton (the `ubus` feature, off by default, declarations only, no references). Integration order (aligned with `rust/README.md` and section 5 of journal-rust-core.md):

1. **ubus method table bridging**: implement `sendat` / `list` / `event_callback*` / `open` / `close` / `load_conf` following the method table in `ubus_at_daemon.h:266-293`; the ubus handler does not block, requests are uniformly posted to the AtPort's mpsc queue and results collected via oneshot.
2. **Side by side with the JSON service**: `service` (unix socket) remains the test interface, `ubus` evolves in parallel as the production interface, eventually replacing the packaging path of the C version `ubus_at_daemon`.
3. **Configuration loading**: align with the JSON configuration of `config_loader.c` (port list, baud rate, event callback scripts); event callbacks switch to `tokio::process` with argument arrays, eliminating the command-injection surface of the C version's event_callback.c (already fixed on the C side in journal-c-core.md).
4. **Cross-compilation integration**: `--features ubus` requires linking libubus/libubox via the OpenWrt SDK sysroot (the pure-musl zero-system-library path in section 4 no longer applies in that case).

For larger follow-up migrations (the vendor rule engine, smsd, TtyPort AsyncFd hardening), see "Follow-up migration plan" in `rust/README.md`.

## 6. AT port exclusivity (hard constraint)

The AT serial port is an **exclusive resource**. Exactly one process in the whole system may hold it — `qmodem-atd`. This is an architectural hard constraint, not a style suggestion:

1. **Single holder**: only `qmodem-atd` may open the modem's AT serial ports (`/dev/ttyUSB*` etc.). Each port is owned exclusively by a single actor thread holding `Box<dyn SerialPort>` with zero shared read/write paths (see section 1).
2. **All consumers go through the socket**: every component that needs AT data — shell (`modem_util.sh` → `qmodem_atd_sendat`), LuCI, voipd, smsd, stats — must use the `sendat` command on `/var/run/qmodem/atd.sock` (commands execute strictly serially in the per-port queue) or `subscribe` for the URC stream. **Opening the serial port directly is forbidden.**
3. **No mixed paths**: during the transition the legacy direct backends (`at_tool=0` → `tom_modem`, `at_tool=1` → `sms_tool_q`, the C `ubus_at_daemon`) are mutually exclusive with `qmodem-atd` until retirement — two active paths on the same port are never allowed. `at_tool=2` (the default) is the qmodem-atd exclusive mode.
4. **Single instance**: `/etc/init.d/qmodem-atd` is procd-managed, guaranteeing one daemon instance; never start a second `qmodem-atd` against the same socket/ports by hand.

Consequences of violating this (observed on a real device): when several processes open the same ttyUSB port concurrently, the modem delivers AT responses to whichever process happens to read first — process A's `AT+CPIN?` receives the response to process B's command (crossed responses), URCs get stolen, and diagnostics become meaningless. qmodem-atd's queued single-point access eliminates this class of failure at the root.

Checklist when a new component needs AT access:

- [ ] Does it go through the socket (`sendat` / `subscribe`), with no direct `open`/`tom_modem`/`sms_tool` calls in the code?
- [ ] Does it handle `status: timeout` (partial lines) and `error: port_closed`?
- [ ] Does it avoid high-frequency polling (queuing inflates latency for other requests on the same port)?
