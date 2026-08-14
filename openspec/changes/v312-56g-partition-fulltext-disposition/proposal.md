# Proposal — V312-56G: Partition/FullText disposition 与 GMP keyword retrieval 决策

## Why

Issue #4257: Partition 和 FullText 是 4.0.0 生产能力、v3.12 受控教学/GMP keyword retrieval 能力，还是延期项，未明确。

## What Changes

### 1. Partition disposition

- `PartitionInfo` / storage-level tests 与 SQL `ALTER TABLE SET PARTITIONED BY` unsupported 状态不再冲突
- 若进入 v3.12，明确 unsupported error
- 若延期，明确 DEFERRED 状态

### 2. FullText disposition

- FullTextIndex / storage-level tests 与 SQL MATCH/AGAINST 或 GMP keyword retrieval 需求有明确关系
- 若 GMP 需要关键词检索，定义受控 FullText 子集和 fixture
- 若不进入 v3.12，README 和 release docs 明确 `DEFERRED`

### 3. GMP keyword retrieval 决策

明确:
- GMP 是否需要 keyword retrieval
- FullText 是否服务于 GMP
- 决策记录在文档中

## Capabilities

### New Capabilities

- **Partition 明确状态** - SUPPORTED/UNSUPPORTED/DEFERRED
- **FullText 明确状态** - 与 GMP keyword retrieval 关联
- **决策文档** - 明确边界和 owner

### Modified Capabilities

- 现有 Partition 实现 → 明确 unsupported 边界
- 现有 FullText 实现 → 与 GMP 关联

## Non-goals

- 不实现完整的 Partition 功能
- 不实现通用的 FullText search

## Acceptance Criteria

- [ ] PartitionInfo/storage-level tests 与 SQL ALTER TABLE SET PARTITIONED BY unsupported 状态不再冲突
- [ ] FullTextIndex/storage-level tests 与 SQL MATCH/AGAINST 或 GMP keyword retrieval 需求有明确关系
- [ ] 若 GMP 需要关键词检索，定义受控 FullText 子集和 fixture
- [ ] 若不进入 v3.12，README 和 release docs 明确 DEFERRED
- [ ] 运行 `cargo test -p sqlrustgo-storage fulltext -- --nocapture` PASS
- [ ] 运行 `cargo test --test partition_e2e_test -- --nocapture` PASS
- [ ] 运行 `bash scripts/gate/check_docs_consistency.sh` PASS

## Issue Reference

Issue #4257 (V312-56G)
