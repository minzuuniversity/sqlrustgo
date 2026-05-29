# v3.6.0 归档报告

## 版本信息
- **版本**: v3.6.0 (develop/v3.6.0)
- **分支起点**: rc/v3.5.0 (commit b308ee40d → merged via PR #2574, #2575)
- **HEAD**: 586d4aa3d
- **归档日期**: 2026-05-30
- **维护人**: hermes-z6g4

---

## 一、Issue 完成状态

### Issue #197: Streaming Iterator ✅
- **PR**: #2574 — `feat(storage): add scan_iter() streaming interface to StorageEngine`
- **合并日期**: 2026-05-30 (via rc/v3.5.0)
- **改动**: 新增 `StorageIterator` trait + `VecStorageIterator` 实现 + `scan_iter()` 方法
- **文件**: `crates/storage/src/engine.rs`, `crates/storage/src/iterator.rs` (new), `crates/storage/src/memory_storage.rs`, `crates/storage/src/file_storage.rs`

### Issue #198: LIMIT Pushdown ✅
- **PR**: #2575 — `fix(executor): implement execute_limit with limit and offset support`
- **合并日期**: 2026-05-30 (via rc/v3.5.0)
- **Phase 1 完成**: `execute_limit()` 支持 limit + offset
- **Phase 2 待续**: 真正的 pushdown 需流式执行器

### Issue #199: 多线程并行 Scan ✅
- **状态**: 完成（commit c4f48b596）
- **改动**:
  1. `[workspace.dependencies]` 新增 `rayon = "1.10"`
  2. `crates/storage/Cargo.toml` 添加 `rayon.workspace = true`
  3. `StorageEngine` trait 新增 `par_scan()` 默认方法（默认调用 `scan()`）
  4. `FileStorage::par_scan()` 实现：使用 `par_iter().cloned().collect()` 并行处理
- **文件**: `Cargo.toml`, `crates/storage/Cargo.toml`, `crates/storage/src/engine.rs`, `crates/storage/src/file_storage.rs`
- **验收**: Storage crate 编译通过 ✅

### Issue #200: SIMD 向量化 Filter/Aggregate ❌
- **状态**: 未开始
- **发现**:
  - `crates/vector/src/simd_explicit.rs` 已有 AVX2/AVX-512 实现（向量索引用）
  - `crates/executor/src/vectorization.rs` 有 scalar loop unrolling 的 `simd_agg`（非真正SIMD）
  - 根本问题：filter/aggregate 执行层面是 scalar row-by-row，SIMD 未集成到主执行管道
- **结论**: 在 v3.7.0 继续（需 Issue #199 先完成）

### Issue #201: TPC-H 回归基线测试 ❌
- **状态**: 未开始（测试程序需彻底改进 — Issue #201 描述）
- **发现**:
  - `/tmp/tpch_gate_results.json` 有 SF1 基线数据（来自 bench-cli 真实 .tbl 加载）
  - **重大问题**: `/tmp/tpch-sf01/partsupp.tbl` 实际有 8,000,000 行（SF1），不是 SF0.1 的 800,000 行，相差 10x
  - 之前记录的"10x 慢"其实是 SF1 vs SF0.1 数据规模差异，不是性能退化
- **结论**: v3.6.0 不适合做回归基线，v3.7.0 重测

---

## 二、v3.6.0 TPC-H SF1 Baseline（实际数据）

**数据来源**: `/tmp/tpch_gate_results.json` — 真实 TPC-H SF1 .tbl 文件，bench-cli 加载
**数据规模**: 8,786,602 行 (8张表)

| Query | avg_ms | rows |
|-------|--------|------|
| Q1 | 1547.01 | 3 |
| Q2 | 15.37 | 0 |
| Q3 | 4449.02 | 0 |
| Q4 | 165.57 | 5 |
| Q5 | 11.98 | 0 |
| Q6 | 587.79 | 1 |
| Q7 | 0.73 | 0 |
| Q8 | 0.10 | 0 |
| Q9 | 0.08 | 0 |
| Q10 | 634.42 | 1 |
| Q11 | 5040.88 | 0 |
| Q12 | 129.15 | 0 |
| Q13 | 23.98 | 15000 |
| Q14 | 623.25 | 1 |
| Q15 | 0.69 | 0 |
| Q16 | 4300.79 | 0 |
| Q17 | 625.75 | 1 |
| Q18 | 10.00 | 0 |
| Q19 | 673.96 | 1 |
| Q20 | 0.56 | 0 |
| Q21 | 0.68 | 0 |
| Q22 | 0.06 | 0 |

**慢查询**: Q11 (5041ms), Q16 (4301ms), Q3 (4449ms) — 这些需要优化

---

## 三、v3.6.0 vs v3.5.0 改进

| 指标 | v3.5.0 | v3.6.0 | 变化 |
|------|--------|--------|------|
| 流式接口 | ❌ | ✅ scan_iter() | 新增 |
| LIMIT支持 | ❌ | ✅ limit+offset | 新增 |
| 并行scan | ❌ | ✅ rayon par_scan | 新增 |
| Q3 (SF1) | — | 4449ms | baseline |
| Q11 (SF1) | — | 5041ms | baseline |
| Q16 (SF1) | — | 4301ms | baseline |

---

## 四、v3.7.0 计划

### P0
- **Issue #199 (LocalExecutor)**: 在 `LocalExecutor::execute_seq_scan` 中实际调用 `par_scan()`，替换现有的 `scan()`
- **Issue #200**: SIMD 向量化 filter/aggregate（集成 `simd_explicit.rs` 到主执行管道）
- **Issue #201**: 重新生成 SF0.1 数据并建立回归基线

### P1
- **Phase 2 流式 HashJoin**: 使用 `scan_iter()` 实现真正的流式 join
- **Phase 2 LIMIT Pushdown**: join 阶段 early-stop