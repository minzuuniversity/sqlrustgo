# SQLRustGo v3.6.0 性能基准测试

> **版本**: v3.6.0
> **HEAD**: 1b2a3c71
> **日期**: 2026-05-30
> **状态**: ⏳ TPC-H 基准测试尚未运行 — 占位文档
> **SSOT**: docs/governance/SSOT_CROSS_CHECK.md

---

## 1. 概述

本文档记录 SQLRustGo v3.6.0 的性能基准测试计划与结果。

**当前状态**: TPC-H 基准测试尚未在 v3.6.0 分支上执行。以下为测试计划和预期目标。

---

## 2. 测试计划

### 2.1 TPC-H 基准测试

| 项目 | 值 |
|------|-----|
| 数据规模 | SF=0.1 (100MB), SF=1 (1GB) |
| 查询数 | 22 个 TPC-H 查询 (Q1-Q22) |
| 测试工具 | `scripts/bench/run_tpch.sh` |
| 执行环境 | Z6G4 Server (192.168.0.252) |
| Rust 版本 | 1.85+ |

### 2.2 执行命令

```bash
# SF=0.1 (22 查询)
bash scripts/bench/run_tpch.sh --scale-factor 0.1

# SF=1 (22 查询)
bash scripts/bench/run_tpch.sh --scale-factor 1
```

### 2.3 QPS 基准测试

```bash
# 查询吞吐量
cargo test --test qps_benchmark_test
```

---

## 3. 预期目标

| 指标 | v3.5.0 (SF=1) | v3.6.0 目标 | 说明 |
|------|----------------|--------------|------|
| TPC-H Q1 | ~280ms | < 250ms | SIMD 可能带来改善 |
| TPC-H 全量 | 22/22 PASS | 22/22 PASS | 功能正确性 |
| SIMD 加速比 | 1x (基础) | >= 2x | 向量距离计算 |
| QPS | 待定 | <= 5% 退化 | 无性能退化 |

---

## 4. 参考: v2.8.0 SIMD 基准

| 操作 | 标量 (ms) | SIMD (ms) | 加速比 |
|------|-----------|-----------|--------|
| L2 距离 (1024维) | 0.045 | 0.008 | **5.6x** |
| 余弦相似度 (1024维) | 0.048 | 0.009 | **5.3x** |
| 批量距离 (1000x1000) | 45.2 | 11.3 | **4.0x** |

> v3.6.0 的 SIMD 集成应保持或超越以上加速比。

---

## 5. 测试环境

### 5.1 Z6G4 Server

| 配置 | 值 |
|------|-----|
| CPU | Intel Xeon (AVX2 支持) |
| 内存 | 32GB+ |
| 磁盘 | NVMe SSD |
| OS | Ubuntu 22.04 LTS |

### 5.2 软件配置

| 配置 | 值 |
|------|-----|
| Rust | 1.85+ |
| SIMD 级别 | AVX2 (lanes=8) |
| 测试工具 | cargo bench, TPC-H harness |

---

## 6. TODO

- [ ] 在 Z6G4 上执行 TPC-H SF=0.1 (22 查询)
- [ ] 在 Z6G4 上执行 TPC-H SF=1 (22 查询)
- [ ] 记录 Q1-Q22 各查询执行时间
- [ ] 与 v3.5.0 结果对比
- [ ] 运行 QPS 基准测试
- [ ] 更新本文档为正式报告

---

*SSOT 参考: docs/governance/SSOT_CROSS_CHECK.md*
*更新日期: 2026-05-30*
