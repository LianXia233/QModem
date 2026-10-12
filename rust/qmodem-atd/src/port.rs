//! Per-port AT command actor (std::thread edition).
//!
//! Replaces the C `reader_thread_func` + `send_at_command_with_response`
//! pair (at_handler.c:123-236,299-465) and the previous tokio actor:
//!
//! * one thread exclusively owns the transport — no cross-thread `fd` use,
//!   no UAF;
//! * commands arrive on a per-port bounded mpsc queue and run strictly
//!   serially; each caller gets a private std-mpsc oneshot instead of
//!   racing on the single `current_response` slot;
//! * the deadline is plain `Instant` arithmetic (monotonic clock) and the
//!   in-flight wait is bounded `recv_timeout`/read slices, never
//!   `CLOCK_REALTIME`;
//! * on timeout the partial response collected so far is returned with
//!   [`ResponseStatus::Timeout`];
//! * URC/data lines are published on the hand-written broadcast hub with
//!   `seq`/`epoch` counters mirroring the C line_events queue semantics
//!   (line_events.c / ubus_at_daemon.h:66-89).

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender, SyncSender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::line::{AtLine, LineHub, LineReceiver};
use crate::transport::{ReadOutcome, SerialPort};

/// Outcome of a queued AT command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseStatus {
    /// A configured/default end flag matched.
    Ok,
    /// Deadline expired before an end flag arrived.
    Timeout,
}

/// Response payload for one AT command.
#[derive(Debug, Clone)]
pub struct AtResponse {
    pub status: ResponseStatus,
    /// Lines received (including the matched end-flag line on success;
    /// partial lines on timeout). Echo lines are excluded.
    pub lines: Vec<String>,
    /// The end flag that matched, empty on timeout.
    pub end_flag: String,
    /// Wall time the command spent queued + executing (monotonic ms).
    pub elapsed_ms: u64,
}

/// Error returned to a waiting caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtError {
    /// The port thread is gone (transport closed or daemon shutting down).
    PortClosed,
}

impl fmt::Display for AtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AtError::PortClosed => write!(f, "port closed"),
        }
    }
}

impl std::error::Error for AtError {}

/// A queued AT command request.
#[derive(Debug, Clone)]
pub struct AtRequest {
    /// Command text without terminator (CR is appended by the actor).
    pub at: String,
    pub timeout_ms: u64,
    /// Custom end flag; `None` uses the default set from
    /// `parse_end_flags` (at_handler.c:28-77).
    pub end_flag: Option<String>,
}

/// Cloneable per-port client handle.
#[derive(Clone)]
pub struct AtHandle {
    name: String,
    tx: SyncSender<PortMsg>,
    hub: Arc<LineHub>,
    shutdown: Arc<AtomicBool>,
}

type PortMsg = (AtRequest, Sender<Result<AtResponse, AtError>>);

/// Configuration knobs for a port actor.
#[derive(Debug, Clone)]
pub struct AtPortConfig {
    /// Broadcast queue capacity per subscriber (C: AT_LINE_QUEUE_CAPACITY).
    pub line_queue_capacity: usize,
    /// Command queue depth.
    pub command_queue_depth: usize,
}

impl Default for AtPortConfig {
    fn default() -> Self {
        AtPortConfig {
            line_queue_capacity: 256,
            command_queue_depth: 64,
        }
    }
}

const DEFAULT_END_FLAGS: [&str; 5] = ["OK", "ERROR", "+CMS ERROR:", "+CME ERROR:", "NO CARRIER"];

/// Idle wait slice: bounds command pickup and shutdown latency.
const IDLE_SLICE: Duration = Duration::from_millis(50);
/// In-flight read slice: bounds deadline/shutdown overshoot.
const READ_SLICE: Duration = Duration::from_millis(20);

/// Spawn the port actor thread. Returns the client handle immediately.
pub fn spawn_port(
    name: impl Into<String>,
    port: Box<dyn SerialPort>,
    epoch: u64,
    config: AtPortConfig,
) -> AtHandle {
    let name = name.into();
    let (tx, rx) = mpsc::sync_channel(config.command_queue_depth);
    let hub = Arc::new(LineHub::with_capacity(config.line_queue_capacity));
    let shutdown = Arc::new(AtomicBool::new(false));
    let actor_name = name.clone();
    let actor_hub = hub.clone();
    let actor_shutdown = shutdown.clone();
    // On actor panic its locals (including pending resp_tx values) drop,
    // so waiting callers observe PortClosed instead of hanging.
    let _ = std::thread::Builder::new()
        .name(format!("at-port:{actor_name}"))
        .spawn(move || {
            run_actor(actor_name, port, rx, actor_hub, actor_shutdown, epoch);
        });
    AtHandle {
        name,
        tx,
        hub,
        shutdown,
    }
}

impl AtHandle {
    /// Queue an AT command and block until its response arrives.
    pub fn send_at(
        &self,
        at: &str,
        timeout_ms: u64,
        end_flag: Option<&str>,
    ) -> Result<AtResponse, AtError> {
        let (rtx, rrx) = mpsc::channel();
        let req = AtRequest {
            at: at.to_string(),
            timeout_ms,
            end_flag: end_flag.map(str::to_string),
        };
        self.tx.send((req, rtx)).map_err(|_| AtError::PortClosed)?;
        rrx.recv().map_err(|_| AtError::PortClosed)?
    }

    /// Subscribe to the line broadcast (URC + response lines).
    pub fn subscribe(&self) -> LineReceiver {
        self.hub.subscribe()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Request actor shutdown; pending requests fail with
    /// [`AtError::PortClosed`].
    pub fn close(&self) {
        self.shutdown.store(true, Ordering::Release);
    }
}

struct Pending {
    lines: Vec<String>,
    end_flags: Vec<String>,
    resp_tx: Sender<Result<AtResponse, AtError>>,
    started: Instant,
    deadline: Instant,
}

impl Pending {
    fn new(req: &AtRequest, resp_tx: Sender<Result<AtResponse, AtError>>) -> Pending {
        let end_flags = match &req.end_flag {
            Some(f) if !f.trim().is_empty() => vec![f.clone()],
            _ => DEFAULT_END_FLAGS.iter().map(|s| s.to_string()).collect(),
        };
        Pending {
            lines: Vec::new(),
            end_flags,
            resp_tx,
            started: Instant::now(),
            deadline: Instant::now()
                + Duration::from_millis(req.timeout_ms.max(1)),
        }
    }

    /// Mirrors `check_end_flags` (at_handler.c:79-121): exact match, or
    /// prefix match where the flag ends in ':' or a space follows.
    fn matches_end_flag(&self, line: &str) -> Option<String> {
        let t = line.trim();
        for f in &self.end_flags {
            if t == f {
                return Some(f.clone());
            }
            if t.starts_with(f)
                && (f.ends_with(':')
                    || t[f.len()..]
                        .chars()
                        .next()
                        .map(|c| c == ' ' || c == '\t')
                        .unwrap_or(false))
            {
                return Some(f.clone());
            }
        }
        None
    }

    fn finish(mut self, status: ResponseStatus, end_flag: String) {
        let _ = self.resp_tx.send(Ok(AtResponse {
            status,
            lines: std::mem::take(&mut self.lines),
            end_flag,
            elapsed_ms: self.started.elapsed().as_millis() as u64,
        }));
    }

    fn fail(self) {
        let _ = self.resp_tx.send(Err(AtError::PortClosed));
    }
}

fn run_actor(
    name: String,
    mut port: Box<dyn SerialPort>,
    rx: mpsc::Receiver<PortMsg>,
    hub: Arc<LineHub>,
    shutdown: Arc<AtomicBool>,
    epoch: u64,
) {
    let mut seq: u64 = 0;
    let mut buf = vec![0u8; 1024];
    let mut line_buf: Vec<u8> = Vec::new();
    let mut pending: Option<Pending> = None;
    // Last written command text, used to suppress its echo.
    let mut last_cmd: String = String::new();

    loop {
        if shutdown.load(Ordering::Acquire) {
            break;
        }

        // Monotonic deadline for the in-flight command.
        if pending.as_ref().is_some_and(|p| Instant::now() >= p.deadline) {
            if let Some(p) = pending.take() {
                p.finish(ResponseStatus::Timeout, String::new());
            }
            continue;
        }

        // Accept a new command only when idle; while a command is in
        // flight further requests simply queue in the bounded channel.
        if pending.is_none() {
            match rx.try_recv() {
                Ok((req, resp_tx)) => {
                    let mut data = req.at.clone().into_bytes();
                    data.push(b'\r');
                    if port.write_all(&data).is_err() {
                        let _ = resp_tx.send(Err(AtError::PortClosed));
                        break;
                    }
                    last_cmd = req.at.trim().to_string();
                    pending = Some(Pending::new(&req, resp_tx));
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => break,
            }
        }

        // Read the port with a bounded slice so deadlines and shutdown
        // stay responsive; the blocking read lives inside this dedicated
        // thread (single owner, no cross-thread fd use).
        let slice = match pending.as_ref() {
            Some(p) => p
                .deadline
                .saturating_duration_since(Instant::now())
                .min(READ_SLICE),
            None => IDLE_SLICE,
        };
        match port.read_chunk(&mut buf, slice) {
            Ok(ReadOutcome::Data(n)) => {
                for b in &buf[..n] {
                    line_buf.push(*b);
                    if *b == b'\n' {
                        let mut line = std::mem::take(&mut line_buf);
                        while matches!(line.last(), Some(b'\r') | Some(b'\n')) {
                            line.pop();
                        }
                        if line.is_empty() {
                            continue;
                        }
                        let text = String::from_utf8_lossy(&line).into_owned();
                        // Suppress the command echo: the modem
                        // repeats exactly the command text (C
                        // suppressed any "AT"-prefixed line,
                        // at_handler.c:354, which would also eat
                        // legitimate response lines like
                        // "AT+... ECHO").
                        if text == last_cmd {
                            continue;
                        }
                        seq += 1;
                        let _ = hub.send(AtLine {
                            port: name.clone(),
                            line: text.clone(),
                            seq,
                            epoch,
                        });
                        if let Some(p) = pending.as_mut() {
                            if let Some(flag) = p.matches_end_flag(&text) {
                                p.lines.push(text);
                                if let Some(p) = pending.take() {
                                    p.finish(ResponseStatus::Ok, flag);
                                }
                            } else {
                                p.lines.push(text);
                            }
                        }
                    }
                }
            }
            Ok(ReadOutcome::TimedOut) => {}
            Ok(ReadOutcome::Eof) | Err(_) => break,
        }
    }

    // Drain: fail any queued/pending request so callers see PortClosed
    // instead of hanging (C version leaves waiters on a dead condvar).
    if let Some(p) = pending.take() {
        p.fail();
    }
    while let Ok((_, resp_tx)) = rx.try_recv() {
        let _ = resp_tx.send(Err(AtError::PortClosed));
    }
}
