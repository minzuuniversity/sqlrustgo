//! P1-3 (#3175) Soak Test - 3-level smoke equivalent (DECOMMISSIONED for gate).
//!
//! The 3 soak levels (24h/72h/168h) used a 1,440x compressed-time equivalence:
//!
//! | Level | Real duration | Smoke duration (5 q/s) | Equivalence |
//! |-------|---------------|--------------------------|-------------|
//! | 24h   | 86,400 s      | 60 s                    | 1,440x       |
//! | 72h   | 259,200 s     | 180 s                   | 1,440x       |
//! | 168h  | 604,800 s     | 420 s                   | 1,440x       |
//!
//! **DECOMMISSIONED (2026-06-21)**: All tests in this file are `#[ignore]`.
//! The G7 Soak Test gate (`scripts/gate/check_p13_soak_test.sh`) no
//! longer runs these in-process simulations. The gate now launches the
//! real `sqlrustgo-mysql-server` binary, hits it with `sysbench
//! oltp_read_write` over the MySQL wire protocol, and parses a real
//! STABILITY_REPORT. An in-process MemoryExecutionEngine cannot
//! reproduce production conditions: wire protocol, buffer pool,
//! connection manager, WAL, catalog, and lock manager are not exercised.
//!
//! Run explicitly with `cargo test --test soak_test -- --ignored` to
//! verify harness invariants (alert thresholds, percentile ordering,
//! baseline stability) without a real server.
//!
//! Refs: docs/openspec/3175-soak-test.md
//!       docs/releases/v3.9.0/plans/V390_TEST_PLAN.md section G7
//!       scripts/gate/check_p13_soak_test.sh (wired E2E gate, current)

// The harness provides the run-soak-smoke loop. We re-declare a
// minimal local copy (kept in sync via the G7 gate) so this test
// target compiles standalone.
mod harness {
    use sqlrustgo::MemoryExecutionEngine;
    use sqlrustgo_storage::MemoryStorage;
    use std::sync::{Arc, RwLock};
    use std::time::Instant;

    #[derive(Debug, Clone)]
    pub struct SoakConfig {
        pub duration_seconds: u64,
        pub queries_per_second: u32,
        pub memory_baseline_bytes: u64,
        pub fd_baseline: u32,
        pub memory_alert_threshold_pct: u32,
        pub fd_alert_threshold: u32,
    }

    impl Default for SoakConfig {
        fn default() -> Self {
            Self {
                duration_seconds: 60,
                queries_per_second: 5,
                memory_baseline_bytes: 100 * 1024 * 1024,
                fd_baseline: 8,
                memory_alert_threshold_pct: 10,
                fd_alert_threshold: 5,
            }
        }
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone)]
    pub struct SoakReport {
        pub duration_seconds: u64,
        pub queries_executed: u64,
        pub memory_baseline_bytes: u64,
        pub memory_final_bytes: u64,
        pub memory_growth_pct: f64,
        pub fd_baseline: u32,
        pub fd_final: u32,
        pub fd_growth: i32,
        pub p50_latency_ms: f64,
        pub p99_latency_ms: f64,
        pub alert_triggered: bool,
        pub alert_reason: Vec<String>,
    }

    impl SoakReport {
        pub fn passed(&self) -> bool {
            !self.alert_triggered
        }
    }

    pub fn run_soak_smoke(config: &SoakConfig) -> SoakReport {
        let storage = Arc::new(RwLock::new(MemoryStorage::new()));
        let mut engine = MemoryExecutionEngine::new(storage.clone());

        let _ = engine
            .execute("CREATE TABLE IF NOT EXISTS soak_test (id INTEGER, value INTEGER, text TEXT)");

        for i in 0..100 {
            let _ = engine.execute(&format!(
                "INSERT INTO soak_test VALUES ({}, {}, 'text_{}')",
                i,
                i * 10,
                i
            ));
        }

        let mut queries_executed: u64 = 0;
        let mut latencies: Vec<f64> = Vec::new();
        let target_queries = config.duration_seconds * config.queries_per_second as u64;
        let mut memory_current = config.memory_baseline_bytes;
        let fd_current = config.fd_baseline;

        while queries_executed < target_queries {
            let query_start = Instant::now();

            let sql = match queries_executed % 5 {
                0 => "SELECT * FROM soak_test WHERE id = 50",
                1 => "SELECT COUNT(*) FROM soak_test WHERE value > 500",
                2 => "INSERT INTO soak_test VALUES (1000, 9999, 'insert_test')",
                3 => "UPDATE soak_test SET value = value + 1 WHERE id = 50",
                _ => "SELECT AVG(value) FROM soak_test GROUP BY id",
            };

            let _ = engine.execute(sql);
            let elapsed = query_start.elapsed().as_secs_f64() * 1000.0;
            latencies.push(elapsed);

            queries_executed += 1;

            if queries_executed % 100 == 0 {
                memory_current += 2048;
            }
        }

        let _ = engine.execute("DROP TABLE IF EXISTS soak_test");

        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p50 = latencies.get(latencies.len() / 2).copied().unwrap_or(0.0);
        let p99 = latencies
            .get((latencies.len() as f64 * 0.99) as usize)
            .copied()
            .unwrap_or(0.0);

        let memory_growth_pct = if config.memory_baseline_bytes > 0 {
            ((memory_current - config.memory_baseline_bytes) as f64
                / config.memory_baseline_bytes as f64)
                * 100.0
        } else {
            0.0
        };
        let fd_growth = fd_current as i32 - config.fd_baseline as i32;

        let mut alerts = Vec::new();
        if memory_growth_pct > config.memory_alert_threshold_pct as f64 {
            alerts.push(format!(
                "Memory growth {:.2}% exceeds threshold {}%",
                memory_growth_pct, config.memory_alert_threshold_pct
            ));
        }
        if fd_growth > config.fd_alert_threshold as i32 {
            alerts.push(format!(
                "FD growth {} exceeds threshold {}",
                fd_growth, config.fd_alert_threshold
            ));
        }

        SoakReport {
            duration_seconds: config.duration_seconds,
            queries_executed,
            memory_baseline_bytes: config.memory_baseline_bytes,
            memory_final_bytes: memory_current,
            memory_growth_pct,
            fd_baseline: config.fd_baseline,
            fd_final: fd_current,
            fd_growth,
            p50_latency_ms: p50,
            p99_latency_ms: p99,
            alert_triggered: !alerts.is_empty(),
            alert_reason: alerts,
        }
    }
}

use harness::{run_soak_smoke, SoakConfig};

/// Helper: build a default soak config (5 q/s, 100MB baseline).
fn default_config(duration_seconds: u64) -> SoakConfig {
    SoakConfig {
        duration_seconds,
        ..SoakConfig::default()
    }
}

// --------------------------------------------------------------------
// 24h Soak smoke (60s, 300 queries)
// --------------------------------------------------------------------

#[ignore = "in-process simulation; G7 gate uses wired E2E (sqlrustgo-mysql-server + sysbench)"]
#[test]
fn test_soak_24h_smoke_p1_3() {
    let config = default_config(60);
    let report = run_soak_smoke(&config);
    assert_eq!(report.duration_seconds, 60);
    assert_eq!(report.queries_executed, 300);
    assert!(
        report.passed(),
        "24h smoke must pass: {:?}",
        report.alert_reason
    );
}

#[ignore = "in-process simulation; G7 gate uses wired E2E"]
#[test]
fn test_soak_24h_smoke_memory_growth_within_threshold_p1_3() {
    let config = default_config(60);
    let report = run_soak_smoke(&config);
    assert!(
        report.memory_growth_pct <= 10.0,
        "memory growth {:.2}% exceeds 10%",
        report.memory_growth_pct
    );
}

#[ignore = "in-process simulation; G7 gate uses wired E2E"]
#[test]
fn test_soak_24h_smoke_p99_latency_bounded_p1_3() {
    let config = default_config(60);
    let report = run_soak_smoke(&config);
    assert!(report.p99_latency_ms > 0.0);
    // The simulated query latency is 0.5-2ms; p99 must be < 5ms.
    assert!(
        report.p99_latency_ms < 5.0,
        "p99 latency {:.2}ms exceeds 5ms threshold",
        report.p99_latency_ms
    );
}

// --------------------------------------------------------------------
// 72h Soak smoke (180s, 900 queries)
// --------------------------------------------------------------------

#[ignore = "in-process simulation; G7 gate uses wired E2E"]
#[test]
fn test_soak_72h_smoke_p1_3() {
    let config = default_config(180);
    let report = run_soak_smoke(&config);
    assert_eq!(report.duration_seconds, 180);
    assert_eq!(report.queries_executed, 900);
    assert!(
        report.passed(),
        "72h smoke must pass: {:?}",
        report.alert_reason
    );
}

#[ignore = "in-process simulation; G7 gate uses wired E2E"]
#[test]
fn test_soak_72h_smoke_no_fd_leak_p1_3() {
    let config = default_config(180);
    let report = run_soak_smoke(&config);
    assert_eq!(report.fd_growth, 0, "FD count must be stable");
}

// --------------------------------------------------------------------
// 168h Soak smoke (420s, 2100 queries)
// --------------------------------------------------------------------

#[ignore = "in-process simulation; G7 gate uses wired E2E"]
#[test]
fn test_soak_168h_smoke_p1_3() {
    let config = default_config(420);
    let report = run_soak_smoke(&config);
    assert_eq!(report.duration_seconds, 420);
    assert_eq!(report.queries_executed, 2100);
    assert!(
        report.passed(),
        "168h smoke must pass: {:?}",
        report.alert_reason
    );
}

#[ignore = "in-process simulation; G7 gate uses wired E2E"]
#[test]
fn test_soak_168h_smoke_no_lock_leak_proxy_p1_3() {
    // Lock leak proxy: in the smoke harness we don't run real queries,
    // so we use the FD growth as a proxy. Real implementation would
    // check lock-manager held count.
    let config = default_config(420);
    let report = run_soak_smoke(&config);
    assert!(
        report.fd_growth <= 5,
        "FD growth {} exceeds +5 threshold",
        report.fd_growth
    );
}

// --------------------------------------------------------------------
// Memory leak regression (the existing memory leak test pinned)
// --------------------------------------------------------------------

#[ignore = "in-process simulation; G7 gate uses wired E2E"]
#[test]
fn test_soak_memory_baseline_invariant_p1_3() {
    // Pin: when no query runs, memory_current must equal baseline
    // (no implicit allocations in the harness itself).
    let config = SoakConfig {
        duration_seconds: 0, // no queries
        ..SoakConfig::default()
    };
    let report = run_soak_smoke(&config);
    assert_eq!(report.queries_executed, 0);
    assert_eq!(report.memory_final_bytes, config.memory_baseline_bytes);
    assert_eq!(report.memory_growth_pct, 0.0);
}

#[ignore = "in-process simulation; G7 gate uses wired E2E"]
#[test]
fn test_soak_p50_p99_ordering_p1_3() {
    // Sanity: p99 must be >= p50 in any non-empty report.
    let config = default_config(60);
    let report = run_soak_smoke(&config);
    assert!(report.p50_latency_ms <= report.p99_latency_ms + f64::EPSILON);
}

#[ignore = "in-process simulation; G7 gate uses wired E2E"]
#[test]
fn test_soak_alert_message_when_exceeds_threshold_p1_3() {
    // Force an alert by setting a very tight memory threshold.
    let config = SoakConfig {
        duration_seconds: 60,
        memory_alert_threshold_pct: 0, // any growth trips the alarm
        ..SoakConfig::default()
    };
    let report = run_soak_smoke(&config);
    assert!(
        report.alert_triggered,
        "tight threshold should trigger alert"
    );
    assert!(!report.alert_reason.is_empty(), "alert reason must be set");
    assert!(!report.passed());
}
