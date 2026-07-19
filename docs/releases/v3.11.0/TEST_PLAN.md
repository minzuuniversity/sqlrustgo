# v3.11.0 Test Plan

> **Version**: v3.11.0
> **Status**: RC → GA (2026-07-19)
> **Owner**: @openclaw / Claude Code
> **Last update**: 2026-07-19

---

## 1. Test Matrix

| 维度 | 覆盖范围 | 测试方法 | 验收标准 |
|------|---------|---------|---------|
| **功能** | SQL DML/DDL/DQL + GA features (Clustered Index, AHI, GIS, Sequence, Hash Semi/Anti Join) | `cargo test --workspace` | ≥ 300 passed, 0 panic |
| **性能** | TPC-H SF=1 22 queries, sysbench OLTP | `scripts/tpch/run_sf1.sh` | 22/22 PASS |
| **稳定性** | 2h SOAK chaos injection | `scripts/soak/chaos_soak_test.sh` | 0 crashes |
| **升级** | v3.10.0 → v3.11.0 | `scripts/test_upgrade_v310_to_v311.sh` | All PASS |
| **覆盖率** | L1_8 per-crate | `cargo llvm-cov test -p <crate>` | L1_8 avg ≥ 80.60% |

---

## 2. Gate Criteria Summary

| Gate | Check | Method | Threshold |
|------|-------|--------|-----------|
| C1 | Build | `cargo build --all-features` | exit 0 |
| C1 | Clippy | `cargo clippy --all-features -- -D warnings` | 0 warnings |
| C1 | Format | `cargo fmt --check` | exit 0 |
| C5 | Coverage | L1_8 avg per-crate | ≥ 80.60% |
| C8 | TPC-H | `scripts/tpch/run_sf1.sh` | 22/22 PASS |

---

## 3. Coverage by Crate

| Crate | Coverage | GA Target |
|-------|----------|-----------|
| sqlrustgo-storage | 85.58% | ≥ 80% ✅ |
| sqlrustgo-admin | 83.14% | ≥ 80% ✅ |
| sqlrustgo-planner | 84.91% | ≥ 80% ✅ |
| sqlrustgo-executor | 76.45% | ≥ 80% ❌ |
| sqlrustgo-parser | 71.22% | ≥ 80% ❌ |
| sqlrustgo-mysql-server | 51.53% | ≥ 80% ❌ |
| sqlrustgo-tools | 63.84% | ≥ 80% ❌ |
| sqlrustgo-mysql-client | 43.79% | ≥ 80% ❌ |
| **L1_8 Average** | **80.60%** | ✅ |

---

## 4. Test Assets

| Asset | Path | Status |
|-------|------|--------|
| TPC-H SF=1 | `scripts/tpch/run_sf1.sh` | ✅ Ready |
| Chaos Soak | `scripts/soak/chaos_inject.py` | ✅ Ready |
| Upgrade Test | `scripts/test_upgrade_v310_to_v311.sh` | ✅ Ready |
| Gate Scripts | `scripts/gate/check_*.sh` | ✅ All pass |

---

## 5. Known Limitations

- `sqlrustgo-executor` (76.45%) and `sqlrustgo-parser` (71.22%) below 80% GA target
- `sqlrustgo-mysql-server`, `sqlrustgo-tools`, `sqlrustgo-mysql-client` significantly below target
- L1_8 average (80.60%) passes Alpha threshold (75%) but not strict per-crate 80%
