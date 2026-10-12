//! Per-port AT command actor.
//!
//! Replaces the C `reader_thread_func` + `send_at_command_with_response`
//! pair (at_handler.c:123-236,299-465):
//!
//! * one task exclusively owns the transport — no cross-thread `fd` use,
//!   no UAF;
//! * commands arrive on a per-port mpsc queue and run strictly serially;
//!   each caller gets a private oneshot instead of racing on the single
//!   `current_response` slot;
//! * the deadline is a `tokio::time::sleep` (monotonic), not
//!   `CLOCK_REALTIME`;
//! * on timeout the partial response collected so far is returned with
//!   [`ResponseStatus::Timeout`].

use std::pin::Pin;
use std::time::{Duration, Instant};

use tokio::sync::watch;
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::line::AtLine;
use crate::transport::SerialPort;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AtError {
    /// The port task is gone (transport closed or daemon shutting down).
    #[error("port closed")]
    PortClosed,
}

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
    tx: mpsc::Sender<PortMsg>,
    line_tx: broadcast::Sender<AtLine>,
    shutdown: std::sync::Arc<watch::Sender<bool>>,
}

type PortMsg = (AtRequest, oneshot::Sender<Result<AtResponse, AtError>>);

/// Configuration knobs for a port actor.
#[derive(Debug, Clone)]
pub struct AtPortConfig {
    /// Broadcast channel capacity (C: AT_LINE_QUEUE_CAPACITY ring).
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

/// Spawn the port actor. Returns the client handle immediately.
pub fn spawn_port(
    name: impl Into<String>,
    port: Box<dyn SerialPort>,
    epoch: u64,
    config: AtPortConfig,
) -> AtHandle {
    let name = name.into();
    let (tx, rx) = mpsc::channel(config.command_queue_depth);
    let (line_tx, _) = broadcast::channel(config.line_queue_capacity);
    let (shutdown, sh_rx) = watch::channel(false);
    let actor_name = name.clone();
    let actor_line_tx = line_tx.clone();
    tokio::spawn(async move {
        run_actor(actor_name, port, rx, actor_line_tx, sh_rx, epoch).await;
    });
    AtHandle {
        name,
        tx,
        line_tx,
        shutdown: std::sync::Arc::new(shutdown),
    }
}

impl AtHandle {
    /// Queue an AT command and await its response.
    pub async fn send_at(
        &self,
        at: &str,
        timeout_ms: u64,
        end_flag: Option<&str>,
    ) -> Result<AtResponse, AtError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let req = AtRequest {
            at: at.to_string(),
            timeout_ms,
            end_flag: end_flag.map(str::to_string),
        };
        self.tx
            .send((req, tx))
            .await
            .map_err(|_| AtError::PortClosed)?;
        rx.await.map_err(|_| AtError::PortClosed)?
    }

    /// Subscribe to the line broadcast (URC + response lines).
    pub fn subscribe(&self) -> broadcast::Receiver<AtLine> {
        self.line_tx.subscribe()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Request actor shutdown; pending requests fail with
    /// [`AtError::PortClosed`].
    pub async fn close(&self) {
        let _ = self.shutdown.send(true);
        // Give the actor a chance to observe it.
        tokio::task::yield_now().await;
    }
}

struct Pending {
    lines: Vec<String>,
    end_flags: Vec<String>,
    resp_tx: oneshot::Sender<Result<AtResponse, AtError>>,
    started: Instant,
    deadline: Pin<Box<tokio::time::Sleep>>,
}

impl Pending {
    fn new(req: &AtRequest, resp_tx: oneshot::Sender<Result<AtResponse, AtError>>) -> Pending {
        let end_flags = match &req.end_flag {
            Some(f) if !f.trim().is_empty() => vec![f.clone()],
            _ => DEFAULT_END_FLAGS.iter().map(|s| s.to_string()).collect(),
        };
        Pending {
            lines: Vec::new(),
            end_flags,
            resp_tx,
            started: Instant::now(),
            deadline: Box::pin(tokio::time::sleep(Duration::from_millis(
                req.timeout_ms.max(1),
            ))),
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

async fn run_actor(
    name: String,
    mut port: Box<dyn SerialPort>,
    mut rx: mpsc::Receiver<PortMsg>,
    line_tx: broadcast::Sender<AtLine>,
    mut shutdown: watch::Receiver<bool>,
    epoch: u64,
) {
    let mut seq: u64 = 0;
    let mut buf = vec![0u8; 1024];
    let mut line_buf: Vec<u8> = Vec::new();
    let mut pending: Option<Pending> = None;
    // Last written command text, used to suppress its echo.
    let mut last_cmd: String = String::new();

    enum Stop {
        Shutdown,
        Eof,
        QueueClosed,
    }

    let _stop: Stop = loop {
        tokio::select! {
            biased;

            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    break Stop::Shutdown;
                }
            }

            // Accept a new command only when idle; while a command is in
            // flight further requests simply queue in the mpsc buffer.
            msg = rx.recv(), if pending.is_none() => {
                match msg {
                    None => break Stop::QueueClosed,
                    Some((req, resp_tx)) => {
                        let mut data = req.at.clone().into_bytes();
                        data.push(b'\r');
                        if let Err(_e) = port.write_all(&data).await {
                            let _ = resp_tx.send(Err(AtError::PortClosed));
                            break Stop::Eof;
                        }
                        last_cmd = req.at.trim().to_string();
                        pending = Some(Pending::new(&req, resp_tx));
                    }
                }
            }

            res = port.read_chunk(&mut buf) => {
                match res {
                    Ok(0) => break Stop::Eof,
                    Ok(n) => {
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
                                let _ = line_tx.send(AtLine {
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
                    Err(_) => break Stop::Eof,
                }
            }

            // Monotonic deadline for the in-flight command.
            _ = async {
                match pending.as_mut() {
                    Some(p) => p.deadline.as_mut().await,
                    None => std::future::pending::<()>().await,
                }
            }, if pending.is_some() => {
                if let Some(p) = pending.take() {
                    p.finish(ResponseStatus::Timeout, String::new());
                }
            }
        }
    };

    // Drain: fail any queued/pending request so callers see PortClosed
    // instead of hanging (C version leaves waiters on a dead condvar).
    if let Some(p) = pending.take() {
        p.fail();
    }
    rx.close();
    while let Ok((_, resp_tx)) = rx.try_recv() {
        let _ = resp_tx.send(Err(AtError::PortClosed));
    }
}
