# Issue #197: Streaming Iterator Infrastructure

## 状态
✅ **DONE** - 已合并到 rc/v3.5.0 (PR #2574)

## 描述
当前 `LocalExecutor` 所有算子都采用物化模式：子节点结果全量加载到内存后父节点才执行。

## 改动
1. 新增 `StorageIterator` trait 和 `VecStorageIterator` 实现 (`crates/storage/src/iterator.rs`)
2. `StorageEngine` trait 新增 `scan_iter()` 方法
3. `MemoryStorage::scan_iter()` 和 `FileStorage::scan_iter()` 实现
4. 新增 `IteratorSchema` 避免 cyclic dependency

## 验收条件
- [x] `StorageIterator` trait 定义完成
- [x] `MemoryStorage::scan_iter()` 实现完成
- [x] `FileStorage::scan_iter()` 实现完成
- [x] 现有 `scan()` 测试通过（向后兼容）

## PR
- PR #2574: `feat(storage): add scan_iter() streaming interface to StorageEngine`

---

# Issue #198: LIMIT Pushdown

## 状态
🚧 **IN PROGRESS** (Phase 1 done - basic limit execution)

## 描述
当前 LIMIT 在 `execute_select` 最后阶段才应用，此时 join 的全量结果已经被 materialised。

SF10 Q10 仅需 TOP 20，但完整 join 产生了数百万行中间结果。

## Phase 1: Basic LIMIT Execution (✅ Done)
已在 `execute_limit()` 中实现 limit 和 offset 支持：
- 从 `LimitExec` plan node 提取 limit 和 offset
- 对子节点结果应用 skip/take 切片

## Phase 2: LIMIT Pushdown to Join (TODO)
真正的 LIMIT pushdown 需要流式执行：
- 在 `execute_hash_join` 内部跟踪 emit 计数
- 当 emit 达到 LIMIT 时停止 build/probe
- 使用 `scan_iter()` 流式扫描避免全量加载

## 验收条件
- [x] Phase 1: `execute_limit()` 正确应用 limit 和 offset
- [ ] Phase 2: Q10 在 join 阶段提前截断
- [ ] SF0.1 TPC-H 22 查询全部通过（语义不变）
- [ ] 带 LIMIT 的查询内存峰值 < 无 LIMIT 的 50%

## PR
- PR #2575: `fix(executor): implement execute_limit with limit and offset support`

---

# Issue #199: 多线程并行 Scan

## 状态
NOT STARTED

## 描述
`storage.scan()` 当前单线程串行读文件。SF10 导入 85M 行花了 233 秒，I/O bound。

## 改动
使用 `rayon` 的 `ParallelIterator` 并行读文件分块：
- 将文件分成 N 个 chunk（按行数或字节数）
- 每个 chunk 由独立线程解析
- 结果合并到统一的 Vec

## 验收条件
- [ ] `storage.scan()` 支持并行读取（可选 feature flag）
- [ ] SF10 导入时间 < 120 秒（当前 233 秒）
- [ ] 并行 scan 正确性验证（逐行对比）

## 依赖
无

## 优先级
P1（Phase 2）

---

# Issue #200: SIMD 向量化 Filter/Aggregate

## 状态
NOT STARTED

## 描述
当前 filter/aggregate 逐行处理，未利用 SIMD 指令。一次比较 4-8 个 u64 值可大幅加速。

## 改动
使用 `portable_simd` crate 实现向量化：
- Filter: 批量比较列值，返回 BitMask
- Aggregate: 向量加法/比较

注意：`std::simd` 需要 Rust nightly，`portable_simd` 是更安全的选择。

## 验收条件
- [ ] Filter 算子使用 SIMD 向量化
- [ ] Aggregate (SUM/COUNT/AVG) 使用 SIMD
- [ ] SF0.1 Q1 性能提升 > 20%

## 依赖
Issue #199（多线程 scan）

## 优先级
P2（Phase 3）

---

# Issue #201: TPC-H 回归基线测试

## 状态
NOT STARTED

## 描述
目前没有 TPC-H 自动化回归测试。每次 PR 不知道是否引入性能退化。

## 改动
创建 `scripts/gate/check_tpch.sh`：
- 运行 SF0.1 TPC-H 22 查询
- 验证所有查询返回结果（非错误）
- 与基线时间对比（可选：性能退化 > 20% 则 FAIL）

## 验收条件
- [ ] `check_tpch.sh` 在 CI 中运行
- [ ] 所有 22 查询通过
- [ ] 基线结果保存在 `docs/releases/v3.6.0/TPCH_BASELINE.md`

## 依赖
至少 Issue #196 + #199 完成

## 优先级
P1（Phase 2）