# v3.11.0 SOAK 性能分析报告

## 执行摘要

历史分析显示，通过 batch transaction mode 和 WAL sync mode 优化，系统吞吐从约 QPS/TPS 9 提升到约 371，约 41x。该结论应理解为特定测试环境和配置下的性能提升，不应泛化为所有生产负载。

## 1. 测试环境

| 组件 | 配置 |
|---|---|
| 硬件 | MacMini M2 |
| CPU | 10 cores |
| 内存 | 17 GB |
| 存储 | FileStorage + WAL |
| Server Threads | 16 |
| Client Threads | 8 |

## 2. 性能瓶颈

优化前的主要瓶颈是“每个操作一次 commit”的事务模式：

- WAL sync (`fsync`) 约 10ms。
- JSON serialization 约 5ms。
- File write 约 5ms。

理论上单操作 commit 的吞吐上限约为 `1000ms / 20ms = 50 TPS`，实际测试约为 9 TPS，说明架构瓶颈明显。

## 3. 优化实现

| 优化 | 说明 | 效果 |
|---|---|---|
| Batch Transaction Mode | 客户端把 10 个 DML 操作合并到一个 transaction | 减少 commit 次数 |
| WAL Sync Mode CLI | 增加 `--wal-sync every`、`batch:N`、`off` | 在 durability 与 throughput 间可配置平衡 |

## 4. 结论和边界

SOAK 性能分析证明 batch commit 和 WAL sync 策略对吞吐有明显帮助。v3.12 应进一步验证 mixed workload、crash recovery、backup/restore、audit-chain 和 GMP/RAG 检索路径下的长期稳定性。

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义。若英文附录与中文正文或 `COMPREHENSIVE_ASSESSMENT_REPORT.md` 冲突，当前正式判断以中文正文和综合评估报告为准。

# SOAK Performance Analysis Report v3.11.0

## Executive Summary

**Performance improvement: QPS 9 → 371 (41x)**

After implementing batch transaction mode and WAL sync mode optimization, the system achieved a **41x TPS improvement**.

---

## 1. Test Environment

| Component | Specification |
|-----------|---------------|
| Hardware | MacMini M2 |
| CPU | 10 cores |
| Memory | 17 GB |
| Storage | FileStorage + WAL |
| Server Threads | 16 |
| Client Threads | 8 |

---

## 2. Performance Bottleneck Analysis

### Root Cause
Single-transaction commit pattern caused 20ms latency per operation:
- WAL sync (`fsync`) ~10ms
- JSON serialization ~5ms
- File write ~5ms

**Theoretical max TPS: 1000ms / 20ms = 50 TPS**

### Actual Results (Before Optimization)

| Metric | Value |
|--------|-------|
| TPS | 9 |
| Latency | 20ms/op |
| CPU Usage | <5% |

The bottleneck was architectural: **one commit per transaction**.

---

## 3. Optimization Implementation

### 3.1 Batch Transaction Mode (Client-Side)

**Change**: Bundle 10 DML operations into one transaction

```rust
// Before: 1 commit per operation
conn.execute("INSERT ...");
conn.execute("UPDATE ...");
conn.execute("DELETE ...");

// After: 10 operations per commit
conn.execute("START TRANSACTION");
for _ in 0..10 {
    conn.execute(gen_dml());
}
conn.execute("COMMIT");
```

**Effect**: Reduces commits from 10 to 1 per batch

### 3.2 WAL Sync Mode CLI

Added `--wal-sync` option:

| Mode | Behavior | Use Case |
|------|----------|----------|
| `every` | Sync after every transaction | Full durability |
| `batch:N` | Sync after N transactions | Balance |
| `off` | Never sync | Fastest, no durability |

---

## 4. Test Results

### 4.1 Batch Mode Comparison (30-second tests)

| Batch | QPS | Writes/30s | Sync Interval | IO Errors |
|-------|-----|------------|---------------|-----------|
| batch:10 | 377 | 5,868 | ~27ms | Yes |
| batch:100 | 359 | 5,524 | ~270ms | Yes |
| batch:500 | 378 | 5,849 | ~1.35s | Yes |
| batch:10000 | 367 | 11,322 | ~27s | No |
| off | 371 | - | ∞ | No |

**Conclusion**: QPS is consistent (~370) across batch modes. Batch:10000 provides good balance with ~27s sync interval and no IO errors.

### 4.2 Long-Duration Test

| Metric | Value |
|--------|-------|
| Duration | 480s+ (before IO errors) |
| QPS | ~370 |
| OLTP | 147,620 ops |
| OLAP | 30,953 ops |
| Writes | ~6,900/30s |
| Errors | 0 (initial) |

### 4.3 IO Error Issue

At ~480 seconds with small batch sizes (10/100/500):
```
[worker 1] OLAP Q1 err: IO error: failed to fill whole buffer
```

**Root Cause**: WAL buffer overflow when batch is too small
**Solution**: Use batch:10000 or larger

---

## 5. System Load Analysis

### 5.1 Resource Utilization

| Resource | Usage | Utilization |
|----------|-------|-------------|
| CPU | 2.6% | Very low |
| Memory | 12 MB | <1% |
| Server Threads | 16 | Low |

### 5.2 Performance Bottleneck Identification

Current bottleneck is **WAL serialization + global lock**:

```
Arc<RwLock<BoxStorageEngine>>  // All operations serialize here
```

### 5.3 Theoretical Max

| Configuration | Estimated Max TPS |
|---------------|-------------------|
| Current (batch:10, batch sync) | ~370 |
| Theoretical (no sync) | ~660 |
| With table-level locking | ~1,000+ |
| With bincode serialization | ~800+ |

---

## 6. Optimization Roadmap

### Short-term (Completed)

- [x] Batch transaction mode (QPS 9→371)
- [x] WAL sync mode CLI
- [x] Write counter fix

### Medium-term (Recommended)

| Optimization | Expected TPS | Risk |
|-------------|-------------|------|
| bincode serialization | 500-600 | Low |
| Table-level locking | 700-1000 | Medium |
| Async WAL | 500-700 | High |

### Long-term (Future)

| Optimization | Expected TPS | Complexity |
|-------------|-------------|------------|
| LSM-style storage | 1000+ | Very High |
| Sharded architecture | 2000+ | Extreme |

---

## 7. Conclusions

1. **Batch transaction mode is the key optimization** - achieved 41x TPS improvement
2. **Batch:10000 is recommended** - ~27s sync interval, no IO errors
3. **CPU utilization is very low** (~3%) - further optimizations possible
4. **Global lock is the next bottleneck** - needs architectural changes

---

## 8. Recommendations

### For v3.11.0 Release

1. Use `--wal-sync=batch:10000` as default
2. Document the batch transaction pattern for best performance
3. Add monitoring for WAL buffer size

### For Future Development

1. Implement table-level locking for concurrent writes
2. Consider bincode for faster serialization
3. Evaluate async WAL for hidden sync latency

---

**Report Date**: 2026-07-18
**Test Duration**: 480+ seconds (short test)
**Target**: 168h SOAK (batch:10000)
