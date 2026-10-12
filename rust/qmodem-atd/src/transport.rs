//! Serial transport abstraction.
//!
//! The C daemon hands a raw `int fd` to a reader pthread while the command
//! path writes to the same fd under a mutex (at_handler.c:193-195,
//! port_manager.c). Here the port actor thread owns the transport
//! exclusively; the [`SerialPort`] trait is synchronous with an explicit
//! read timeout, so the single actor thread can interleave command-queue
//! polling, deadline checks and shutdown signalling without any external
//! runtime. Tests substitute a [`LoopbackPort`]; the `tty` feature supplies
//! a real [`TtyPort`](self) built on raw `extern "C"` termios declarations
//! (no libc crate).

use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// Outcome of a [`SerialPort::read_chunk`] call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadOutcome {
    /// `n` bytes were read into the caller's buffer.
    Data(usize),
    /// No bytes arrived before `timeout` elapsed.
    TimedOut,
    /// Port closed / EOF (buffered bytes are drained before Eof).
    Eof,
}

/// Scripted response callback: AT command text -> response lines.
pub type Responder = Box<dyn FnMut(&str) -> Vec<String> + Send>;

/// Serial transport owned by one port actor thread.
pub trait SerialPort: Send + 'static {
    /// Write a whole buffer.
    fn write_all(&mut self, data: &[u8]) -> io::Result<()>;

    /// Read into `buf`, waiting at most `timeout`.
    fn read_chunk(&mut self, buf: &mut [u8], timeout: Duration) -> io::Result<ReadOutcome>;
}

#[cfg(feature = "tty")]
pub mod tty {
    //! Raw tty transport via termios (`feature = "tty"`).
    //!
    //! Uses `O_NONBLOCK` plus short sleeps inside the read call so the port
    //! thread never blocks indefinitely; the FFI is declared inline with
    //! `extern "C"`, keeping the build free of external crates.

    use super::{ReadOutcome, SerialPort};
    use std::io;
    use std::os::fd::RawFd;
    use std::thread;
    use std::time::{Duration, Instant};

    extern "C" {
        fn open(path: *const u8, flags: i32, ...) -> i32;
        fn close(fd: i32) -> i32;
        fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
        fn write(fd: i32, buf: *const u8, count: usize) -> isize;
        fn tcgetattr(fd: i32, t: *mut Termios) -> i32;
        fn tcsetattr(fd: i32, act: i32, t: *const Termios) -> i32;
        fn cfsetispeed(t: *mut Termios, speed: u32) -> i32;
        fn cfsetospeed(t: *mut Termios, speed: u32) -> i32;
    }

    // Mirrors the platform `struct termios` on Linux (glibc/musl layout).
    #[repr(C)]
    struct Termios {
        c_iflag: u32,
        c_oflag: u32,
        c_cflag: u32,
        c_lflag: u32,
        c_line: u8,
        c_cc: [u8; 32],
        c_ispeed: u32,
        c_ospeed: u32,
    }

    const O_RDWR: i32 = 0o2;
    const O_NOCTTY: i32 = 0o400;
    const O_NONBLOCK: i32 = 0o4000;
    const TCSANOW: i32 = 0;
    const B115200: u32 = 0o100002;

    // c_iflag / c_oflag / c_lflag clears for raw mode.
    const IGNBRK: u32 = 1;
    const BRKINT: u32 = 2;
    const PARMRK: u32 = 8;
    const ISTRIP: u32 = 0o40;
    const INLCR: u32 = 0o100;
    const IGNCR: u32 = 0o200;
    const ICRNL: u32 = 0o400;
    const IXON: u32 = 0o2000;
    const OPOST: u32 = 1;
    const ECHO: u32 = 8;
    const ECHONL: u32 = 0o100;
    const ICANON: u32 = 2;
    const ISIG: u32 = 1;
    const IEXTEN: u32 = 0o100000;
    const CSIZE: u32 = 0o60;
    const PARENB: u32 = 0o400;
    const CSTOPB: u32 = 0o100;
    const CS8: u32 = 0o60;
    const CREAD: u32 = 0o200;
    const CLOCAL: u32 = 0o4000;

    /// Real tty port. The fd is owned; dropped on close.
    pub struct TtyPort {
        fd: RawFd,
    }

    // Safety: the fd is exclusively owned by this value, which the port
    // actor moves into its single thread.
    unsafe impl Send for TtyPort {}

    impl TtyPort {
        /// Open `path` at 115200 8N1, raw mode, non-blocking.
        pub fn open(path: &str) -> io::Result<TtyPort> {
            let mut path_bytes = path.as_bytes().to_vec();
            path_bytes.push(0);
            // SAFETY: path is NUL-terminated; flags are read-only constants.
            let fd = unsafe { open(path_bytes.as_ptr(), O_RDWR | O_NOCTTY | O_NONBLOCK) };
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            let mut t = Termios {
                c_iflag: 0,
                c_oflag: 0,
                c_cflag: 0,
                c_lflag: 0,
                c_line: 0,
                c_cc: [0; 32],
                c_ispeed: 0,
                c_ospeed: 0,
            };
            // SAFETY: fd is a valid open descriptor, t is a valid termios.
            if unsafe { tcgetattr(fd, &mut t) } != 0 {
                let err = io::Error::last_os_error();
                unsafe { close(fd) };
                return Err(err);
            }
            t.c_iflag &= !(IGNBRK | BRKINT | PARMRK | ISTRIP | INLCR | IGNCR | ICRNL | IXON);
            t.c_oflag &= !OPOST;
            t.c_lflag &= !(ECHO | ECHONL | ICANON | ISIG | IEXTEN);
            t.c_cflag &= !(CSIZE | PARENB | CSTOPB);
            t.c_cflag |= CS8 | CREAD | CLOCAL;
            // SAFETY: valid pointers as above.
            unsafe {
                cfsetispeed(&mut t, B115200);
                cfsetospeed(&mut t, B115200);
                tcsetattr(fd, TCSANOW, &t);
            }
            Ok(TtyPort { fd })
        }
    }

    impl Drop for TtyPort {
        fn drop(&mut self) {
            // SAFETY: fd was opened by TtyPort::open and not closed before.
            unsafe { close(self.fd) };
        }
    }

    impl SerialPort for TtyPort {
        fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
            let mut off = 0;
            while off < data.len() {
                // SAFETY: fd valid; slice outlives the call.
                let n = unsafe { write(self.fd, data[off..].as_ptr(), data.len() - off) };
                if n < 0 {
                    let err = io::Error::last_os_error();
                    if err.kind() == io::ErrorKind::WouldBlock {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    return Err(err);
                }
                off += n as usize;
            }
            Ok(())
        }

        fn read_chunk(&mut self, buf: &mut [u8], timeout: Duration) -> io::Result<ReadOutcome> {
            let deadline = Instant::now() + timeout;
            loop {
                // SAFETY: fd valid; buf outlives the call.
                let n = unsafe { read(self.fd, buf.as_mut_ptr(), buf.len()) };
                if n > 0 {
                    return Ok(ReadOutcome::Data(n as usize));
                }
                if n == 0 {
                    return Ok(ReadOutcome::Eof);
                }
                let err = io::Error::last_os_error();
                match err.kind() {
                    io::ErrorKind::WouldBlock => {
                        if Instant::now() >= deadline {
                            return Ok(ReadOutcome::TimedOut);
                        }
                        thread::sleep(Duration::from_millis(5));
                    }
                    io::ErrorKind::Interrupted => continue,
                    _ => return Err(err),
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// LoopbackPort: programmable fake modem for tests
// ---------------------------------------------------------------------------

struct LoopInner {
    /// Bytes queued for the reader.
    rx: VecDeque<u8>,
    /// Responder invoked for each written command line (without CR).
    responder: Option<Responder>,
    closed: bool,
    /// Commands seen by the transport (test assertions).
    tx_log: Vec<String>,
}

/// Test double: echoes nothing, feeds scripted responses, injects URCs.
///
/// The handle side (tests) and the actor side share state via
/// `Arc<Mutex<..>>` plus a `Condvar` for wakeup; no raw pointers escape.
pub struct LoopbackPort {
    shared: Arc<LoopShared>,
}

struct LoopShared {
    inner: Mutex<LoopInner>,
    cv: Condvar,
}

/// Control side of a [`LoopbackPort`], usable from tests.
#[derive(Clone)]
pub struct LoopbackHandle {
    shared: Arc<LoopShared>,
}

impl Default for LoopbackPort {
    fn default() -> Self {
        Self::new()
    }
}

impl LoopbackPort {
    pub fn new() -> LoopbackPort {
        LoopbackPort {
            shared: Arc::new(LoopShared {
                inner: Mutex::new(LoopInner {
                    rx: VecDeque::new(),
                    responder: None,
                    closed: false,
                    tx_log: Vec::new(),
                }),
                cv: Condvar::new(),
            }),
        }
    }

    /// Get the test control handle.
    pub fn handle(&self) -> LoopbackHandle {
        LoopbackHandle {
            shared: self.shared.clone(),
        }
    }
}

impl LoopbackHandle {
    /// Set a responder mapping each written AT command to response lines
    /// (CRLF framing is added by the transport).
    pub fn set_responder(&self, f: Responder) {
        self.shared.inner.lock().unwrap().responder = Some(f);
    }

    /// One-shot convenience: the next command returns the scripted lines;
    /// afterwards commands get no response (drives timeout tests).
    pub fn set_response(&self, lines: Vec<String>) {
        let mut next = Some(lines);
        self.set_responder(Box::new(move |_cmd| {
            next.take().unwrap_or_default()
        }));
    }

    /// Inject an unsolicited line (URC) into the read stream immediately.
    pub fn push_urc(&self, line: &str) {
        self.push_str(line);
    }

    /// Close the port: reader observes EOF after draining.
    pub fn close(&self) {
        let mut g = self.shared.inner.lock().unwrap();
        g.closed = true;
        drop(g);
        self.shared.cv.notify_all();
    }

    /// Commands written so far (without CR terminators).
    pub fn written(&self) -> Vec<String> {
        self.shared.inner.lock().unwrap().tx_log.clone()
    }

    fn push_str(&self, s: &str) {
        let mut g = self.shared.inner.lock().unwrap();
        g.rx.extend(s.as_bytes());
        g.rx.extend(b"\r\n");
        drop(g);
        self.shared.cv.notify_all();
    }
}

impl SerialPort for LoopbackPort {
    fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        let lines = String::from_utf8_lossy(data).into_owned();
        let mut g = self.shared.inner.lock().unwrap();
        if g.closed {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "loopback closed",
            ));
        }
        for cmd in lines.trim_end_matches('\r').split('\r') {
            if cmd.is_empty() {
                continue;
            }
            g.tx_log.push(cmd.to_string());
            let resp = match g.responder.as_mut() {
                Some(f) => f(cmd),
                None => vec!["OK".to_string()],
            };
            for line in resp {
                g.rx.extend(line.as_bytes());
                g.rx.extend(b"\r\n");
            }
        }
        drop(g);
        self.shared.cv.notify_all();
        Ok(())
    }

    fn read_chunk(&mut self, buf: &mut [u8], timeout: Duration) -> io::Result<ReadOutcome> {
        let deadline = Instant::now() + timeout;
        let mut g = self.shared.inner.lock().unwrap();
        loop {
            if g.closed && g.rx.is_empty() {
                return Ok(ReadOutcome::Eof);
            }
            let mut n = 0;
            while n < buf.len() {
                match g.rx.pop_front() {
                    Some(b) => {
                        buf[n] = b;
                        n += 1;
                    }
                    None => break,
                }
            }
            if n > 0 {
                return Ok(ReadOutcome::Data(n));
            }
            let now = Instant::now();
            if now >= deadline {
                return Ok(ReadOutcome::TimedOut);
            }
            let (ng, _) = self.shared.cv.wait_timeout(g, deadline - now).unwrap();
            g = ng;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_eof_on_close() {
        let mut p = LoopbackPort::new();
        let h = p.handle();
        h.close();
        let mut buf = [0u8; 16];
        assert_eq!(
            p.read_chunk(&mut buf, Duration::from_millis(100)).unwrap(),
            ReadOutcome::Eof
        );
    }

    #[test]
    fn loopback_write_triggers_responder() {
        let mut p = LoopbackPort::new();
        let h = p.handle();
        h.set_response(vec!["hello".into(), "OK".into()]);
        p.write_all(b"AT+TEST\r").unwrap();
        let mut buf = [0u8; 64];
        let n = match p.read_chunk(&mut buf, Duration::from_millis(100)).unwrap() {
            ReadOutcome::Data(n) => n,
            other => panic!("expected data, got {other:?}"),
        };
        assert_eq!(&buf[..n], b"hello\r\nOK\r\n");
        assert_eq!(h.written(), vec!["AT+TEST".to_string()]);
    }

    #[test]
    fn loopback_read_times_out_when_silent() {
        let mut p = LoopbackPort::new();
        let mut buf = [0u8; 16];
        let t0 = Instant::now();
        assert_eq!(
            p.read_chunk(&mut buf, Duration::from_millis(40)).unwrap(),
            ReadOutcome::TimedOut
        );
        assert!(t0.elapsed() >= Duration::from_millis(35));
    }
}
