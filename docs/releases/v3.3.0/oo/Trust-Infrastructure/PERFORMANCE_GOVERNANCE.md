# OO-TI1: Performance Governance System

> **版本**: v1.0
> **日期**: 2026-05-18
> **基于**: v3.3.0
> **维护人**: hermes-agent
> **Issue**: #1235
> **状态**: 已实现 (PR #1244)

---

## 一、概述

### 1.1 目标

建立 PR 级性能门禁系统，防止性能继续退化，确保每次合入的代码不会导致性能回归。

### 1.2 核心理念

```
Performance Governance = Baseline Database + Regression Detection + Automated Alerting
```

### 1.3 问题背景

v3.2.0 暴露的性能问题：
- UPDATE QPS 下降 89%
- DELETE QPS 下降 91%
- 根因：冷存储分层 + S3 签名计算引入每次 DML 的 tier 检查开销

---

## 二、技术架构

### 2.1 组件关系

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Performance Governance System                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────────────┐    │
│  │   Git Hook   │───▶│ perf-gate   │───▶│  Baseline Database      │    │
│  │  (pre-push) │    │   script    │    │  (SQLite/JSON)        │    │
│  └──────────────┘    └──────────────┘    └──────────────────────────┘    │
│                              │                        │                     │
│                              │                        │                     │
│                              ▼                        ▼                     │
│                     ┌──────────────┐    ┌──────────────────────────┐    │
│                     │  Regression  │    │   Flamegraph Generator   │    │
│                     │  Detector   │    │   (perf/flamegraph)     │    │
│                     └──────────────┘    └──────────────────────────┘    │
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────┐    │
│  │                    Microbenchmark Suite                              │    │
│  │  - point_select    - update_simple    - delete_simple           │    │
│  │  - insert          - complex_where    - aggregation            │    │
│  └──────────────────────────────────────────────────────────────────┘    │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 组件说明

| 组件 | 说明 | 位置 |
|------|------|------|
| `perf-gate` | PR 级性能门禁脚本 | `scripts/gate/check_perf_baseline.sh` |
| `perf-baseline` crate | 性能时序数据库 | `crates/perf-baseline/` |
| `flamegraph` automation | perf profile 自动生成 | `scripts/perf/flamegraph.sh` |
| Microbenchmark suite | 关键路径 QPS 测量 | `benches/` |

---

## 三、数据结构

### 3.1 Baseline Database Schema

```sql
CREATE TABLE perf_baseline (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    commit_sha      TEXT NOT NULL,
    branch         TEXT NOT NULL,
    benchmark_name TEXT NOT NULL,
    qps            REAL NOT NULL,
    latency_p50    REAL,
    latency_p99    REAL,
    measured_at    TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(commit_sha, benchmark_name)
);

CREATE TABLE perf_regression (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    baseline_id     INTEGER REFERENCES perf_baseline(id),
    current_qps     REAL NOT NULL,
    regression_pct  REAL NOT NULL,
    status          TEXT DEFAULT 'open',  -- open/confirmed/fixed/rejected
    detected_at     TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### 3.2 Benchmark Result Format

```json
{
  "benchmark": "point_select",
  "commit": "abc123def",
  "branch": "develop/v3.3.0",
  "timestamp": "2026-05-18T12:00:00Z",
  "results": {
    "qps": 45230.5,
    "latency_p50_ms": 0.22,
    "latency_p99_ms": 0.89,
    "latency_p999_ms": 1.45
  },
  "environment": {
    "cpu": "Apple M2 Pro",
    "memory_gb": 32,
    "os": "macOS 14.5"
  }
}
```

---

## 四、执行流程

### 4.1 Pre-Push Performance Gate Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    Pre-Push Performance Gate                      │
└─────────────────────────────────────────────────────────────────┘

User runs: git push
        │
        ▼
┌─────────────────┐
│   Git Hook      │
│  (pre-push)    │
└────────┬────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────┐
│  1. Capture baseline commit (HEAD~1)                              │
│     - git rev-parse HEAD~1                                       │
│     - Store in PERF_BASELINE_COMMIT                              │
└─────────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────┐
│  2. Run Microbenchmarks on HEAD                                   │
│     - cargo bench -- --noplot                                    │
│     - Parse output: point_select, update_simple, etc.            │
│     - Store in PERF_CURRENT_RESULTS                              │
└─────────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────┐
│  3. Compare HEAD vs HEAD~1                                      │
│     For each benchmark:                                           │
│       regression = (current - baseline) / baseline * 100        │
│       If regression > 10%:                                        │
│         → BLOCK push                                              │
│         → Generate flamegraph                                    │
│         → Post comment to PR                                       │
└─────────────────────────────────────────────────────────────────┘
         │
         ▼
    ┌────────────┐
    │ regression │
    │   > 10%   │
    └─────┬──────┘
          │
    ┌────┴────┐
    │         │
   YES       NO
    │         │
    ▼         ▼
┌─────────┐  ┌─────────┐
│  BLOCK  │  │ ALLOW  │
│  Push   │  │  Push  │
└─────────┘  └─────────┘
```

### 4.2 Flamegraph Generation Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                  Flamegraph Generation                            │
└─────────────────────────────────────────────────────────────────┘

Trigger: Performance regression detected (>10%)
         │
         ▼
┌─────────────────────────────────────────────────────────────────┐
│  1. Profile with perf record                                    │
│     - perf record -F 99 -a -g -- cargo run --bin sqlrustgo    │
│     - Duration: 60 seconds                                       │
└─────────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────┐
│  2. Generate flamegraph                                         │
│     - perf script | flamegraph > perf.svg                       │
│     - Upload to artifacts/perf/                                   │
└─────────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────┐
│  3. Post to PR                                                  │
│     - Add comment with flamegraph link                            │
│     - @mention PR author                                        │
│     - Request performance review                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 五、验收标准

### 5.1 功能验收

| 检查项 | 命令 | 标准 |
|--------|------|------|
| Baseline 存储 | `perf-gate store baseline` | 成功存储 |
| Regression 检测 | `perf-gate check --commit HEAD~1 --head HEAD` | 正确计算 |
| 10% 阈值 | regression > 10% 时 | 自动 block |
| Flamegraph 生成 | `perf-gate flamegraph` | 生成 SVG |

### 5.2 集成验收

```bash
# 完整流程测试
git checkout feature/test-perf
cargo bench
perf-gate check --baseline HEAD~1 --head HEAD
# Expected: PASS if no regression

# 模拟 regression
git checkout HEAD~10
# 修改代码降低 point_select 20%
cargo bench
perf-gate check --baseline HEAD~1 --head HEAD
# Expected: FAIL with block message
```

---

## 六、实现状态

| 组件 | 状态 | PR |
|------|------|-----|
| perf-gate script | ✅ 已实现 | #1244 |
| perf-baseline crate | ✅ 已实现 | #1244 |
| Flamegraph automation | ✅ 已实现 | #1244 |
| Microbenchmark suite | ✅ 已实现 | #1244 |
| CI integration | ✅ 已实现 | #1244 |

---

## 七、相关文档

- `docs/releases/v3.3.0/TRUST_INFRASTRUCTURE_STRATEGY.md` - 战略定位
- `scripts/gate/check_perf_baseline.sh` - 门禁脚本
- `crates/perf-baseline/` - 性能基线 crate

---

*本文档由 hermes-agent 生成*
*版本 1.0 - 2026-05-18*
