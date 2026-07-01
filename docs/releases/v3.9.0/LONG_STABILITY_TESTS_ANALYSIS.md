<!-- 2026-07-01 status addendum (auto-applied) -->
> **状态更新**: 本机 L1 lint + 架构整理已闭环。HEAD `d77821f6d1`, 3 个 PR 已合并 (PR #3664, #3665, #3666)。
> - `src/execution_engine.rs` 1471 行 (AD-001 1500 目标达标, 2630 → 1471)
> - C-ARCH-05 上限锁回 1500 (从 3000/1800 统一)
> - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
> - Open issues (4, 全部硬件阻塞, 本机无法推进):
>   - #3648 TPC-H 混合负载 SOAK 跨平台验证 (需要 Z6G4/Z440)
>   - #3423 TPC-H SF=1.0 baseline (需要 75GB+ 磁盘, Mac mini 仅 1GB)
>   - #3265 72h 长跑 SOAK (blocked-on-S1, 需 72+ 小时持续运行)
>   - #3266 168h 长跑 SOAK (blocked-on-S1, 需 168 小时持续运行)
> - 详见: issue #3667 (closed as state snapshot) + CHANGELOG.md
>
> 本文件原始内容保持不变,仅顶部加 addendum。

---

# Long Stability Tests Analysis (Track C, v3.9.0-rc7)

> Generated: 2026-06-17
> Purpose: Document the 14 long-running wall-clock tests that need
> real-hardware (Z6G4) validation per GA-P0 Stability milestone.

## Background

Per #3225, #3265, #3266, #3229: 24h/72h/168h real wall-clock soak is a
GA-final blocker. The simulated `tests/soak_test.rs:73-87` harness
(1,440× compression) is no substitute for real runs.

The 14 long-running tests below are NOT the 14 in `tests/soak_test.rs`.
They are the tests that, in their current form, take > 30s per run and
require:

- `cargo test --release -- --ignored` to invoke
- dedicated CI lane or local developer time
- in some cases, real wall-clock environments (not Z6G4 local hardware)

## Categories

### 1. Long Stability (1 test)

| Test | File | Reason | Action |
|------|------|--------|--------|
| `long_run_stability_72h_smoke` | `tests/long_run_stability_72h_test.rs:5` | Real 72h wall-clock (compressed 5s smoke exists but ignored by default) | KEEP IGNORED — needs Z6G4 |

### 2. QPS Benchmarks (10 tests)

All in `tests/qps_benchmark_test.rs`. 10000 iterations × 8 threads.
Total wall-clock ~minutes per test.

| Test | Line | Reason | Action |
|------|------|--------|--------|
| 10 QPS tests | 69, 94, 123, 152, 184, 212, 237, 284, 348, 374 | "Performance benchmark (long runtime, run with --ignored, dedicated test env)" | KEEP IGNORED — too slow for default CI |

### 3. Batched Insert Perf (3 tests)

`tests/perf_eng_batched_insert_test.rs`. 1000-row + 10000-row INSERT
through wire protocol. Several seconds per test in release mode.

| Test | Line | Reason | Action |
|------|------|--------|--------|
| 3 batched tests | 55, 94, +1 | "Performance benchmark (long runtime)" | KEEP IGNORED |

### 4. v3.8.0 Perf Benchmarks (6 tests)

`tests/bench_v380_point_agg.rs`. Performance baselines.

| Test | Line | Reason | Action |
|------|------|--------|--------|
| 6 v380 perf tests | 40, 90, 136, 183, 231, 274 | "Performance baseline (long runtime)" | KEEP IGNORED |

### 5. tx_wal Contract Tests (6 of 20 marked `#[ignore]`)

`tests/tx_wal_contract_tests.rs`. Sprint 3 decision: tests expect
`Err` for INSERT/UPDATE/DELETE without BEGIN, but MySQL AUTOCOMMIT=ON
allows them. Issue #2870 follow-up tracks reconciliation.

| Test | Line | Reason (inline) |
|------|------|-----------------|
| 6 tx_wal tests | 30, 53, 75, 102, 137, 166 | "Sprint 3 decision: ignored. Path A contradicts MySQL AUTOCOMMIT=ON..." |

The other 14 in tx_wal_contract_tests.rs are NOT `#[ignore]` — they
are part of the default suite. The 6 above are the deferred ones.

## Total

| Category | Count | All have `#[ignore = "..."]` reason? |
|----------|-------|--------------------------------------|
| Long stability | 1 | Yes (`#[ignore]` only, with file header doc) |
| QPS benchmarks | 10 | Yes (uniform inline reason) |
| Batched insert perf | 3 | Module docstring explains `#[ignore]` |
| v3.8.0 perf | 6 | Each has `#[ignore = "..."]` reason |
| tx_wal contract | 6 | Detailed Sprint 3 decision reason |

**Subtotal of long-running tests**: ~26 (not exactly 14 — the
"14 long stability tests" wording in #3225 is approximate).

## Verification Steps for Z6G4 / Real Wall-Clock Run

```bash
# Step 1: Build release binary
cargo build --release -p sqlrustgo-mysql-server

# Step 2: Smoke test (36s) — verify soak runner works
./target/release/sqlrustgo-mysql-server soak \
    --duration 0.01 --qps 1 \
    --output /tmp/soak_smoke.jsonl \
    --sample-interval-s 5

# Step 3: Real 24h run
nohup ./target/release/sqlrustgo-mysql-server soak \
    --duration 24 --qps 1 \
    --output /var/log/sqlrustgo/soak_24h.jsonl \
    --sample-interval-s 60 \
    > /var/log/sqlrustgo/soak_24h.log 2>&1 &

# Step 4: After 24h, verify
cat /var/log/sqlrustgo/SOAK_24H_REPORT.md
# RSS growth < 50MB, FD stable, 0 crashes

# Step 5: Run all long-running tests once
cargo test --release --all-features -- \
    --ignored --test-threads=1 \
    long_run_stability_72h_smoke qps_ benchmark_v380_point_agg \
    perf_eng_batched_insert tx_wal_contract
```

## Acceptance Criteria

Per #3225:
- [x] `soak_runner` binary implemented (Track C de8b6b2fd)
- [ ] 24h real wall-clock — REQUIRES Z6G4
- [ ] 72h real wall-clock — REQUIRES Z6G4
- [ ] 14 long stability tests analyzed and registered — THIS DOC
- [ ] 0 crashes during real run — REQUIRES Z6G4

## Related Files

- `crates/mysql-server/src/main.rs` — soak subcommand
- `crates/mysql-server/Cargo.toml` — signal-hook + libc deps
- `tests/baseline/ignore_registry.json` — Track B registry (42 entries + 1 marker)
- `tests/soak_test.rs` — existing SIMULATED harness (1,440× compressed)
- `scripts/gate/check_ignore_count.sh` — P12 detector validates registry

## Sign-off

- Author: Track C implementation 2026-06-17
- Owner: Future Z6G4 hardware owner
- Tracker: #3225, #3265, #3266, #3229
