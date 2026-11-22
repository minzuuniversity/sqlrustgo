// =============================================================================
// tests/soak/v312_mixed_soak.rs — v3.12.0 GA Mixed-Workload SOAK Harness (Issue #4387 / GA-2)
// =============================================================================
// Five-class mixed workload SOAK framework for v3.12.0 GA promotion.
//
// Five workload classes (mixed concurrently):
//   W1: OLTP transactional (INSERT/UPDATE/DELETE + simple SELECT)        ~30%
//   W2: Read-heavy analytical (point lookups + range scans)             ~25%
//   W3: Aggregation (GROUP BY, COUNT, SUM, AVG)                        ~15%
//   W4: DDL/schemalight (CREATE/ALTER/DROP, index creation)             ~10%
//   W5: Long-running reports (heavy JOINs, subqueries, window funcs)    ~20%
//
// #5449：这个文件此前**从未被编译、从未被执行** —— 它在 `tests/soak/` 下
// （cargo 不自动发现子目录），也没有 `[[test]]` 声明。
//
// 把它变成可执行目标后暴露出的问题，比「编译不过」更严重：原来的
// `WorkloadDriver::execute_one` **根本不碰数据库** —— 它只是
// `thread::sleep(模拟延迟)` 然后返回一个伪造的延迟值。注释写得很直白：
//
//   // Stub: real impl uses mysql_async / diesel connection pool.
//   // For GA-2 scaffold, simulate workload-specific latency.
//
// ⇒ 那个 60 秒的 "smoke" 跑的是 5 个线程的定时器，断言的是**模拟出来的**
// 失败率；GA-2 的 "SOAK PASS" 判的是**脚手架文件在不在**
// （`scripts/gate/check_ga_only_v312.sh:35` 只检查路径存在）。
// 当时 `scripts/soak/run_ga_soak.sh` 与 `scripts/soak/extract_soak_report.py`
// **根本不存在**。
//
// #5491：harness 变真之后，**判据还是没变** —— GA-2 仍然只检查脚手架在不在，
// 1h demo 仍然没有调用者，它写出的 summary 仍然没有消费者。于是：
//
//   * `scripts/soak/run_ga_soak.sh` —— **现在有了**：跑 harness、产出产物、
//     并把产物交给校验器；
//   * `scripts/gate/check_ga2_soak_artifact.sh` —— **现在有了**：GA-2 判据的
//     唯一实现（产物存在 / 不是模板 / 记录的是**跑完**的运行 / 覆盖五类负载 /
//     失败率在阈值内 / 足够新鲜）；
//   * `scripts/gate/check_ga_only_v312.sh` 与 `check_ga_v3.12.0.sh` 的 GA-2
//     条目改为**调用那个校验器**，不再问「脚手架文件在不在」。
//
// ⇒ 一个从不执行任何 SQL 的 harness，配上一个只检查文件存在的门禁，
// 制造的是**假覆盖**。本文件因此被改成**真的跑引擎**：
//
//   * 每个 workload class 持有自己的内存引擎（`ExecutionEngine<MemoryStorage>`），
//     按自己的形状建表并灌入种子数据；
//   * `execute_one` 执行**真实 SQL**，失败就是真实失败（SQL 错误会被记进
//     `ops_failed`，并让失败率断言变红）；
//   * 1h demo 仍然是 nightly-only 的忽略用例，但它现在 soak 的是真引擎。
//
// 设计约束（per STAGE.yaml line ~109 promotion_to_GA_requires #2）:
//   - 1h demo run (GA promotion) — full 168h SOAK runs in background
//   - No expiry 2027-06-30 / v3.13 deferral
//   - Runs as `cargo test --test v312_mixed_soak -- --ignored` for nightly
//   - Emits a per-workload-class summary (attempted / succeeded / failed /
//     avg / p99 latency) as a plain-text report.
//
// #5491: the header of this file used to claim the summary was
// "compatible with extract_soak_report.py". That was **false** and it was
// the reason the summary had no consumer: `scripts/soak/extract_soak_report.py`
// parses a **procfs metrics.csv** (RSS + file-descriptor counts) plus a
// mysqlslap log and emits a 14-field JSON SoakReport — this harness emits
// none of those. The two formats have nothing in common, so the reference is
// removed and the actual format is stated instead.
//
// The consumer of THIS summary is `scripts/gate/check_ga2_soak_artifact.sh`,
// which validates a GA-2 SOAK artifact: a completed run, all five workload
// classes, failure rates within threshold, and a fresh timestamp.
// `scripts/soak/run_ga_soak.sh` runs the harness and hands the summary to
// that validator.
//
// Integration points:
//   - Uses existing sqlrustgo-server harness (from tests/integration/stress/soak_test.rs)
//   - Uses crud_templates.py / tpch_soak_driver.py query generators
//   - Writes run logs to evidence/v312-59/soak/
// =============================================================================

use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};

// #5577：输出目录规则收敛到共享 helper（唯一实现）。
#[path = "../common/evidence_output.rs"]
mod evidence_output;

const DEMO_DURATION_SECS_DEFAULT: u64 = 60 * 60; // 1h demo (GA gate)

/// Duration of the GA-2 mixed-workload demo, in seconds.
///
/// 2026-11-16 (v4.1.0 168h SOAK, #5598/GA 路径): the 168h run needs the SAME
/// harness to execute **continuously for 168 hours** rather than a fixed 1h.
/// `SQLRUSTGO_SOAK_DURATION_SECS` follows the env-var convention this file
/// already uses (`SQLRUSTGO_SOAK_SUMMARY_PATH`), so the **default stays 1h**
/// and every existing caller is bit-for-bit unaffected.
///
/// A malformed / zero / negative value falls back to the default rather than
/// running a zero-length soak that would look like a completed run.
fn demo_duration_secs() -> u64 {
    match std::env::var("SQLRUSTGO_SOAK_DURATION_SECS") {
        Ok(raw) => raw
            .trim()
            .parse::<u64>()
            .ok()
            .filter(|secs| *secs > 0)
            .unwrap_or(DEMO_DURATION_SECS_DEFAULT),
        Err(_) => DEMO_DURATION_SECS_DEFAULT,
    }
}
const SMOKE_DURATION_SECS: u64 = 60; // 60s smoke (CI)
const WORKLOAD_FRACTION: [u32; 5] = [30, 25, 15, 10, 20];
const TARGET_OPS_PER_MIN: u32 = 600; // total across all 5 classes

/// 五类负载的定义。
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(usize)]
enum WorkloadClass {
    W1Oltp = 0,
    W2ReadHeavy = 1,
    W3Aggregate = 2,
    W4DdlLight = 3,
    W5LongReport = 4,
}

impl WorkloadClass {
    fn name(&self) -> &'static str {
        match self {
            WorkloadClass::W1Oltp => "W1-OLTP",
            WorkloadClass::W2ReadHeavy => "W2-ReadHeavy",
            WorkloadClass::W3Aggregate => "W3-Aggregate",
            WorkloadClass::W4DdlLight => "W4-DDL-Light",
            WorkloadClass::W5LongReport => "W5-LongReport",
        }
    }
}

/// 每类的指标。
#[derive(Debug, Default, Clone)]
struct ClassMetrics {
    ops_attempted: u64,
    ops_succeeded: u64,
    ops_failed: u64,
    total_latency_ms: u64,
    latency_samples: Vec<u64>,
    errors: Vec<String>,
}

impl ClassMetrics {
    fn avg_latency_ms(&self) -> f64 {
        if self.latency_samples.is_empty() {
            0.0
        } else {
            self.total_latency_ms as f64 / self.latency_samples.len() as f64
        }
    }

    fn p99_latency_ms(&self) -> u64 {
        if self.latency_samples.is_empty() {
            return 0;
        }
        let mut sorted = self.latency_samples.clone();
        sorted.sort_unstable();
        let idx = ((sorted.len() as f64 * 0.99) as usize).min(sorted.len() - 1);
        sorted[idx]
    }
}

/// 新建一个内存引擎。
fn new_engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

/// 执行一条 SQL，把错误变成字符串。
fn exec(engine: &mut ExecutionEngine<MemoryStorage>, sql: &str) -> Result<(), String> {
    engine
        .execute(sql)
        .map(|_| ())
        .map_err(|e| format!("`{}` 失败: {}", sql, e))
}

/// 一个 workload class 的驱动器（每个线程一个自己的引擎）。
struct WorkloadDriver {
    class: WorkloadClass,
    metrics: Arc<Mutex<ClassMetrics>>,
    engine: ExecutionEngine<MemoryStorage>,
    /// 该 class 已执行的操作序号 —— 用来在多条 SQL 之间轮转。
    op_seq: u64,
}

impl WorkloadDriver {
    fn new(class: WorkloadClass) -> Self {
        let mut engine = new_engine();
        // 建表 + 灌种子数据。建表失败说明引擎连最基本的能力都没有，
        // 那时必须让整个 harness 立刻失败，而不是把错误吞进 metrics。
        let setup: Vec<String> = match class {
            WorkloadClass::W1Oltp => {
                let mut s = vec!["CREATE TABLE oltp(id INT, v INT, note TEXT)".to_string()];
                let values: Vec<String> = (0..200)
                    .map(|i| format!("({}, {}, 'row{}')", i, i % 97, i))
                    .collect();
                s.push(format!("INSERT INTO oltp VALUES {}", values.join(", ")));
                s
            }
            WorkloadClass::W2ReadHeavy => {
                let mut s = vec!["CREATE TABLE rh(id INT, v INT)".to_string()];
                let values: Vec<String> =
                    (0..1000).map(|i| format!("({}, {})", i, i % 53)).collect();
                s.push(format!("INSERT INTO rh VALUES {}", values.join(", ")));
                s
            }
            WorkloadClass::W3Aggregate => {
                let mut s = vec!["CREATE TABLE ag(g INT, v INT)".to_string()];
                let values: Vec<String> =
                    (0..1000).map(|i| format!("({}, {})", i % 13, i)).collect();
                s.push(format!("INSERT INTO ag VALUES {}", values.join(", ")));
                s
            }
            // W4 不预建表：它的操作循环本身就是 CREATE → ALTER → DROP，
            // 预建会让第一个 CREATE 撞上 "already exists"。
            WorkloadClass::W4DdlLight => vec![],
            WorkloadClass::W5LongReport => {
                let mut s = vec![
                    "CREATE TABLE r1(id INT, v INT)".to_string(),
                    "CREATE TABLE r2(id INT, w INT)".to_string(),
                ];
                let v1: Vec<String> = (0..800).map(|i| format!("({}, {})", i, i % 71)).collect();
                let v2: Vec<String> = (0..800).map(|i| format!("({}, {})", i, i % 29)).collect();
                s.push(format!("INSERT INTO r1 VALUES {}", v1.join(", ")));
                s.push(format!("INSERT INTO r2 VALUES {}", v2.join(", ")));
                s
            }
        };
        for sql in setup {
            if let Err(e) = exec(&mut engine, &sql) {
                panic!("{} 的准备语句失败: {}", class.name(), e);
            }
        }

        Self {
            class,
            metrics: Arc::new(Mutex::new(ClassMetrics::default())),
            engine,
            op_seq: 0,
        }
    }

    /// 执行**一个**真实操作。
    ///
    /// 返回 `Ok(延迟毫秒)` 或 `Err(错误信息)`。错误来自引擎的真实报错 ——
    /// 不再是模拟出来的。
    fn execute_one(&mut self) -> Result<u64, String> {
        let start = Instant::now();
        let seq = self.op_seq;
        self.op_seq = self.op_seq.wrapping_add(1);

        match self.class {
            WorkloadClass::W1Oltp => {
                let id = (seq % 400) as i64;
                let sql = match seq % 4 {
                    0 => format!("INSERT INTO oltp VALUES ({}, {}, 'x')", 1000 + id, seq % 97),
                    1 => format!("UPDATE oltp SET v = v + 1 WHERE id = {}", id % 200),
                    2 => format!("DELETE FROM oltp WHERE id = {}", 1000 + id),
                    _ => format!("SELECT COUNT(*) FROM oltp WHERE v > {}", id % 13),
                };
                exec(&mut self.engine, &sql)?;
            }
            WorkloadClass::W2ReadHeavy => {
                let id = (seq % 1000) as i64;
                let sql = match seq % 2 {
                    0 => format!("SELECT v FROM rh WHERE id = {}", id),
                    _ => format!(
                        "SELECT COUNT(*) FROM rh WHERE v BETWEEN {} AND {}",
                        id % 7,
                        id % 7 + 20
                    ),
                };
                exec(&mut self.engine, &sql)?;
            }
            WorkloadClass::W3Aggregate => {
                let sql = match seq % 3 {
                    0 => "SELECT g, COUNT(*), SUM(v), AVG(v) FROM ag GROUP BY g".to_string(),
                    1 => "SELECT COUNT(*), MIN(v), MAX(v) FROM ag".to_string(),
                    _ => "SELECT g, SUM(v) FROM ag GROUP BY g HAVING SUM(v) > 100".to_string(),
                };
                exec(&mut self.engine, &sql)?;
            }
            WorkloadClass::W4DdlLight => {
                // 严格的三步循环：建表 → 加列 → 删表。任何一步失败都是真实缺陷。
                let sql = match seq % 3 {
                    0 => "CREATE TABLE ddl_t(id INT)".to_string(),
                    1 => "ALTER TABLE ddl_t ADD COLUMN v INT".to_string(),
                    _ => "DROP TABLE ddl_t".to_string(),
                };
                exec(&mut self.engine, &sql)?;
            }
            WorkloadClass::W5LongReport => {
                let sql = match seq % 3 {
                    0 => "SELECT COUNT(*) FROM r1 JOIN r2 ON r1.id = r2.id WHERE r1.v > 10"
                        .to_string(),
                    1 => "SELECT COUNT(*) FROM r1 WHERE v > (SELECT AVG(v) FROM r1)".to_string(),
                    _ => "SELECT g, SUM(v) FROM (SELECT id % 10 AS g, v FROM r1) t GROUP BY g"
                        .to_string(),
                };
                exec(&mut self.engine, &sql)?;
            }
        }

        Ok(start.elapsed().as_millis() as u64)
    }

    fn run_loop(&mut self, stop_at: Instant, ops_per_min_target: u32) {
        let ops_per_sec = ops_per_min_target.div_ceil(60);
        let sleep_between = Duration::from_micros((1_000_000 / ops_per_sec.max(1)) as u64);
        while Instant::now() < stop_at {
            let result = self.execute_one();
            let mut m = self.metrics.lock().unwrap();
            m.ops_attempted += 1;
            match result {
                Ok(latency_ms) => {
                    m.ops_succeeded += 1;
                    m.total_latency_ms += latency_ms;
                    m.latency_samples.push(latency_ms);
                }
                Err(err) => {
                    m.ops_failed += 1;
                    if m.errors.len() < 10 {
                        m.errors.push(err);
                    }
                }
            }
            drop(m);
            thread::sleep(sleep_between);
        }
    }
}

/// 跑一轮混合负载，返回每类的指标与总账。
fn run_mixed(
    duration_secs: u64,
    ops_per_min_total: u32,
) -> (Vec<Arc<Mutex<ClassMetrics>>>, u64, u64, u64) {
    let stop_at = Instant::now() + Duration::from_secs(duration_secs);

    let classes = [
        WorkloadClass::W1Oltp,
        WorkloadClass::W2ReadHeavy,
        WorkloadClass::W3Aggregate,
        WorkloadClass::W4DdlLight,
        WorkloadClass::W5LongReport,
    ];

    let mut handles = Vec::new();
    let mut metric_arcs: Vec<Arc<Mutex<ClassMetrics>>> = Vec::new();
    for (i, class) in classes.iter().enumerate() {
        let class_ops_per_min = (ops_per_min_total * WORKLOAD_FRACTION[i]) / 100;
        let mut driver = WorkloadDriver::new(*class);
        metric_arcs.push(driver.metrics.clone());
        handles.push(thread::spawn(move || {
            driver.run_loop(stop_at, class_ops_per_min);
        }));
    }
    for h in handles {
        h.join().expect("workload thread panicked");
    }

    let mut total_attempted = 0u64;
    let mut total_succeeded = 0u64;
    let mut total_failed = 0u64;
    for m in metric_arcs.iter() {
        let m = m.lock().unwrap();
        total_attempted += m.ops_attempted;
        total_succeeded += m.ops_succeeded;
        total_failed += m.ops_failed;
    }
    (metric_arcs, total_attempted, total_succeeded, total_failed)
}

/// 打印每类与总账。
fn print_summary(metric_arcs: &[Arc<Mutex<ClassMetrics>>]) {
    let classes = [
        WorkloadClass::W1Oltp,
        WorkloadClass::W2ReadHeavy,
        WorkloadClass::W3Aggregate,
        WorkloadClass::W4DdlLight,
        WorkloadClass::W5LongReport,
    ];
    eprintln!("╔══════════════════════════════════════════════════════════════════╗");
    eprintln!("║                       SOAK Demo Summary                          ║");
    eprintln!("╠══════════════════════════════════════════════════════════════════╣");
    for (i, m) in metric_arcs.iter().enumerate() {
        let m = m.lock().unwrap();
        eprintln!(
            "║ {:<14} attempts={:>6} ok={:>6} fail={:>4} avg={:>5.1}ms p99={:>4}ms ║",
            classes[i].name(),
            m.ops_attempted,
            m.ops_succeeded,
            m.ops_failed,
            m.avg_latency_ms(),
            m.p99_latency_ms(),
        );
        for e in m.errors.iter() {
            eprintln!("║   ! {}", e);
        }
    }
    eprintln!("╚══════════════════════════════════════════════════════════════════╝");
}

#[test]
#[ignore = "runs 1h demo; nightly-only via scripts/soak/run_ga_soak.sh"]
fn v312_ga_mixed_workload_demo() {
    eprintln!("╔══════════════════════════════════════════════════════════════════╗");
    eprintln!("║ v3.12.0 GA Mixed-Workload SOAK Demo (Issue #4387 / GA-2)         ║");
    eprintln!("║  Duration: 1h demo (background 168h run separate)                ║");
    eprintln!("║  Classes: W1-OLTP / W2-ReadHeavy / W3-Aggregate / W4-DDL / W5-Rep ║");
    eprintln!("╚══════════════════════════════════════════════════════════════════╝");

    let demo_secs = demo_duration_secs();
    let (metric_arcs, total_attempted, total_succeeded, total_failed) =
        run_mixed(demo_secs, TARGET_OPS_PER_MIN);

    print_summary(&metric_arcs);

    // 写 summary 到 **构建产物目录**。
    //
    // #5491: the output path is overridable so that a runner can keep the
    // artifact OUT of the repository tree. Writing a generated file into
    // `docs/` from a test is the same shape as #5577 (a test overwrites
    // committed evidence): the working tree becomes dirty after every run,
    // and the file that lands there says nothing about the run that
    // produced it — it is simply whatever the last run left behind.
    //
    // #5577: the **default** used to be
    // `docs/releases/v3.12.0/evidence/v312-59/soak/…` — i.e. the
    // overridable knob existed but the default still pointed **into the
    // repository**, so the intent ("keep the artifact OUT of the tree")
    // was not what the code did. The default is now the build-output
    // directory; writing into the repository requires the explicit
    // `SQLRUSTGO_SOAK_SUMMARY_PATH`.
    //
    // The rule itself lives in `tests/common/evidence_output.rs`
    // (**one implementation**, #5577).
    let summary_path = evidence_output::evidence_file(
        "SQLRUSTGO_SOAK_SUMMARY_PATH",
        "soak/v312_mixed_soak_summary.txt",
    )
    .to_string_lossy()
    .to_string();
    fs::create_dir_all(
        std::path::Path::new(summary_path.as_str())
            .parent()
            .unwrap(),
    )
    .ok();

    let totals = SoakTotals::from_counters(total_attempted, total_succeeded, total_failed);
    write_summary(
        &summary_path,
        &metric_arcs,
        totals,
        totals.verdict(),
        demo_secs,
    );

    // GA gate: 每个 class 都必须真的跑到 SQL，失败率必须低于 1%。
    assert!(total_attempted > 0, "no operations attempted");
    assert!(
        totals.failure_rate < 0.01,
        "GA failure rate {:.3}% exceeds 1% threshold ({} fails / {} total)",
        totals.failure_rate * 100.0,
        totals.failed,
        totals.attempted
    );
    let _ = total_succeeded;
}

/// 2026-11-16 (v4.1.0 168h SOAK): the duration override must be exactly what
/// the operator asked for, and anything malformed must fall back to the 1h
/// default rather than running a zero-length "completed" soak.
#[test]
fn demo_duration_env_override_behaviour() {
    // The default (no env) is the historical 1h GA demo.
    assert_eq!(demo_duration_secs(), DEMO_DURATION_SECS_DEFAULT);

    // A valid override is honoured verbatim.
    std::env::set_var("SQLRUSTGO_SOAK_DURATION_SECS", "604800");
    assert_eq!(demo_duration_secs(), 604_800);

    // Malformed / zero / negative values fall back to the default — a
    // zero-length soak would look like a completed run.
    for bad in ["", "   ", "abc", "0", "-1"] {
        std::env::set_var("SQLRUSTGO_SOAK_DURATION_SECS", bad);
        assert_eq!(
            demo_duration_secs(),
            DEMO_DURATION_SECS_DEFAULT,
            "malformed duration {bad:?} must fall back to the default"
        );
    }
    std::env::remove_var("SQLRUSTGO_SOAK_DURATION_SECS");
}

/// The current UTC time as ISO-8601, e.g. `2026-10-24T12:00:00Z`.
///
/// #5491: the summary used to carry `generated_at: epoch=1758600000`, which
/// is not a timestamp a reader — or a validator — can place in time. The
/// GA-2 artifact validator (`scripts/gate/check_ga2_soak_artifact.sh`)
/// judges freshness by the artifact's own ISO-8601 timestamp, so the
/// artifact has to carry one.
///
/// The conversion is the standard civil-from-days algorithm (Howard
/// Hinnant's); it needs no calendar crate.
fn iso_utc_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);

    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 }.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, h, mi, s)
}

/// The run's totals, in one place.
///
/// #5491: the artifact has to carry the totals, the failure rate and the
/// verdict — and the artifact writer has to take them as one value rather
/// than as four loose numbers, so that the numbers in the file cannot drift
/// apart from each other.
#[derive(Clone, Copy)]
struct SoakTotals {
    attempted: u64,
    succeeded: u64,
    failed: u64,
    failure_rate: f64,
}

impl SoakTotals {
    /// Build the totals and the verdict from the raw counters.
    ///
    /// The verdict is computed BEFORE the artifact is written, so the
    /// artifact records what actually happened — a failed run leaves behind
    /// a file that says so, instead of one that looks fine because it was
    /// written before the assertion fired.
    fn from_counters(attempted: u64, succeeded: u64, failed: u64) -> Self {
        let failure_rate = if attempted > 0 {
            failed as f64 / attempted as f64
        } else {
            1.0
        };
        SoakTotals {
            attempted,
            succeeded,
            failed,
            failure_rate,
        }
    }

    fn verdict(&self) -> &'static str {
        if self.attempted > 0 && self.failure_rate < 0.01 {
            "PASS"
        } else {
            "FAIL"
        }
    }
}

/// Write the GA-2 SOAK artifact.
///
/// #5491: the artifact carries everything the GA-2 validator judges — an
/// ISO-8601 timestamp, all five workload classes, the failure rate and the
/// run's verdict — so that "did a SOAK really run, and did it stay healthy"
/// can be answered from the artifact instead of from a file's existence.
fn write_summary(
    summary_path: &str,
    metric_arcs: &[Arc<Mutex<ClassMetrics>>],
    totals: SoakTotals,
    verdict: &str,
    duration_secs: u64,
) {
    let classes = [
        WorkloadClass::W1Oltp,
        WorkloadClass::W2ReadHeavy,
        WorkloadClass::W3Aggregate,
        WorkloadClass::W4DdlLight,
        WorkloadClass::W5LongReport,
    ];
    if let Some(parent) = std::path::Path::new(summary_path).parent() {
        fs::create_dir_all(parent).ok();
    }
    let mut summary = String::new();
    summary.push_str(&format!(
        "v3.12.0 GA Mixed-Workload SOAK Summary\n\
         ===========================================\n\
         duration: {} secs\n\
         generated_at: {}\n\
         workload_classes: 5 (W1-OLTP W2-ReadHeavy W3-Aggregate W4-DDL W5-Report)\n\
         result: {}\n\
         total: attempted={} succeeded={} failed={}\n\
         failure_rate: {:.4} (threshold 0.0100)\n\n",
        duration_secs,
        iso_utc_now(),
        verdict,
        totals.attempted,
        totals.succeeded,
        totals.failed,
        totals.failure_rate,
    ));
    for (i, m) in metric_arcs.iter().enumerate() {
        let m = m.lock().unwrap();
        summary.push_str(&format!(
            "{}: attempted={} succeeded={} failed={} avg_latency_ms={:.1} p99_latency_ms={}\n",
            classes[i].name(),
            m.ops_attempted,
            m.ops_succeeded,
            m.ops_failed,
            m.avg_latency_ms(),
            m.p99_latency_ms(),
        ));
    }
    fs::write(summary_path, &summary).expect("write summary");
}

/// 轻量 "smoke" 版本：60 秒。CI 里不带 --ignored 跑。
///
/// 它现在**真的执行 SQL**，所以它的断言是有判别力的：
/// 五类负载都必须跑到 SQL，且一条都不能失败。
#[test]
fn v312_ga_mixed_workload_smoke_60s() {
    eprintln!(
        "[SMOKE] v3.12.0 mixed-workload smoke ({}s, 真实 SQL)",
        SMOKE_DURATION_SECS
    );

    let (metric_arcs, total_attempted, total_succeeded, total_failed) =
        run_mixed(SMOKE_DURATION_SECS, 120);

    print_summary(&metric_arcs);

    assert!(
        total_attempted > 0,
        "smoke 一次 SQL 都没执行 —— harness 没有真的驱动引擎"
    );
    assert_eq!(
        total_succeeded, total_attempted,
        "smoke 期间有 {} 条真实 SQL 失败（out of {}）",
        total_failed, total_attempted
    );

    // #5491: the smoke writes an artifact **only when a caller asked for
    // one** (SQLRUSTGO_SOAK_SUMMARY_PATH is set). A bare `cargo test --test
    // v312_mixed_soak` must not drop a generated file into `docs/` — that
    // is the shape of #5577, where a test overwrites committed evidence and
    // leaves the working tree dirty after every run.
    if let Ok(summary_path) = std::env::var("SQLRUSTGO_SOAK_SUMMARY_PATH") {
        let totals = SoakTotals::from_counters(total_attempted, total_succeeded, total_failed);
        write_summary(
            &summary_path,
            &metric_arcs,
            totals,
            totals.verdict(),
            SMOKE_DURATION_SECS,
        );
    }
}

/// The GA-2 harness's **preconditions** are present.
///
/// #5491: this test is a precondition check, NOT the GA-2 criterion. It
/// answers "are the driver and its config in the tree?" — it says nothing
/// about whether a SOAK ran, and it must not be read as GA-2 evidence.
///
/// The GA-2 criterion is `scripts/gate/check_ga2_soak_artifact.sh`, which
/// judges the SOAK **artifact**: a completed run, all five workload classes,
/// failure rates within threshold, and a fresh timestamp.
#[test]
fn v312_ga_mixed_workload_scaffold_present() {
    let must_exist = [
        "tests/soak/v312_mixed_soak.rs",
        "tests/soak/mixed_workload.py",
        "tests/soak/mixed_workload_config.yaml",
    ];
    for path in must_exist {
        assert!(
            std::path::Path::new(path).exists(),
            "GA-2 harness precondition missing: {}",
            path
        );
    }
}
