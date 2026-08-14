# Tasks — V312-56G: Partition/FullText disposition 与 GMP keyword retrieval 决策

## Phase 1: 现状调研 ✅

### 1.1 FullText Index ✅
- [x] `FullTextIndex` 实现在 `crates/storage/src/bplus_tree/index.rs`
  - 支持 `FullTextIndex::tokenize()` 分词
  - 支持 `FullTextIndex::intersect()` 交集搜索
- [x] Parser 支持 `Token::Fulltext` 和 `CREATE FULLTEXT INDEX` 解析
- [x] 测试存在于 `tests/integration/sql/parser_e2e_test.rs`

### 1.2 Partition ✅
- [x] **MySQL TABLE PARTITION BY (RANGE/LIST/HASH) 未实现**
- [x] 现有的 `partition_scan` 是并行执行数据分区，不是 MySQL 语法
- [x] `HashPartitioner` 是 vector sharding，不是 table partitioning
- [x] `AlterTableOperation::SetPartitionedBy` 存在于 sql-corpus (parser 层)，但执行层未实现

### 1.3 MATCH/AGAINST ❌
- [x] Parser 可以解析 `CREATE FULLTEXT INDEX`
- [x] **MATCH (col) AGAINST ('keyword') 在 SQL 执行层未实现**

## Phase 2: 决策

### 2.1 Partition 决策
- [ ] 2.1.1 标为 UNSUPPORTED/DEFERRED
- [ ] 2.1.2 更新 README.md
- [ ] 2.1.3 更新 MYSQL_COMPAT_STATUS.md

### 2.2 FullText 决策
- [ ] 2.2.1 评估 MATCH/AGAINST 实现工作量
- [ ] 2.2.2 若 v3.12 不进，明确 DEFERRED
- [ ] 2.2.3 若进，创建 MATCH/AGAINST 执行路径 + teaching fixture

### 2.3 GMP keyword retrieval
- [ ] 2.3.1 评估 FullText 是否服务于 GMP
- [ ] 2.3.2 决策记录

## Phase 3: 测试验证

- [ ] 3.1 `cargo test -p sqlrustgo-storage --lib -- fulltext --nocapture` PASS
- [ ] 3.2 `cargo test -p sqlrustgo-parser --lib -- fulltext --nocapture` PASS
- [ ] 3.3 `bash scripts/gate/check_docs_consistency.sh` PASS

## Acceptance Criteria

- [ ] PartitionInfo 与 ALTER TABLE SET PARTITIONED BY unsupported 状态不再冲突
- [ ] FullTextIndex 与 SQL MATCH/AGAINST 有明确关系
- [ ] 若 GMP 需要关键词检索，定义受控 FullText 子集和 fixture
- [ ] 若不进入 v3.12，README 和 release docs 明确 DEFERRED
