//! SOAK 起跑复验的**稳态窗口**回归套件 —— 2026-11-17 实测误杀事故
//!
//! 事故
//! ----
//! Z6G4（192.168.0.252）上的 168h SOAK 启动器有一道**吞吐复验**门禁：
//! 起跑后等 `VERIFY_SECS` 秒，看驱动真实跑了多少查询、错了多少；折算
//! QPS 低于 `MIN_DRIVER_QPS` 或错误率高于 `MAX_ERROR_RATE_PCT`，就说明
//! 机器还被别人压着 —— 停掉这一轮，回到等待。
//!
//! 那道复验把 `driver.log` 的第 2 列当成了「驱动启动以来的经过时间」：
//!
//! ```text
//! /^Q,/ { if ($2+0 >= 30000) q++; ... }
//! ```
//!
//! 但驱动写的第 2 列是**单条查询自己的耗时**（毫秒）—— `t0` 取在循环
//! 体内、每条查询之前（`v400_soak_driver.py`）。于是「elapsed ≥ 30 s」
//! 这个谓词**任何一条延迟采样都满足不了**（它要求单条查询跑满 30 s），
//! 稳态区间恒为空集 ⇒ 复验恒判「0 q/s、错误率 100 %」。
//!
//! **实测代价**：attempt-31 被这道仪器误杀。那一轮其实健康 —— 180 s 内
//! 9194 条查询（≈ 51 q/s）、0 错误 —— 却因为读错时间基准被判成
//! 「0 q/s vs floor 10」，白跑一轮。
//!
//! 修法
//! ----
//! 时间基准改从 armA 的 `metrics.csv` 取（15 s 一采样，带 `elapsed_s` 与
//! **累计** `driver_q` / `driver_e`），计算逻辑收敛到单一实现
//! `scripts/soak/soak_steady_window.py`。
//!
//! 口径（**读不出来 ≠ 达标**）
//! ---------------------------
//! - 稳态窗口 = `elapsed_s ≥ warmup`（默认 30 s，即热身段之后）到最后一个
//!   采样点之间的区间；窗口内**新增**的查询数 / 错误数才是分子分母。
//! - 稳态窗口短于 30 s ⇒ `SUSTAINED=0`（驱动多半还没跑热）。
//! - 读不出统计量（文件不存在 / 采样点不足 2 个 / 窗口长度为 0）⇒
//!   **非零退出码且不输出统计行**，调用方据此判红。

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn helper_path() -> PathBuf {
    repo_root()
        .join("scripts")
        .join("soak")
        .join("soak_steady_window.py")
}

fn launcher_path() -> PathBuf {
    repo_root()
        .join("scripts")
        .join("soak")
        .join("run_v410_168h_soak_on_z6g4.sh")
}

fn driver_path() -> PathBuf {
    repo_root()
        .join("scripts")
        .join("soak")
        .join("v400_soak_driver.py")
}

/// Write a metrics.csv fixture and return its path.
fn write_metrics(tag: &str, rows: &[(f64, u32, u32)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sqlrustgo_soak_steady_{}", tag));
    std::fs::create_dir_all(&dir).expect("create fixture dir");
    let path = dir.join("metrics.csv");
    let mut body = String::from(
        "ts,elapsed_s,rss_mb,fd_count,cpu_pct,driver_q,driver_e,panic_count,error_high,host_load1,host_busy\n",
    );
    for (i, (elapsed, q, e)) in rows.iter().enumerate() {
        body.push_str(&format!(
            "2026-11-17T00:{:02}:{:02},{:.0},191.0,47,118,{},{:02},00,0,28.83,22\n",
            i / 4,
            i % 4,
            elapsed,
            q,
            e
        ));
    }
    std::fs::write(&path, body).expect("write fixture");
    path
}

/// Run the helper; returns (exit_code, stdout, stderr).
fn run_helper(csv: &Path, warmup: &str) -> (i32, String, String) {
    let out = Command::new("python3")
        .arg(helper_path())
        .arg(csv)
        .arg(warmup)
        .current_dir(repo_root())
        .output()
        .expect("failed to spawn python3");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

/// The steady window must be computed from `elapsed_s` + the **cumulative**
/// counters, and the warm-up rows (elapsed < 30 s) must be excluded.
///
/// Fixture: driver_q grows 100 → 200 → 400 → 600 → 900 at elapsed
/// 15/31/46/61/76 s. The warm-up row (15 s) must not be the baseline, so
/// the window is 31 s → 76 s: Δq = 700, Δe = 2, Δt = 45 s ⇒ 15.6 q/s.
#[test]
fn test_steady_window_excludes_warmup_and_uses_cumulative_counters() {
    let csv = write_metrics(
        "warmup",
        &[(15.0, 100, 0), (31.0, 200, 0), (46.0, 400, 1), (61.0, 600, 1), (76.0, 900, 2)],
    );
    let (code, stdout, stderr) = run_helper(&csv, "30");
    assert_eq!(code, 0, "stderr: {}", stderr);
    let line = stdout.trim();
    assert_eq!(
        line, "700 2 45 15.6 0.285 1",
        "steady window must be 31s→76s (Δq=700, Δe=2, Δt=45s, 15.6 q/s, 0.285% errors, sustained); got:\n{}",
        line
    );
}

/// A window shorter than 30 s must report `SUSTAINED=0` — the driver has
/// not warmed up yet, so its throughput says nothing about the host.
///
/// This is the **reverse anchor** of the test above: without it, a helper
/// that always returns 1 would pass.
#[test]
fn test_steady_window_too_short_is_not_sustained() {
    let csv = write_metrics("short", &[(31.0, 200, 0), (40.0, 400, 0)]);
    let (code, stdout, stderr) = run_helper(&csv, "30");
    assert_eq!(code, 0, "stderr: {}", stderr);
    let line = stdout.trim();
    assert_eq!(
        line, "200 0 9 22.2 0.000 0",
        "a 9 s steady window must not be reported as sustained; got:\n{}",
        line
    );
}

/// Unreadable input must fail closed: non-zero exit, **no** statistics
/// line. A caller that reads an empty line would otherwise see "0 q/s"
/// and could mistake it for "measured, and low" — or worse, for "fine".
#[test]
fn test_missing_metrics_file_fails_closed_without_statistics() {
    let missing = std::env::temp_dir().join("sqlrustgo_soak_steady_absent/nope.csv");
    let (code, stdout, stderr) = run_helper(&missing, "30");
    assert_ne!(code, 0, "missing metrics.csv must not exit 0");
    assert!(
        stdout.trim().is_empty(),
        "no statistics line may be printed when the input is unreadable; got:\n{}",
        stdout
    );
    assert!(
        stderr.to_lowercase().contains("fail"),
        "the failure must be stated, not silent; got:\n{}",
        stderr
    );
}

/// One sample is not a rate. Two are needed to have a window at all.
#[test]
fn test_single_sample_is_not_a_rate() {
    let csv = write_metrics("single", &[(40.0, 500, 0)]);
    let (code, stdout, stderr) = run_helper(&csv, "30");
    assert_ne!(code, 0, "a single sample must not exit 0");
    assert!(
        stdout.trim().is_empty(),
        "no statistics line may be printed for a single sample; got:\n{}",
        stdout
    );
    assert!(!stderr.trim().is_empty(), "the failure must be stated");
}

/// The launcher must actually use the helper, and must not keep a second
/// implementation of the same judgement inline.
///
/// This is the **anti-drift** anchor: the bug existed precisely because
/// the launcher carried its own copy of a computation whose input format
/// was misunderstood. If someone re-inlines an awk over `driver.log`,
/// this test fails.
#[test]
fn test_launcher_uses_the_single_steady_window_implementation() {
    let launcher = std::fs::read_to_string(launcher_path()).expect("read launcher");
    assert!(
        launcher.contains("soak_steady_window.py"),
        "the launcher must delegate the steady-window computation to \
         scripts/soak/soak_steady_window.py (one implementation, one rule)"
    );
    assert!(
        !launcher.contains(">= 30000"),
        "the launcher must not filter driver.log rows by a 30000-unit \
         elapsed threshold any more — that column is a per-query latency, \
         not an elapsed timestamp"
    );
}

/// The driver's own documentation must state what the column actually is.
///
/// The docstring said `elapsed_ms` while the code wrote a per-query
/// latency; that mismatch is what made the launcher's mistake look
/// reasonable. Pinning the wording keeps the next reader from repeating it.
#[test]
fn test_driver_documents_the_latency_column_as_latency() {
    let driver = std::fs::read_to_string(driver_path()).expect("read driver");
    assert!(
        driver.contains("Q,<query_latency_ms>"),
        "the driver must document the 2nd column as the query latency, \
         not as 'elapsed since start'"
    );
    assert!(
        !driver.contains("Q,<elapsed_ms>"),
        "the misleading 'Q,<elapsed_ms>' form must not come back"
    );
}
