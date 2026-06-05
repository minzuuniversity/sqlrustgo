# G11 QPS/TPS Benchmark Report (v3.9.0)

> **Generated**: 2026-06-05
> **Gate**: G11 (cargo bench 5 workloads × 4 thread counts = 20 measurements)
> **Ref**: V390_TEST_PLAN_SUPPLEMENT_PERF.md §G11
> **Tool**: cargo bench (criterion-based)

---

## 1. 概述

G11 通过 `benches/qps_bench.rs` (criterion-based) 测量 5 类工作负载在不同线程数下的吞吐量.

| 组件 | 状态 |
|------|------|
| qps_bench.rs | ✅ 实施 (5 workloads, 4 thread counts) |
| Cargo.toml 注册 | ✅ |
| G11 gate (5/5) | ✅ PASS |
| 真实测量 (Z6G4) | ⏳ W11 D1 完成 (本地 quick mode) |

---

## 2. 5 工作负载 × Thread Counts

| 工作负载 | 线程数 | 测量目标 |
|----------|--------|----------|
| **qps_point_select** | 1/4/8/16 | 主键点查 (1K queries/iter) |
| **qps_range_select** | 1/4/8 | 索引扫描 (1K queries/iter) |
| **qps_insert** | 1/4/8 | 写 (1K rows/iter) |
| **qps_update** | 1/4/8 | 索引列更新 (500 rows/iter) |
| **qps_mixed_oltp** | 4/8 | sysbench-like (70% SELECT + 20% UPDATE + 10% INSERT) |

---

## 3. 本地 Quick Mode 结果 (示例)

```
qps_point_select 1:   335170161 ns/iter (335ms/1000q)
qps_point_select 4:   432007645 ns/iter (432ms/4000q)
qps_point_select 8:   687388999 ns/iter (687ms/8000q)
qps_point_select 16: 1249473177 ns/iter (1.25s/16Kq)

qps_range_select 1:   338172396 ns/iter
qps_range_select 4:   406998604 ns/iter
qps_range_select 8:   678648218 ns/iter

qps_insert 1:           123860 ns/iter
qps_insert 4:           724080 ns/iter
qps_insert 8:          2635270 ns/iter

qps_update 1:          6667794 ns/iter
qps_update 4:         28536838 ns/iter
qps_update 8:         60042239 ns/iter

qps_mixed_oltp 4:     536237291 ns/iter
qps_mixed_oltp 8:    1191237984 ns/iter
```

(Quick mode = 1 sample. 真实测量需 Z6G4 + criterion 默认 100 samples)

---

## 4. 真实结果 (TBD, Z6G4 测量后填)

| 工作负载 | 线程 | v3.8.0 (QPS) | v3.9.0 (QPS) | Δ | 阈值 (±10%) | 状态 |
|----------|------|--------------|--------------|---|------------|------|
| point_select | 1 | TBD | TBD | TBD | 4,500-5,500 | TBD |
| point_select | 4 | TBD | TBD | TBD | 13,500-16,500 | TBD |
| point_select | 8 | TBD | TBD | TBD | 22,500-27,500 | TBD |
| point_select | 16 | TBD | TBD | TBD | 31,500-38,500 | TBD |
| range_select | 1 | TBD | TBD | TBD | 900-1,100 | TBD |
| range_select | 4 | TBD | TBD | TBD | 2,700-3,300 | TBD |
| range_select | 8 | TBD | TBD | TBD | 4,500-5,500 | TBD |
| insert | 1 | TBD | TBD | TBD | 1,800-2,200 | TBD |
| insert | 4 | TBD | TBD | TBD | 4,500-5,500 | TBD |
| insert | 8 | TBD | TBD | TBD | 7,200-8,800 | TBD |
| update | 1 | TBD | TBD | TBD | 1,350-1,650 | TBD |
| update | 4 | TBD | TBD | TBD | 3,600-4,400 | TBD |
| update | 8 | TBD | TBD | TBD | 5,400-6,600 | TBD |
| mixed_oltp | 4 | TBD | TBD | TBD | 2,700-3,300 | TBD |
| mixed_oltp | 8 | TBD | TBD | TBD | 4,500-5,500 | TBD |

**目标**: v3.9.0 QPS ≥ v3.8.0 × 0.9 (-10% threshold)

---

## 5. 测量方法

```bash
# 完整 (默认 100 samples, ~5-10 分钟)
bash scripts/bench/run_qps_benchmarks.sh

# 快速 (1 sample, ~30 秒)
cargo bench --bench qps_bench -- --quick
```

---

## 6. 验证

```
✅ qps_bench.rs compiles (cargo check --bench qps_bench)
✅ 5 workloads detected by G11 gate
✅ 15 measurements (5 × 3-4 thread counts) run successfully
✅ TPC-H 22/22 维持
⏳ 真实 QPS 数字 (W11 D1, Z6G4)
```

---

**Ref**:
- benches/qps_bench.rs (290 lines, 5 workloads)
- scripts/bench/run_qps_benchmarks.sh (编排)
- scripts/gate/check_g11_qps.sh (5-check gate)
- PERFORMANCE_BASELINE.md §2 (G11 数据填入位置)
