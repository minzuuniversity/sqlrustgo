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
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
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

    /// Immediately after a WAL entry has been appended to the log file,
    /// *before* any `flush`/`fsync`.
    ///
    /// This is the **append** boundary, not a durability boundary. A crash
    /// here may lose the entry (it was only `write(2)`'d into the kernel
    /// page cache). Tests use it to check the recovery path copes with a
    /// torn or missing trailing entry, and that earlier `fsync`ed entries
    /// still replay.
    AfterWALAppend,

    /// Immediately after the on-disk secondary index has been updated to
    /// point at a just-written page, but *before* that page's data has
    /// been flushed.
    ///
    /// This is the window where an index entry can reference a page whose
    /// contents never reached disk. Recovery must either re-derive the
    /// index from the page store or refuse to serve reads that would hit
    /// the dangling entry (fail closed), never silently return wrong rows.
    AfterIndexUpdate,
}

impl FaultPoint {
    pub fn as_str(self) -> &'static str {
        match self {
            FaultPoint::AfterCommitFsync => "after_commit_fsync",
            FaultPoint::AfterWALAppend => "after_wal_append",
            FaultPoint::AfterIndexUpdate => "after_index_update",
        }
    }

    /// Index of this point in the per-point counters.
    ///
    /// #5556: the counters are **per point**, not global: a process may
    /// exercise several points (the `a06_failpoint_reachability` test
    /// does exactly that), and a global "already fired" flag would make
    /// the second point silently never announce itself.
    fn index(self) -> usize {
        match self {
            FaultPoint::AfterCommitFsync => 0,
            FaultPoint::AfterWALAppend => 1,
            FaultPoint::AfterIndexUpdate => 2,
        }
    }

    pub fn all() -> &'static [FaultPoint] {
        &[
            FaultPoint::AfterCommitFsync,
            FaultPoint::AfterWALAppend,
            FaultPoint::AfterIndexUpdate,
        ]
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
    // #5556: which occurrence of the point is the boundary under test.
    // Default 1 = the first time the point is reached.
    let occurrence = std::env::var("SQLRUSTGO_FAULT_OCCURRENCE")
        .ok()
        .and_then(|v| v.trim().parse::<u32>().ok())
        .filter(|n| *n >= 1)
        .unwrap_or(1);
    Some(FaultConfig {
        point,
        signal_dir: PathBuf::from(dir),
        run_id,
        occurrence,
    })
}

#[derive(Debug, Clone)]
pub struct FaultConfig {
    pub point: FaultPoint,
    pub signal_dir: PathBuf,
    pub run_id: String,
    /// Which occurrence of the point holds the process (#5556).
    pub occurrence: u32,
}

impl FaultConfig {
    pub fn signal_file(&self) -> PathBuf {
        self.signal_dir
            .join(format!("{}.reached", self.point.as_str()))
    }
}

/// Block until the process is killed, if this is the armed point.
///
/// Returns immediately when fault injection is off or when a different
/// point is armed, so one binary serves both cases.
///
/// #5420: this definition **must** be gated on the feature, exactly like the
/// no-op twin below. The module itself is compiled when `cfg(test)` is on
/// *or* the feature is on (`crates/storage/src/lib.rs`), so in a **lib-test
/// build with the feature off** both definitions used to exist at the same
/// time and rustc rejected the crate:
///
/// ```text
/// error[E0428]: the name `reach` is defined multiple times
///    --> crates/storage/src/fault_injection.rs:188:1
///     |
/// 156 | pub fn reach(point: FaultPoint) {
///     | ------------------------------- previous definition of the value `reach` here
/// ```
///
/// That broke `cargo test --lib -p sqlrustgo-storage` (the RC/GA gate's
/// D1-A2 dimension), which is exactly the configuration the gate runs.
#[cfg(feature = "fault-injection")]
pub fn reach(point: FaultPoint) {
    let Some(cfg) = from_env() else { return };
    if cfg.point != point {
        return;
    }

    // #5556: the point holds the process **once**, at the configured
    // occurrence.
    //
    // Before this, every occurrence held: the server wrote the signal and
    // then spun for up to 120 s, and a script with N WAL-writing
    // statements paid N × 120 s. Measured on this tree (macOS arm64):
    // `cargo test -p sqlrustgo-storage --test crash_commit_fsync_b1` took
    // 240.93 s for two cases — and on CI the member job has a 5400 s
    // budget for the whole workspace, which three such targets could
    // eat on their own.
    //
    // The occurrence counter is what makes "hold once" safe: the crash
    // test needs the crash to land on **the boundary under test** (the
    // explicit COMMIT, not the autocommit seed), and "hold at the first
    // occurrence" would move the crash to the wrong place.
    let idx = point.index();
    let seen = OCCURRENCES[idx].fetch_add(1, Ordering::SeqCst) + 1;
    if seen < cfg.occurrence {
        return;
    }
    // Already held once for this point: pass through. The boundary is a
    // single point in the process's life, not a mode the process stays in.
    if FIRED[idx].swap(true, Ordering::SeqCst) {
        return;
    }

    write_signal(&cfg);

    // Spin rather than block forever on a condition nobody will satisfy:
    // the harness kills us. The timeout is a backstop so a harness that
    // dies first cannot leave the server running indefinitely.
    //
    // #5556: the duration is configurable (`SQLRUSTGO_FAULT_HOLD_SECS`,
    // default 120 s). A harness that reacts to the signal immediately
    // cuts the hold short with its SIGKILL, so the duration only matters
    // for the cases where the crash is meant to land **after** the
    // script — and there a 120 s hold is 120 s of wall clock for nothing.
    let hold = std::env::var("SQLRUSTGO_FAULT_HOLD_SECS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(120);
    let deadline = std::time::Instant::now() + Duration::from_secs(hold);
    while std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// How many times each point has been reached in this process (#5556).
#[cfg(feature = "fault-injection")]
static OCCURRENCES: [AtomicU32; 3] = [AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0)];

/// Whether each point has already held the process (#5556).
#[cfg(feature = "fault-injection")]
static FIRED: [AtomicBool; 3] = [
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
];

/// #5420: gated with its only caller — in a build without the feature this
/// function would be dead code, and the crate already carries a warning
/// budget it should not spend on that.
#[cfg(feature = "fault-injection")]
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
