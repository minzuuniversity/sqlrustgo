# Issue #198: LIMIT Pushdown

## 描述
当前 LIMIT 在 `execute_select` 最后阶段才应用，此时 join 的全量结果已经被 materialised。

SF10 Q10 仅需 TOP 20，但完整 join 产生了数百万行中间结果。

## 改动
将 LIMIT 提前到 join 阶段：
- 在 `execute_join` 内部跟踪 emit 计数
- 当 matched 行数达到 LIMIT 时，停止 build/probe
- 使用 `num_buckets` 和 early-stop 逻辑

## 验收条件
- [ ] Q10 在 join 阶段提前截断
- [ ] SF0.1 TPC-H 22 查询全部通过（语义不变）
- [ ] 带 LIMIT 的查询内存峰值 < 无 LIMIT 的 50%

## 依赖
Issue #197（流式 Iterator）

## 优先级
P1（Phase 1）

## 状态
NOT STARTED

---

# Issue #199: 多线程并行 Scan

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

## 状态
NOT STARTED

---

# Issue #200: SIMD 向量化 Filter/Aggregate

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

## 状态
NOT STARTED

---

# Issue #201: TPC-H 回归基线测试

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

## 状态
NOT STARTED