//! B-1.1 coordinator: drive a real server subprocess and kill it at
//! `after_commit_fsync`.
//!
//! # Flow
//!
//! ```text
//! start subprocess (fault point armed)
//!   ↓
//! run SQL through the `mysql` CLI
//!   ↓
//! server reaches the boundary and writes `<point>.reached` with run_id|pid|point
//!   ↓
//! parent reads the signal, verifies run_id and pid match, then SIGKILLs
//!   ↓
//! restart, wait until queryable
//!   ↓
//! verify the committed rows survived
//! ```
//!
//! # No sleeps
//!
//! The parent polls for the signal file and compares its `run_id`. A
//! leftover signal from an earlier run has a different run id and is
//! rejected, so the test cannot pass by reading stale state — which is
//! how a crash test ends up green without ever having crashed anything.
//!
//! # The boundary under test
//!
//! `after_commit_fsync` is the **durability** boundary: the WAL Commit
//! entry has been `fsync`ed. It is *not* the point at which the client
//! learns COMMIT succeeded. The two are separated by the protocol write
//! path, and a test that conflates them would overstate what it proves.
//! What `SIGKILL` proves is process-crash recovery; power-loss durability
//! needs the `fsync` boundary verified separately.

use sqlrustgo_storage::fault_injection::{FaultPoint, Signal};

use std::io::Write;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub const BOOT_TIMEOUT: Duration = Duration::from_secs(60);
pub const REACH_TIMEOUT: Duration = Duration::from_secs(60);

pub fn run_id(tag: &str) -> String {
    format!("{tag}_{}", std::process::id())
}

pub fn server_bin() -> PathBuf {
    std::env::var("SQLRUSTGO_SERVER_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("target/release/sqlrustgo-mysql-server"))
}

pub fn mysql_bin() -> PathBuf {
    std::env::var("SQLRUSTGO_MYSQL_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("mysql"))
}

/// A unique, self-cleaning directory. `Drop` removes it unless the test
/// asked to keep it, so a failing iteration leaves a reproducible
/// directory behind for inspection instead of poisoning the next run.
pub struct Scratch {
    pub path: PathBuf,
    keep: bool,
}

impl Scratch {
    pub fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!("b1_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create scratch dir");
        Self { path, keep: false }
    }

    pub fn keep(mut self) -> Self {
        self.keep = true;
        self
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

pub struct Server {
    child: Child,
    bin: PathBuf,
    data: PathBuf,
    port: u16,
}

impl Server {
    pub fn start(bin: &Path, data: &Path, port: u16, arm: Option<Arm>) -> Result<Self, String> {
        let mut c = Command::new(bin);
        c.arg("serve")
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(port.to_string())
            .arg("--data-dir")
            .arg(data)
            .arg("--server-threads")
            .arg("4")
            .arg("--storage")
            .arg("file")
            .arg("--auth-mode")
            .arg("none")
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        if let Some(a) = arm {
            c.env("SQLRUSTGO_FAULT_POINT", a.point.as_str());
            c.env("SQLRUSTGO_FAULT_SIGNAL_DIR", &a.signal_dir);
            c.env("SQLRUSTGO_FAULT_RUN_ID", &a.run_id);
            c.env("SQLRUSTGO_WAL_SYNC", &a.sync_mode);
        }

        let child = c.spawn().map_err(|e| format!("spawn server: {e}"))?;
        let s = Self {
            child,
            bin: bin.to_path_buf(),
            data: data.to_path_buf(),
            port,
        };
        s.wait_listening(BOOT_TIMEOUT)?;
        Ok(s)
    }

    /// Poll the port. A fixed sleep here would turn a slow start into a
    /// flaky "never came up" failure.
    fn wait_listening(&self, timeout: Duration) -> Result<(), String> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if TcpStream::connect(("127.0.0.1", self.port)).is_ok() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        Err(format!(
            "server did not listen on {} in {timeout:?}",
            self.port
        ))
    }

    /// SIGKILL: no unwinding, no destructors, no flush — a real crash.
    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn restart(self) -> Result<Self, String> {
        let (bin, data, port) = (self.bin.clone(), self.data.clone(), self.port);
        drop(self);
        Server::start(&bin, &data, port, None)
    }
}

/// Everything needed to arm one fault point in a child.
#[derive(Debug, Clone)]
pub struct Arm {
    pub point: FaultPoint,
    pub signal_dir: PathBuf,
    pub run_id: String,
    pub sync_mode: String,
}

impl Arm {
    pub fn new(point: FaultPoint, scratch: &Scratch, tag: &str) -> Self {
        Self {
            point,
            signal_dir: scratch.path.join("signals"),
            run_id: run_id(tag),
            sync_mode: "every".to_string(),
        }
    }

    pub fn with_sync_mode(mut self, m: &str) -> Self {
        self.sync_mode = m.to_string();
        self
    }

    pub fn signal_file(&self) -> PathBuf {
        self.signal_dir
            .join(format!("{}.reached", self.point.as_str()))
    }
}

/// Block until the child announces it reached the boundary.
///
/// Returns the parsed signal so the caller can check identity, not just
/// existence — a stale file from a previous run has a different run id.
pub fn await_signal(arm: &Arm, child_pid: u32) -> Result<Signal, String> {
    let f = arm.signal_file();
    let deadline = Instant::now() + REACH_TIMEOUT;
    while Instant::now() < deadline {
        if let Ok(s) = std::fs::read_to_string(&f) {
            if let Some(sig) = Signal::parse(&s) {
                if sig.run_id != arm.run_id {
                    return Err(format!(
                        "stale signal from a previous run: file has run_id={} but this run is {}",
                        sig.run_id, arm.run_id
                    ));
                }
                if sig.pid != child_pid {
                    return Err(format!(
                        "signal pid {} does not match the live child {}",
                        sig.pid, child_pid
                    ));
                }
                return Ok(sig);
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Err(format!(
        "server never reached fault point {} within {REACH_TIMEOUT:?}",
        arm.point.as_str()
    ))
}

pub fn sql(mysql: &Path, port: u16, script: &str) -> Result<(), String> {
    let out = Command::new(mysql)
        .arg("--host=127.0.0.1")
        .arg(format!("--port={port}"))
        .arg("--user=root")
        .arg("--batch")
        .arg("--execute")
        .arg(script)
        .output()
        .map_err(|e| format!("run mysql: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "script failed: {script}\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(())
}

pub fn scalar(mysql: &Path, port: u16, q: &str) -> Result<String, String> {
    let out = Command::new(mysql)
        .arg("--host=127.0.0.1")
        .arg(format!("--port={port}"))
        .arg("--user=root")
        .arg("--skip-column-names")
        .arg("--batch")
        .arg("--execute")
        .arg(q)
        .output()
        .map_err(|e| format!("run mysql: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "query failed: {q}\n{}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Wait until the restarted server answers queries at all — the listener
/// can bind before WAL replay finishes.
pub fn await_queryable(mysql: &Path, port: u16, timeout: Duration) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    let mut last = String::new();
    while Instant::now() < deadline {
        match scalar(mysql, port, "SELECT 1") {
            Ok(v) if v == "1" => return Ok(()),
            Ok(v) => last = v,
            Err(e) => last = e,
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(format!("server on {port} never became queryable: {last}"))
}

pub fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .unwrap_or(0)
}

/// What one iteration observed. Printed so a failure leaves a record.
#[derive(Debug, Clone)]
pub struct Observation {
    pub fault_point: String,
    pub sync_mode: String,
    pub run_id: String,
    pub child_pid: u32,
    pub reached: bool,
    pub expected_sum: i64,
    pub actual_sum: String,
    pub outcome: String,
}

impl Observation {
    pub fn line(&self) -> String {
        format!(
            "B1 point={} sync={} run_id={} pid={} reached={} expected={} actual={} outcome={}",
            self.fault_point,
            self.sync_mode,
            self.run_id,
            self.child_pid,
            self.reached,
            self.expected_sum,
            self.actual_sum,
            self.outcome
        )
    }
}

pub fn print_obs(o: &Observation) {
    println!("{}", o.line());
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("b1_observations.log")
        .expect("open observation log");
    let _ = writeln!(f, "{}", o.line());
}
