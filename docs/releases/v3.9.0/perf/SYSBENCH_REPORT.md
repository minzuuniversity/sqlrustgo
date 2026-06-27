# G12 Sysbench OLTP Report (v3.9.0)

> **Generated**: 2026-06-05
> **Gate**: G12 (Sysbench 5 OLTP workloads + 30+ unit tests)
> **Ref**: V390_TEST_PLAN_SUPPLEMENT_PERF.md §G12
> **Tool**: sysbench 1.0.20 + sqlrustgo (MySQL wire protocol)

---

## 1. 概述

| 组件 | 状态 |
|------|------|
| 5 sysbench 脚本 | ✅ 已实施 (scripts/sysbench/) |
| 30+ OLTP unit tests | ✅ 已 PASS (crates/bench/tests/oltp_test.rs) |
| 真实 5 工作负载运行 | ⏳ W12 D2 (Z6G4, 需 sqlrustgo server) |

---

## 2. 5 Sysbench 工作负载脚本

| 脚本 | 工作负载 | Threads | 用途 |
|------|----------|---------|------|
| `oltp_point_select.sh` | 主键点查 | 1/4/8/16 | 最低延迟基线 |
| `oltp_read_only.sh` | 只读事务 | 1/4/8 | 读路径 |
| `oltp_read_write.sh` | 混合读写 (80/20) | 8 | OLTP 标准 |
| `oltp_write_only.sh` | 纯写事务 | 8 | 写吞吐 |
| `oltp_insert.sh` | 批量插入 | 8 | 数据加载 |

---

## 3. 30+ OLTP Unit Tests (Pre-GA PASS)

| 类别 | Tests | 状态 |
|------|-------|------|
| 原有 (7) | oltp_point_select, range_select, insert, delete, mixed, bulk_insert, aggregation | ✅ PASS |
| 扩展 (23) | range_scan_100, point_select_batch_1000, secondary_index_scan_500, update_by_id_200, delete_by_id_100, count_star_50, sum_k_50, avg_k_50, min_max_50, group_by_k_20, order_by_50, distinct_20, concurrent_point_select_4_threads, concurrent_mixed_4_threads, large_table_setup_10k, count_with_filter_50, indexed_lookup_500, varchar_like_30, range_with_order_50, aggregation_with_aliases_20, subquery_count_20, insert_select_round_trip_100, batch_update_50 | ✅ PASS |
| **Total** | **30/30** | ✅ PASS |

耗时: 64.66s (30 tests, 100ms-15s per test)

---

## 4. 真实 Sysbench 运行结果 (TBD, W12 D2 跑)

| 工作负载 | 线程 | v3.8.0 (TPS) | v3.9.0 (TPS) | Δ | 阈值 (±10%) | 状态 |
|----------|------|--------------|--------------|---|------------|------|
| oltp_point_select | 1 | TBD | TBD | TBD | TBD | TBD |
| oltp_point_select | 4 | TBD | TBD | TBD | TBD | TBD |
| oltp_point_select | 8 | TBD | TBD | TBD | TBD | TBD |
| oltp_read_only | 1 | TBD | TBD | TBD | TBD | TBD |
| oltp_read_only | 4 | TBD | TBD | TBD | TBD | TBD |
| oltp_read_only | 8 | TBD | TBD | TBD | TBD | TBD |
| oltp_read_write | 8 | TBD | TBD | TBD | TBD | TBD |
| oltp_write_only | 8 | TBD | TBD | TBD | TBD | TBD |
| oltp_insert | 8 | TBD | TBD | TBD | TBD | TBD |

**目标**: v3.9.0 TPS ≥ v3.8.0 × 0.9 (-10% threshold)

---

## 5. 验证

```
✅ 5 sysbench 脚本存在 (scripts/sysbench/oltp_*.sh)
✅ 30+ OLTP unit tests PASS (cargo test -p sqlrustgo-bench --test oltp_test)
✅ G12 门禁 7/7 PASS (scripts/gate/check_g12_sysbench.sh)
✅ TPC-H 22/22 维持
⏳ 真实 5-工作负载 sysbench (W12 D2, Z6G4)
```

---

## 6. 与 v3.8.0 baseline 对比

详见 [PERFORMANCE_BASELINE.md](PERFORMANCE_BASELINE.md) §3.

---

**Ref**:
- V390_TEST_PLAN_SUPPLEMENT_PERF.md §G12
- scripts/sysbench/oltp_*.sh (5 scripts)
- crates/bench/tests/oltp_test.rs (30 tests)
- scripts/gate/check_g12_sysbench.sh (7-check gate)
- PERFORMANCE_BASELINE.md (G12 数据填入位置)
