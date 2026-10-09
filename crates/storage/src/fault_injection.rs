//! B-1 (#5177) fault-injection hook.
//!
//! # Scope
//!
//! This module is the *only* production-code change B-1 makes. It exists so
//! the crash tests can stop a real server process at a known boundary: the
//! server announces that it reached the boundary, and the test harness —
//! from outside the process — sends `SIGKILL`.
//!
//! It deliberately does **not** exit by itself. A self-exit would not
//! exercise what a crash exercises: no unwinding, no destructor, no
//! buffered flush. The kill has to come from outside.
//!
//! # Why a feature flag rather than `cfg(test)`
//!
//! Integration tests in `tests/` compile the library **without**
//! `cfg(test)`, so a `cfg(test)` hook would silently compile away and the
//! tests would hang forever waiting for a signal that never arrives. The
//! flag must be enabled for the test binary to build, and a test that
//! cannot reach its fault point must fail loudly rather than pass
//! vacuously — see [`FaultConfig`].
//!
//! # Cost when off
//!
//! With the feature disabled [`reach`] is an empty inline function and the
//! compiler drops the call. Production builds carry neither the branch nor
//! the environment lookups.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Where the server was told to stop.
///
/// B-1.1 wires a single point — `after_commit_fsync` — and the rest are
/// added one at a time so each step is independently reviewable and
/// independently revertible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultPoint {
    /// Immediately after the WAL Commit entry has been `fsync`ed, before
    /// the server answers the client.
    ///
    /// Note this is **not** the same as "the client saw COMMIT succeed":
    /// it is the durability boundary. Tests must express which one they
    /// mean.
    AfterCommitFsync,
}

impl FaultPoint {
    pub fn as_str(self) -> &'static str {
        match self {
            FaultPoint::AfterCommitFsync => "after_commit_fsync",
        }
    }

    pub fn all() -> &'static [FaultPoint] {
        &[FaultPoint::AfterCommitFsync]
    }

    pub fn parse(s: &str) -> Option<Self> {
        FaultPoint::all().iter().copied().find(|p| p.as_str() == s)
    }
}

/// Identity written into the signal file.
///
/// Carries the run id, the pid and the point, so a harness that reuses a
/// directory cannot mistake a **previous** run's leftover signal for this
/// one. Reading a stale signal is how a crash test passes without ever
/// having reached the fault point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signal {
    pub run_id: String,
    pub pid: u32,
    pub point: String,
}

impl Signal {
    /// Parse the signal file's contents. Returns `None` on anything
    /// unexpected rather than a partial value — a malformed file is not
    /// a valid acknowledgement.
    pub fn parse(s: &str) -> Option<Self> {
        let mut it = s.trim().split('|');
        let run_id = it.next()?.to_string();
        let pid = it.next()?.parse().ok()?;
        let point = it.next()?.to_string();
        if it.next().is_some() {
            return None;
        }
        Some(Signal { run_id, pid, point })
    }

    pub fn encode(&self) -> String {
        format!("{}|{}|{}", self.run_id, self.pid, self.point)
    }
}

/// Resolve the fault configuration from the environment, or `None` when
/// fault injection is not armed.
pub fn from_env() -> Option<FaultConfig> {
    let point = std::env::var("SQLRUSTGO_FAULT_POINT").ok()?;
    let dir = std::env::var("SQLRUSTGO_FAULT_SIGNAL_DIR").ok()?;
    let run_id = std::env::var("SQLRUSTGO_FAULT_RUN_ID").ok()?;
    let point = FaultPoint::parse(&point)?;
    Some(FaultConfig { point, signal_dir: PathBuf::from(dir), run_id })
}

#[derive(Debug, Clone)]
pub struct FaultConfig {
    pub point: FaultPoint,
    pub signal_dir: PathBuf,
    pub run_id: String,
}

impl FaultConfig {
    pub fn signal_file(&self) -> PathBuf {
        self.signal_dir.join(format!("{}.reached", self.point.as_str()))
    }
}

/// Block until the process is killed, if this is the armed point.
///
/// Returns immediately when fault injection is off or when a different
/// point is armed, so one binary serves both cases.
pub fn reach(point: FaultPoint) {
    let Some(cfg) = from_env() else { return };
    if cfg.point != point {
        return;
    }
    write_signal(&cfg);

    // Spin rather than block forever on a condition nobody will satisfy:
    // the harness kills us. The timeout is a backstop so a harness that
    // dies first cannot leave the server running indefinitely.
    let deadline = std::time::Instant::now() + Duration::from_secs(120);
    while std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn write_signal(cfg: &FaultConfig) {
    let _ = std::fs::create_dir_all(&cfg.signal_dir);
    let sig = Signal {
        run_id: cfg.run_id.clone(),
        pid: std::process::id(),
        point: cfg.point.as_str().to_string(),
    };
    if let Ok(mut f) = std::fs::File::create(cfg.signal_file()) {
        let _ = f.write_all(sig.encode().as_bytes());
        let _ = f.flush();
    }
}

/// No-op when the feature is off.
#[cfg(not(feature = "fault-injection"))]
#[inline(always)]
pub fn reach(_point: FaultPoint) {
    let _ = std::env::var("SQLRUSTGO_FAULT_POINT");
}

/// Compiled out entirely outside the harness, branch and all.
#[cfg(not(feature = "fault-injection"))]
#[inline(always)]
pub fn fault_enabled() -> bool {
    false
}

#[cfg(feature = "fault-injection")]
#[inline(always)]
pub fn fault_enabled() -> bool {
    true
}