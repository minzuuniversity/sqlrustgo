# v3.6.0 Benchmark Gate Specification

> **版本**: 1.0
> **创建日期**: 2026-05-29
> **维护人**: hermes-agent
> **用途**: v3.6.0 Benchmark Gate CI/CD 规范

---

## 一、Benchmark Gate 概述

### 为什么现在必须建立

现在是"优化器时代"：
- CI 不会因为性能回退变红
- 用户会 silently fail
- 比 crash 更危险

**历史教训**：
- v3.0.0 GA 发布时无性能基线
- 无法判断哪个 commit 导致性能变化
- CBO/SIMD 优化效果无法量化

### Benchmark Gate 目标

建立性能回归检测体系，确保：
1. 每个版本有可量化的性能基线
2. 性能回退可被 CI 自动检测
3. 优化效果有历史趋势可查

---

## 二、Benchmark 场景定义

### 2.1 核心场景

| 场景 | 指标 | 测量方式 | CI Threshold |
|------|------|---------|-------------|
| Point Select | QPS | 并发 16 线程，1000 次查询 | 基准 ±15% |
| Range Scan | rows/sec | 10K 行范围扫描 | 基准 ±15% |
| Hash Join | rows/sec | 1M × 1M 等值 join | 基准 ±20% |
| Aggregation | rows/sec | 10M 行 GROUP BY | 基准 ±15% |
| WAL fsync | latency (p99) | 单事务提交 | ≤ 基准 ×1.3 |
| TPC-H SF=1 | total time (s) | 22 queries | ≤ 180s |
| Concurrent TXN | TPS | 16 并发事务 | 基准 ±20% |
| Vector Search | QPS | ANN-10 查询 | 基准 ±25% |

### 2.2 详细测量方法

#### Point Select
```bash
cargo bench -- scenario point_select \
    --threads 16 \
    --iterations 1000 \
    --key-range 1-100000
```
输出：`{"qps": 125000, "latency_p50_us": 128, "latency_p99_us": 256}`

#### TPC-H SF=1
```bash
cargo bench -- scenario tpch \
    --sf 1 \
    --output json \
    --timeout 300
```
输出：`{"query_1_time_ms": 123, ..., "total_time_s": 145, "passed": 22, "failed": 0}`

#### WAL fsync
```bash
cargo bench -- scenario wal_fsync \
    --iterations 10000 \
    --batch-size 100
```
输出：`{"latency_p50_us": 820, "latency_p99_us": 1200, "latency_p999_us": 1500}`

---

## 三、CI 集成

### 3.1 Baseline 建立

```yaml
# .github/workflows/benchmark-baseline.yml
name: Benchmark Baseline

on:
  push:
    branches: [main, 'release/**']

jobs:
  baseline:
    runs-on: [self-hosted, benchmark]
    steps:
      - uses: actions/checkout@v4
      - name: Build
        run: cargo build --release
      - name: Establish Baseline
        run: |
          cargo bench --save-baseline main-$(date +%Y%m%d)
          cargo bench --save-baseline main
```

### 3.2 Regression Gate

```yaml
# .github/workflows/benchmark-gate.yml
name: Benchmark Gate

on:
  pull_request:
    branches: ['develop/**']

jobs:
  benchmark:
    runs-on: [self-hosted, benchmark]
    steps:
      - uses: actions/checkout@v4
      - name: Build
        run: cargo build --release
      - name: Run Benchmarks
        run: |
          cargo bench --baseline main --compare-to main --json > benchmark_results.json
      - name: Check Regression
        run: |
          cargo bench --fail-under-baseline 0.85
```

### 3.3 Threshold 配置文件

```yaml
# benchmarks/thresholds.yaml
thresholds:
  point_select:
    metric: qps
    fail_under_baseline: 0.85  # -15%
    warn_under_baseline: 0.90

  range_scan:
    metric: rows_per_sec
    fail_under_baseline: 0.85
    warn_under_baseline: 0.90

  hash_join:
    metric: rows_per_sec
    fail_under_baseline: 0.80  # -20%
    warn_under_baseline: 0.85

  aggregation:
    metric: rows_per_sec
    fail_under_baseline: 0.85
    warn_under_baseline: 0.90

  wal_fsync:
    metric: latency_p99_us
    fail_over_baseline: 1.30  # +30%
    warn_over_baseline: 1.20

  tpch_sf1:
    metric: total_time_s
    fail_over_baseline: 1.20  # +20%
    warn_over_baseline: 1.15
    absolute_max: 180  # hard limit

  concurrent_txn:
    metric: tps
    fail_under_baseline: 0.80
    warn_under_baseline: 0.85

  vector_search:
    metric: qps
    fail_under_baseline: 0.75  # -25%
    warn_under_baseline: 0.80
```

---

## 四、输出格式

### 4.1 JSON 输出

```json
{
  "version": "v3.6.0-alpha",
  "timestamp": "2026-05-29T10:30:00Z",
  "commit": "abc1234",
  "benchmarks": [
    {
      "name": "point_select",
      "metrics": {
        "qps": 125000,
        "latency_p50_us": 128,
        "latency_p95_us": 210,
        "latency_p99_us": 256
      },
      "baseline": {
        "qps": 120000,
        "delta_pct": 4.2
      },
      "status": "pass"
    },
    {
      "name": "tpch_sf1_q1",
      "metrics": {
        "time_ms": 123,
        "rows_scanned": 1000000
      },
      "baseline": {
        "time_ms": 130,
        "delta_pct": -5.4
      },
      "status": "pass"
    }
  ],
  "summary": {
    "total": 8,
    "passed": 8,
    "failed": 0,
    "warnings": 0
  }
}
```

### 4.2 趋势存储

```
benchmarks/
├── baselines/
│   ├── main-20260528.json
│   ├── main-20260529.json
│   └── develop-v360-alpha-20260529.json
├── trends/
│   ├── point_select.png
│   ├── tpch_sf1.png
│   └── wal_fsync.png
└── reports/
    └── v360-rc1-benchmark-report.md
```

---

## 五、Benchmark 工具要求

### 5.1 必须满足的条件

1. **可重复性**: 相同 commit，多次运行结果差异 < 2%
2. **隔离性**: 单 benchmark 不受其他 benchmark 影响
3. **预热**: 正式测量前有 3 次预热
4. **清理**: 每次 benchmark 前清理 page cache / buffer pool
5. **超时**: 单场景超时 300s

### 5.2 环境要求

| 资源 | 最小要求 |
|------|---------|
| CPU | 16 cores |
| Memory | 32GB |
| Disk | SSD, 50GB free |
| OS | Linux (no container) |

---

## 六、TPC-H SF=1 详细指标

### 6.1 Query Timeouts

| Query | Timeout | 说明 |
|-------|---------|------|
| Q1 | 30s | Price aggregation |
| Q2 | 30s | Part/Supplier join |
| Q3 | 30s | Lineitem filter/join |
| ... | ... | ... |
| Q22 | 30s | Country sales |

### 6.2 通过标准

- 22/22 queries 必须 PASS
- 总时间 ≤ 180s
- 无 query 超时
- 无 crash
- 无 OOM

---

## 七、Performance Regression 检测

### 7.1 自动检测规则

```python
def check_regression(benchmark_name, current, baseline):
    threshold = THRESHOLDS[benchmark_name]
    if benchmark_name in ["wal_fsync", "tpch_sf1"]:
        # 越小越好
        if current > baseline * threshold["fail_over_baseline"]:
            return "FAIL", f"+{((current/baseline)-1)*100:.1f}%"
        if current > baseline * threshold["warn_over_baseline"]:
            return "WARN", f"+{((current/baseline)-1)*100:.1f}%"
    else:
        # 越大越好
        if current < baseline * threshold["fail_under_baseline"]:
            return "FAIL", f"-{((1-current/baseline))*100:.1f}%"
        if current < baseline * threshold["warn_under_baseline"]:
            return "WARN", f"-{((1-current/baseline))*100:.1f}%"
    return "PASS", f"+/-{((current/baseline)-1)*100:.1f}%"
```

### 7.2 历史趋势分析

```python
# benchmarks/analyze_trend.py
def analyze_trend(benchmark_name, days=30):
    """分析某 benchmark 过去 N 天的趋势"""
    data = load_benchmarks(benchmark_name, days=days)
    # 线性回归
    slope, intercept = linear_regression(data.timestamps, data.metrics)
    # 检测异常
    anomalies = detect_anomalies(data)
    return {
        "trend": slope,
        "anomalies": anomalies,
        "recommendation": "optimize" if slope < 0 else "maintain"
    }
```

---

## 八、版本基线记录

| 版本 | 日期 | Point Select | TPC-H SF=1 | WAL p99 | Commit |
|------|------|-------------|------------|---------|--------|
| v3.5.0 GA | 2026-05-28 | TBD | ~150s | TBD | 5720805b |

---

*文档版本: 1.0 | 待 Benchmark 工具实现后更新*
