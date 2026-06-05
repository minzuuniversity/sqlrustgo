# SQLRustGo v3.9.0 Performance Baseline

> **Generated**: 2026-06-05
> **Ref**: V390_TEST_PLAN_ROUND2_REVIEW §Perf Baseline (用户评审新增)
> **Goal**: 量化性能回退。任何指标 < 阈值 → FAIL (GA 卡死)

---

## 1. 测量环境

| 项目 | 规格 |
|------|------|
| 平台 | x86_64 (Z6G4) |
| CPU | 8 cores @ 2.4 GHz |
| RAM | 64 GB DDR4 |
| Storage | NVMe SSD |
| 操作系统 | Linux 5.x |
| 测量工具 | cargo bench (criterion) + sysbench 1.0.20 + Z6G4 |

| 版本对比 | 详情 |
|----------|------|
| **v3.8.0 baseline** | commit `40f62ab5a367` (v3.8.0 GA Final) |
| **v3.9.0** | commit `fd91a33dc` (develop/v3.9.0 HEAD) |

---

## 2. QPS 基准 (G11 cargo bench)

| 工作负载 | 线程 | v3.8.0 (QPS) | v3.9.0 (QPS) | Δ | 阈值 (±10%) | 状态 |
|----------|------|---------------|--------------|---|------------|------|
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

## 3. Sysbench OLTP 基准 (G12 sysbench 1.0.20)

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

**目标**: v3.9.0 TPS ≥ v3.8.0 × 0.9

---

## 4. 延迟基准 (P50 / P95 / P99)

| 工作负载 | 百分位 | v3.8.0 (ms) | v3.9.0 (ms) | Δ | 阈值 (+15%) | 状态 |
|----------|--------|-------------|-------------|---|------------|------|
| point_select | P50 | TBD | TBD | TBD | TBD | TBD |
| point_select | P95 | TBD | TBD | TBD | TBD | TBD |
| point_select | P99 | TBD | TBD | TBD | TBD | TBD |
| insert | P95 | TBD | TBD | TBD | TBD | TBD |
| update | P95 | TBD | TBD | TBD | TBD | TBD |

**目标**: v3.9.0 延迟 ≤ v3.8.0 × 1.15 (+15% threshold)

---

## 5. 资源基准 (24h 后)

| 指标 | v3.8.0 | v3.9.0 | Δ | 阈值 | 状态 |
|------|--------|--------|---|------|------|
| RSS 增长 (MB/24h) | TBD | TBD | TBD | < 50 MB | TBD |
| FD 句柄增长 | TBD | TBD | TBD | < 50 | TBD |
| WAL 大小 (MB) | TBD | TBD | TBD | < 1000 MB | TBD |
| 锁等待 (avg ms) | TBD | TBD | TBD | < 50 ms | TBD |
| 崩溃次数 | TBD | TBD | TBD | == 0 | TBD |

**目标**: v3.9.0 资源 ≤ v3.8.0 × 1.10 (or absolute threshold)

---

## 6. 跨版本升级 (G16)

| 场景 | v3.8.0→v3.9.0 兼容? | 数据完整? | 状态 |
|------|----------------------|----------|------|
| Case 1: data dir | TBD | TBD | TBD |
| Case 2: WAL replay | TBD | TBD | TBD |
| Case 3: Snapshot/MVCC | TBD | TBD | TBD |
| Case 4: Metadata/Catalog | TBD | TBD | TBD |
| Case 5: Rollback | TBD | TBD | TBD |

**目标**: 5/5 cases pass

---

## 7. 性能回退门禁规则 (Gate Logic)

```rust
// scripts/gate/check_perf_baseline.sh
for row in baseline:
    delta = (v3.9.0 - v3.8.0) / v3.8.0
    if row.kind == "QPS" or row.kind == "TPS":
        if delta < -0.10:  // -10% 阈值
            FAIL "性能回退 >10% in {row.workload}"
    elif row.kind == "latency":
        if delta > +0.15:  // +15% 阈值
            FAIL "延迟增加 >15% in {row.workload}"
    elif row.kind == "resource":
        if row.value > row.threshold:
            FAIL "资源超阈 in {row.metric}"
```

---

## 8. 测量与更新流程

### 8.1 何时测量

| 阶段 | 测量频次 | 谁 |
|------|----------|------|
| 每次 PR | CI 跑 cargo bench smoke (--quick) | CI |
| RC 阶段 | Z6G4 真实 1 次 (本 baseline 填入) | Hermes |
| GA 前 | Z6G4 真实 1 次 (验证未退化) | Hermes |
| Post-GA 持续 | 每月 1 次 (Nightly) | Nightly CI |

### 8.2 如何更新 baseline

1. 跑 `bash scripts/bench/run_qps_benchmarks.sh` → 生成 QPS_REPORT.md
2. 跑 `bash scripts/sysbench/oltp_*.sh` (8 thread, 60s) → 生成 SYSBENCH_REPORT.md
3. 把数字填入本表 (TBD → 实际值)
4. 把阈值计算结果填入 (基于 ±10% 规则)
5. git commit + 4 remote push (alpha → GA 流程)

### 8.3 失败处理

如果某项 < 阈值 (FAIL):
1. **不要 close Issue, 保持 Open + progress report**
2. 调查: 是代码退化? 数据集差异? 测量误差?
3. 修复 → 重测 → baseline 更新 → close

---

## 9. 状态

| 项目 | 当前 | 目标 |
|------|------|------|
| Baseline 表存在 | ✅ | ✅ |
| v3.8.0 数据 | TBD (待 Z6G4 测量) | ✅ |
| v3.9.0 数据 | TBD (待 Z6G4 测量) | ✅ |
| Gate 规则实现 | ✅ scripts/gate/check_perf_baseline.sh | ✅ |
| 5/5 cases pass | TBD (W12 D3 真实跑) | ✅ |

---

**Refs**:
- V390_TEST_PLAN_SUPPLEMENT_PERF.md (G11-G15)
- V390_TEST_PLAN_ROUND2_REVIEW.md (G16 + Baseline, 用户评审新增)
- GA §4.2 (governance 性能验收)
- scripts/gate/check_perf_baseline.sh (自动门禁)
- scripts/bench/run_qps_benchmarks.sh (G11 编排)
- scripts/sysbench/*.sh (G12 5 个工作负载)
