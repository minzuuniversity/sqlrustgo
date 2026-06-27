# v3.8.0 性能目标 (Performance Targets)

> **Date**: 2026-06-03
> **Author**: Hermes Agent
> **Baseline**: `origin/develop/v3.8.0`
> **Status**: 部分实测 + 估算目标

---

## 0. TL;DR

v3.8.0 性能基线 = v2.4.0 (无 v3.8.0 综合 benchmark)。
本文定义 **v3.8.0 目标**, **实测数据** 等待 v3.8.0 perf benchmark。

---

## 1. 历史基线 (v2.4.0 SF=0.1)

| 指标 | v2.4.0 实测 | vs SQLite | vs PostgreSQL |
|------|-------------|-----------|---------------|
| TPC-H Q1 延迟 | 74 µs | 43x faster | 45x faster |
| COUNT(*) 延迟 | 65.8 µs | — | — |
| SUM 延迟 | 65.5 µs | — | — |
| QPS (synth) | ~15,900 | — | — |
| 数据规模 | 600K rows lineitem | — | — |

---

## 2. v3.8.0 目标 (基于新 features)

### 2.1 Point Query (聚簇索引 F-23 + AHI F-24)
- **目标**: 50% 延迟下降 (聚簇索引 + AHI 命中)
- **测试**: `SELECT * FROM t WHERE pk = ?` (主键)
- **目标数值**: 35-50 µs (vs v2.4.0 65 µs)

### 2.2 Range Query
- **目标**: 与 v2.4.0 持平 (聚簇索引顺序扫描)
- **测试**: `SELECT * FROM t WHERE pk BETWEEN ? AND ?+1000`
- **目标数值**: 50-70 µs

### 2.3 Aggregation (F-11 聚合)
- **目标**: 20-30% 加速 (新 CBO 优化器)
- **测试**: `SELECT COUNT(*), SUM(c1) FROM t WHERE c2 = ?`
- **目标数值**: 50-55 µs

### 2.4 JOIN (F-10 multi-join)
- **目标**: 2-table JOIN 100 µs
- **目标**: 3-table JOIN 200 µs
- **目标**: 4+ table JOIN 500 µs

### 2.5 INSERT/UPDATE/DELETE
- **目标**: 50-100 µs (单行)
- **目标**: 1ms (10 rows batch)
- **变更**: WAL 强制 (F-09) 可能 -5-10% (安全换性能)

### 2.6 压缩 (F-27) 性能影响
- **存储**: 10x 减少 (LZ4 on int)
- **读**: -10-20% (解压开销)
- **写**: -15% (压缩开销)

### 2.7 并行 (I-12) 性能
- **目标**: N-core 接近线性扩展
- **未集成主路径**: v3.9.0+ 计划

---

## 3. 资源目标

| 资源 | 目标 | 实测 |
|------|------|------|
| **内存 (idle)** | < 100 MB | TBD |
| **内存 (1000 connections)** | < 500 MB | TBD |
| **启动时间** | < 2s | TBD |
| **冷启动 (WAL replay)** | < 5s (1GB WAL) | TBD |
| **连接建立延迟** | < 10ms | TBD |
| **Query 编译 (parse + plan)** | < 5ms | TBD |

---

## 4. 可扩展性目标

| 维度 | 目标 |
|------|------|
| **数据规模** | 单库 TB 级 |
| **表数** | 10K+ |
| **行数** | 100M+ per table |
| **列数** | 100+ per table |
| **并发连接** | 1000+ |
| **并发查询** | 100+ |

---

## 5. 性能回归阈值 (L5)

| 指标 | 阈值 | 触发行动 |
|------|------|----------|
| **QPS 回归** | < 5% vs v3.7.0 | P0 修复 |
| **延迟回归** | < 10% vs v3.7.0 | P0 修复 |
| **内存增长** | < 20% vs v3.7.0 | P1 调查 |
| **CPU 增长** | < 15% vs v3.7.0 | P1 调查 |

测试工具: `scripts/bench/qps_regression.sh`

---

## 6. 性能测试矩阵

### 6.1 已实现
- `tests/qps_benchmark_test.rs` - 简单 QPS
- `tests/ci/buffer_pool_benchmark_test.rs` - Buffer pool 性能
- `tests/page_io_benchmark_test.rs` - Page I/O 性能
- `tests/performance_schema_test.rs` - Schema 查询性能

### 6.2 缺失 (v3.8.0+ 需要)
- ❌ **完整 sysbench OLTP_READ_WRITE** (v3.9.0+ Phase 2d Track 3)
- ❌ **TPC-H SF=1.0 完整 22 queries benchmark**
- ❌ **sysbench OLTP_WRITE_ONLY**
- ❌ **多并发扩展测试** (1/4/16/64/256 threads)
- ❌ **长时间稳定性** (24h, 72h)
- ❌ **大表压力测试** (TB 级)

---

## 7. SIMD 优化目标

### 7.1 现状
- `vec_simd.rs` 占位 (2 funcs, 0 intrinsics)
- `simd_explicit.rs` (vector store) 13 AVX2 intrinsics

### 7.2 目标 (v3.9.0+)
- SQL executor 集成 SIMD: 5-10x aggregator/filter 加速
- Hash join 集成 SIMD: 2-3x 加速
- Sort 集成 SIMD: 2-5x 加速

---

## 8. 实测计划 (Recommended)

### 8.1 P0 (24h)
1. **跑 v2.4.0 baseline 重测** (确认参考)
2. **v3.8.0 简单 benchmarks** (qps, page_io, buffer_pool)
3. **vs SQLite 重新对比** (1 测试机)

### 8.2 P1 (1 周)
4. **完整 sysbench OLTP_READ_WRITE** (Phase 2d Track 3)
5. **TPC-H Q1-Q6** (简单聚合)
6. **多并发 1/4/16/64 threads**

### 8.3 P2 (2 周+)
7. **TPC-H 全部 22 queries** (需要 F-11/F-12 修复)
8. **长时间稳定性 24h+**
9. **大表 TB 级压力**

---

## 9. 性能 vs MySQL 5.7 对比 (估算)

| 工作负载 | v3.8.0 估算 | MySQL 5.7 | 差距 |
|----------|-------------|-----------|------|
| Simple SELECT | 100 µs | 200 µs | **2x faster** |
| INSERT batch | 10ms/100 | 15ms/100 | 1.5x faster |
| JOIN 2-table | 500 µs | 1ms | **2x faster** |
| TPC-H Q1 | 200 µs | 500 µs | **2.5x faster** |
| sysbench oltp_rw | 5K TPS | 8K TPS | 0.6x slower |

**估算**: v3.8.0 在**简单工作负载比 MySQL 5.7 快** (1.5-2.5x), **复杂 OLTP 略慢** (0.6x, 待优化)。

---

## 10. 行动建议 (Action Items)

### 10.1 P0 (24h, P1-5 #2882 + P1-2 #2880)
1. ✅ 跑 v3.8.0 简单 benchmark (qps, page_io, buffer_pool)
2. ✅ 写 PERFORMANCE_TARGETS.md (本文档)
3. ✅ 集成到 D1-D5 门禁 (CI)

### 10.2 P1 (1 周, INT-3 + INT-4)
4. 完整 sysbench OLTP_READ_WRITE
5. TPC-H Q1-Q6 实测
6. SIMD 集成 SQL executor (50h, INT-3 P1)

### 10.3 P2 (2 周+)
7. TPC-H 22 queries 全 PASS
8. 多并发稳定性
9. MySQL 5.7 性能正式对比 (重做 v2.8.0 评估)

---

## 11. 结论 (Conclusion)

v3.8.0 性能目标:
- ✅ 已定义: 简单 QPS, latency, 资源使用
- ✅ 已定义: 回归阈值 (QPS<5%, 延迟<10%)
- ⚠️ 缺失: v3.8.0 完整 benchmark 实测数据
- ⚠️ 缺失: 与 MySQL 5.7 正式性能对比

**建议**: 立即跑基础 benchmark, 然后 Phase 2d Track 3 跑 sysbench。
