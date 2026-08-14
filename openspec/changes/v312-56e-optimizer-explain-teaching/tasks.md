# Tasks — V312-56E: Optimizer/EXPLAIN 教学实验

## Phase 1: 现状调研 ✅

- [x] 1.1 EXPLAIN 实现 - **已完整** (`crates/executor/src/explain.rs`)
  - 支持 Tree 和 Traditional 格式
  - 输出 join type, estimated rows, access path
  - 支持: SeqScan, IndexScan, Projection, Filter, HashJoin, SortMergeJoin, Aggregate, Sort, Limit, SetOperation, Window
- [x] 1.2 EXPLAIN 变体 - **Tree format 已实现**
- [x] 1.3 单元测试 - **已存在** (`explain.rs` 中的 #[cfg(test)])

## Phase 2: 缺失项

- [ ] 2.1 创建 teaching fixtures 展示不同计划选择
  - [ ] 2.1.1 全表扫描 vs 索引扫描 fixture
  - [ ] 2.1.2 hash join vs 嵌套循环 join fixture
  - [ ] 2.1.3 semi/anti join fixture
  - [ ] 2.1.4 aggregate group by fixture
  - [ ] 2.1.5 ORDER BY + LIMIT fixture
- [ ] 2.2 ANALYZE TABLE 命令测试
- [ ] 2.3 统计信息更新前后计划变化测试

## Phase 3: 文档边界说明

- [ ] 3.1 明确哪些是 heuristic 哪些是 cost-based
- [ ] 3.2 文档说明 heuristic 不等于 cost model 严格正确

## Phase 4: 验证

- [ ] 4.1 `cargo test -p sqlrustgo-executor --lib explain -- --nocapture` PASS
- [ ] 4.2 `cargo test -p sqlrustgo-optimizer --all-features -- --nocapture` PASS
- [ ] 4.3 `cargo test -p sqlrustgo-planner --all-features -- --nocapture` PASS

## Acceptance Criteria

- [x] EXPLAIN 可稳定输出 join type、estimated rows、chosen access path
- [ ] 至少 5 个教学 SQL fixture 展示不同计划选择
- [ ] 统计信息更新前后计划变化可复现
- [ ] 不得把 heuristic plan 误写成成本模型严格正确
