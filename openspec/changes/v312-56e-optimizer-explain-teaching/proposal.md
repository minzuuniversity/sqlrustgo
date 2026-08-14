# Proposal — V312-56E: Optimizer/EXPLAIN 教学实验

## Why

Issue #4255: 用于教学的 EXPLAIN、统计信息、histogram、hash join、semi/anti join 和简单 CBO 选择实验未补齐。

## What Changes

### 1. EXPLAIN 输出

稳定输出:
- join type
- estimated rows
- chosen access path

### 2. 教学 SQL Fixture (至少5个)

展示不同计划选择:
- 全表扫描 vs 索引扫描
- 嵌套循环 join vs hash join
- 不同 WHERE 条件的计划变化

### 3. 统计信息与 CBO

- 统计信息更新前后计划变化可复现
- histogram 基础支持
- hash join / semi join / anti join 计划

### 4. 明确边界

不得把 heuristic plan 误写成成本模型严格正确。

## Capabilities

### New Capabilities

- **EXPLAIN 输出** - 结构化 plan dump
- **教学 SQL fixtures** - 展示不同计划选择
- **统计信息实验** - 可复现的计划变化

### Modified Capabilities

- 现有 optimizer → 添加 EXPLAIN 教学输出

## Non-goals

- 不实现完整的 CBO 成本模型
- 不实现多表 join  reorder 优化
- 不实现 auto analyze

## Acceptance Criteria

- [ ] EXPLAIN 或等价 plan dump 可稳定输出 join type、estimated rows、chosen access path
- [ ] 至少 5 个教学 SQL fixture 展示不同计划选择
- [ ] 统计信息更新前后计划变化可复现
- [ ] 不得把 heuristic plan 误写成成本模型严格正确
- [ ] 运行 `cargo test -p sqlrustgo-optimizer --all-features -- --nocapture` PASS
- [ ] 运行 `cargo test -p sqlrustgo-planner --all-features -- --nocapture` PASS

## Issue Reference

Issue #4255 (V312-56E)
