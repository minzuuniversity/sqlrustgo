# SPEC: Benchmark Gate CI Framework

> **Issue**: #1462
> **Status**: Implemented
> **Created**: 2026-05-29
> **Branch**: `develop/v3.6.0`

---

## 1. 概述

### 1.1 目标

建立 Benchmark Gate CI 框架，防止性能回归。

### 1.2 为什么需要 Benchmark Gate

现在是"优化器时代"，性能回归比功能回归更危险：
- CI 不会变红
- 用户会 silently fail
- 无法知道哪个 commit 导致回退

### 1.3 解决方案

```
Baseline JSON → check_benchmark_gate.sh → PASS/FAIL
                        ↑
                        |
              当前 Benchmark JSON
```

---

## 2. 架构设计

### 2.1 文件结构

```
scripts/
├── bench/
│   ├── run_bench.sh              # 运行 benchmark 并输出 JSON
│   └── check_benchmark_gate.sh   # CI gate 脚本
└── .benchmarks/
    └── baseline.json             # 基线数据
```

### 2.2 Baseline JSON 格式

```json
{
  "version": "3.6.0",
  "timestamp": "2026-05-29T12:00:00Z",
  "commit": "47c51082",
  "benchmarks": [
    {
      "name": "bench_scan::seq_scan_1000",
      "unit": "ns",
      "mean": 15234.5
    }
  ]
}
```

### 2.3 check_benchmark_gate.sh 逻辑

```
1. 读取 baseline.json
2. 读取当前 benchmark.json
3. 对每个 benchmark：
   - 如果 current_mean > baseline_mean * (1 + threshold) → 回归
   - 如果 current_mean < baseline_mean * (1 - threshold) → 改进
4. 任何回归 → exit 1 (FAIL)
5. 全部通过 → exit 0 (PASS)
```

---

## 3. 验收条件

| 条件 | 说明 |
|------|------|
| `cargo bench -- --list` 正常 | 列出所有 bench |
| TPC-H SF=1 Q1 基线建立 | bench_tpch::q1 |
| Point Select QPS 基线建立 | bench_scan::seq_scan_1000 |
| check_benchmark_gate.sh 可执行 | chmod +x |
| 基线不存在时自动创建 | 首次运行不失败 |

---

## 4. 已实现文件

### 4.1 scripts/bench/run_bench.sh

运行 benchmark 并输出 JSON。

### 4.2 scripts/bench/check_benchmark_gate.sh

CI gate 脚本，阈值 15%。

---

## 5. 使用方法

### 5.1 首次运行（创建基线）

```bash
./scripts/bench/run_bench.sh
cp benchmark.json .benchmarks/baseline.json
```

### 5.2 日常 CI 检查

```bash
./scripts/bench/run_bench.sh
./scripts/bench/check_benchmark_gate.sh
```

### 5.3 GitHub Actions 集成

```yaml
- name: Benchmark Gate
  run: |
    ./scripts/bench/run_bench.sh
    ./scripts/bench/check_benchmark_gate.sh
```

---

## 6. 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| 基线环境差异 | 硬件不同导致误报 | 使用相同的 CI runner |
| 单次运行波动 | 噪声导致不稳定 | 使用多次运行取中位数 |

---

*Spec version: 1.0 | Implemented*
