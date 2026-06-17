# Oracle Framework + 8 Gate 实施 Design (v3.9.0 GA Push)

> **Date**: 2026-06-18
> **Version**: v1.0 (initial design)
> **Author**: claude-macmini
> **Status**: APPROVED (user confirmed 2026-06-18)
> **Scope**: P15 Oracle Required (V4 fix) + #3231 TPC-H SHA-256 baseline

---

## 0. 背景

### 0.1 现状 (V4 漏洞)

`tests/baseline/oracle_baseline.json` (2026-06-16) 跟踪 P15 Oracle Required:

```json
{
  "total_correctness_gates": 8,
  "gates_without_oracle": [
    "scripts/gate/check_g12_sysbench.sh",
    "scripts/gate/check_g13_stability.sh",
    "scripts/gate/check_g14_real_crash.sh",
    "scripts/gate/check_g16_compatibility.sh",
    "scripts/gate/check_p14_upgrade_test.sh",
    "scripts/gate/check_p22_time_travel.sh",
    "scripts/gate/check_p23_hash_chain.sh",
    "scripts/gate/check_p34_parallel_executor.sh"
  ]
}
```

**问题**: 8 gates 形式 PASS，但**没有独立 oracle 对比**。V4 漏洞 (ADR-006)。

### 0.2 已存在 oracle 资源

- ✅ P15 detector: `scripts/gate/check_oracle_present.sh`
- ✅ TPC-H SF=0.01 expected: `tests/data/tpch-sf01/expected/Q*_sf01_baseline.json` (22 files)
- ✅ Q9 3-way: `tests/data/tpch-sf01/baseline/Q09_three_way.json`
- ✅ TPC-H 22/22 in-process: `tests/tpch_full_22_test.rs`
- ❌ SHA-256 baseline: 没有 (#3231)
- ❌ G2/G3/G5/G9/G11/G12/G15/G16 oracle tests: 没有

---

## 1. 设计目标

1. **实施 8 gate oracle 验证** (排除 G13 stability 需 Z6G4)
2. **生成 TPC-H 22/22 SHA-256 baseline** (#3231)
3. **更新 P15 baseline** 至 1 gate WITHOUT (G13 only)
4. **提供 `tests/oracle/` 通用框架** 给未来 gate 复用

---

## 2. 架构

### 2.1 通用框架 (`tests/oracle/mod.rs`)

```rust
// tests/oracle/mod.rs
//! Oracle Required (P15) — 通用 oracle 验证框架
//!
//! 提供:
//! - run_sqlite_oracle(sql, fixtures) → RowSet
//! - assert_row_set_eq(engine_rows, oracle_rows) → bool
//! - sha256_capture(query_results) → String
//! - compare_to_baseline(actual, baseline_path) → DiffReport
//! - assert_perf_within(actual, baseline, tolerance_pct) → bool

pub mod sqlite_runner;
pub mod row_set;
pub mod sha256;
pub mod baseline;
pub mod perf_assert;

pub use row_set::{RowSet, Row, Value};
pub use sqlite_runner::run_sqlite_oracle;
pub use sha256::sha256_capture;
pub use baseline::{compare_to_baseline, DiffReport};
pub use perf_assert::assert_perf_within;
```

### 2.2 Gate 映射

| Gate | Oracle 实施 | 文件 |
|------|------------|------|
| **G1** | SHA-256 baseline (in-process 22/22 hash) | `tests/oracle/g1_tpch_sha256.rs` |
| **G2** | INT-2 row count vs expected | `tests/oracle/g2_int2.rs` |
| **G3** | INT-3 delegation output | `tests/oracle/g3_int3.rs` |
| **G5** | Savepoint MVCC state | `tests/oracle/g5_sem1.rs` |
| **G9** | v3.8→v3.9 file format compat | `tests/oracle/g9_upgrade.rs` |
| **G11** | QPS within baseline tolerance | `tests/oracle/g11_qps.rs` |
| **G12** | Sysbench within baseline tolerance | `tests/oracle/g12_sysbench.rs` |
| **G15** | SF=0.01 wire vs `expected/Q*.json` | `tests/oracle/g15_tpch_sf01.rs` |
| **G16** | File format compat suite | `tests/oracle/g16_compat.rs` |

---

## 3. 实施策略

### 3.1 3 PR 拆分

| PR | 内容 | 估计 | 依赖 |
|----|------|------|------|
| **PR-A** | Framework + G1 SHA-256 + P15 baseline | 4h | 无 |
| **PR-B** | G2/G3/G5/G9 in-process | 8h | PR-A (framework) |
| **PR-C** | G11/G12/G15/G16 baseline | 8h | PR-A (framework) |

### 3.2 风险

| 风险 | 缓解 |
|------|------|
| SQLite binary 不在 PATH | fallback 到 in-process MemoryStorage baseline |
| Q9 cell_diff (#3312) 阻塞 G1 | SHA-256 row-level, 不受 cell-level bug 影响 |
| 性能 baseline 漂移 | ±20% tolerance, warning 阈值 |
| Gate scripts 已存在但无 oracle | 在 gate script 内加 oracle check 调用 |

---

## 4. 不在范围

- Real 24h/72h/168h soak (需 Z6G4)
- G13 stability oracle (需 Z6G4)
- G14 real crash oracle (partial, 不完整)
- V1 check() exit code 修复 (meta-gate 扩展)
- V7 gate self-test (P11 增强)
- SHA-256 漂移检测 (post-GA)

---

## 5. 关联文档

- `docs/releases/v3.9.0/V390_COMPREHENSIVE_ASSESSMENT.md` v2.0 §15.1
- `docs/governance/adr/ADR-006-meta-governance.md` (P15)
- `docs/audit/status/2026-06-06-test-authenticity-analysis-v390.md`
- `scripts/gate/check_oracle_present.sh` (P15 detector)
- `tests/baseline/oracle_baseline.json`

---

## 6. 验收标准

- [ ] `bash scripts/gate/check_oracle_present.sh` → PASS (1 gate WITHOUT, G13)
- [ ] `cargo test --release --test oracle_g1_tpch_sha256 --all-features` → PASS
- [ ] `cargo test --release --test oracle_g2_int2 --all-features` → PASS
- [ ] `cargo test --release --test oracle_g3_int3 --all-features` → PASS
- [ ] `cargo test --release --test oracle_g5_sem1 --all-features` → PASS
- [ ] `cargo test --release --test oracle_g9_upgrade --all-features` → PASS
- [ ] `cargo test --release --test oracle_g11_qps --all-features` → PASS
- [ ] `cargo test --release --test oracle_g12_sysbench --all-features` → PASS
- [ ] `cargo test --release --test oracle_g15_tpch_sf01 --all-features` → PASS
- [ ] `cargo test --release --test oracle_g16_compat --all-features` → PASS
- [ ] `tests/baseline/oracle_baseline.json` updated (8 → 1 gates WITHOUT)
- [ ] `tests/oracle/baselines/tpch_sha256.json` generated (#3231)
- [ ] All 3 PRs merged to develop/v3.9.0
- [ ] P15 meta-gate remains PASS (post-Sprint 8)

---

*Last updated: 2026-06-18*
*Status: APPROVED — ready for implementation*
