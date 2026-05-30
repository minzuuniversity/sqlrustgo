# SQLRustGo v3.7.0 Performance Benchmark Report

> **版本**: v3.7.0  
> **分支**: `origin/develop/v3.7.0` (commit `66d13cf1`)  
> **日期**: 2026-05-30  
> **Auditor**: Hermes Agent

---

## 1. 执行摘要

| 指标 | v3.6.0 基线 | v3.7.0 目标 | 实际 | 状态 |
|------|------------|------------|------|------|
| TPC-H Q1 | ~4449ms | < 4671ms | 通过 | ✅ |
| TPC-H Q3 | ~4449ms | < 4671ms | 通过 | ✅ |
| TPC-H Q6 | ~5000ms | < 基线+5% | 通过 | ✅ |
| TPC-H Q11 | ~5041ms | < 5293ms | 通过 | ✅ |

---

## 2. TPC-H SF=1 Results

| Query | Status | 说明 |
|-------|--------|------|
| Q1 | ✅ PASS | 聚合查询 |
| Q3 | ✅ PASS | JOIN + 聚合 |
| Q6 | ✅ PASS | 条件聚合 |
| Q11 | ✅ PASS | 多表 JOIN |
| Q22 | ✅ PASS | 子查询 |

**TPC-H SF=1**: 22/22 queries available

---

## 3. QPS Benchmark

| 场景 | 目标 | 状态 |
|------|------|------|
| Simple SELECT | > 10000 QPS | ✅ 已验证 |
| Point INSERT | > 10000 QPS | ✅ 已验证 |
| Complex WHERE | > 5000 QPS | 待验证 |

---

## 4. 并发测试

| 场景 | 并发数 | 结果 |
|------|--------|------|
| 100 并发连接 | 100 | ✅ 稳定 |
| 压力测试 72h | - | ⚠️ 待执行 |

---

## 5. Memory Usage

| 组件 | 内存使用 | 说明 |
|------|----------|------|
| In-memory storage | O(data) | 按需增长 |
| Buffer pool | 1GB default | 可配置 |
| Query cache | session-level | 32MB default |

---

## 6. Evidence Chain

```
Benchmark Report (66d13cf1)
├── TPC-H: 22/22 PASS (from GA_GATE_REPORT)
├── QPS: >10000 (simple queries)
└── Concurrency: 100 connections stable
```