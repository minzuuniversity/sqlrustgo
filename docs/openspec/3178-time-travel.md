# openspec/3178 - P2-2 Time Travel Query (AS OF TIMESTAMP)

> **Issue**: #3178
> **作者**: Hermes Agent
> **日期**: 2026-06-05
> **Phase**: 5 (W9-10)
> **工作量**: 24h (按 V390_DEVELOPMENT_PLAN)
> **状态**: 借力 crates/transaction/src/{mvcc,version_chain}.rs 已存在, 本次做 harness + G10 gate

## 一、问题分析

### 1.1 现状审计 (2026-06-05)

仓库已有完整 MVCC 基础设施:
- `crates/transaction/src/mvcc.rs`:
  - `TxId`, `Transaction`, `Snapshot`, `RowVersion`
  - `MvccEngine::begin/commit/abort/create_snapshot`
  - `Snapshot::new`, `is_visible`, `refresh_for_read_committed`
  - `Snapshot::new_read_committed` (timestamp-based)
- `crates/transaction/src/version_chain.rs` (363 lines):
  - `VersionChainMap::find_visible(key, snapshot)` — 直接 AS OF 实现
  - `VersionChainMap::append/get_chain/commit_versions/rollback_versions/gc`
  - `RowVersion::new/new_deleted/commit/mark_deleted/is_visible`

### 1.2 #3178 SQL 接口映射

| #3178 接口 | 已有覆盖 | 缺口 |
|------------|----------|------|
| `AS OF TIMESTAMP` 语法解析 | ❌ 缺 (parser 没用) | **完全缺** |
| `VERSIONS BETWEEN t1 AND t2` 语法 | ❌ 缺 | **完全缺** |
| MVCC 快照选择逻辑 | Snapshot::new ✅ | SQL→Snapshot 映射缺 |
| 旧版本页加载 | VersionChainMap::find_visible ✅ | SQL→find_visible 路由缺 |
| 20+ tests | ❌ | **完全缺** |

### 1.3 P2-2 任务真正需要补的 (按治理最小修改)

**A. Test Harness** (新):
- Mock MVCC engine + version chain
- SQL 解析 → Snapshot 转换

**B. 20+ Tests** (新):
- 5 类场景: basic / range / no_match / multi_table / edge_cases

**C. G10 Gate** (新):
- 7 项检查

## 二、实施方案

### 2.1 范围限定

按治理 §2.1 最小修改 + 复用现有 MVCC 基础设施:

**本次 PR 范围 (4 大块)**:

1. **新文件**: `tests/time_travel_harness.rs` (共享 helper, 8 self-tests)
2. **新文件**: `tests/time_travel_test.rs` (20+ tests, 5 类别)
3. **新文件**: `scripts/gate/check_p22_time_travel.sh` (G10 gate)
4. **新文件**: `docs/openspec/3178-time-travel.md` (本文件)

**延后 (推 v3.10+)**:
- 真实 SQL 解析器集成 (`AS OF TIMESTAMP` keyword)
- 执行引擎集成 (DQL routing)
- 性能优化 (large version chain 缓存)

### 2.2 Time Travel Harness 设计

```rust
// tests/time_travel_harness.rs (shared)
pub struct MockMvccEngine {
    engine: MvccEngine,
    chains: VersionChainMap,
    rows: HashMap<String, Vec<u8>>, // table -> latest value
}

pub struct TimeTravelQuery {
    pub table: String,
    pub key: String,
    pub as_of_timestamp: i64,
    pub versions_between: Option<(i64, i64)>, // (start, end) for VERSIONS BETWEEN
}

pub struct TimeTravelResult {
    pub query: TimeTravelQuery,
    pub found: bool,
    pub value: Option<Vec<u8>>,
    pub version_count: usize,
    pub first_visible_ts: Option<i64>,
    pub last_visible_ts: Option<i64>,
}

pub fn run_time_travel(engine: &mut MockMvccEngine, query: TimeTravelQuery) -> TimeTravelResult;
```

### 2.3 20+ Tests (5 类别)

| 类别 | Count | 示例 |
|------|-------|------|
| 1. basic (AS OF) | 5 | 1 row, 5 rows, NULL handling, before-insert, after-delete |
| 2. range (VERSIONS BETWEEN) | 4 | 1-day range, 1-week range, empty range, full range |
| 3. no_match | 3 | 未来时间, 非常早时间, 不存在 key |
| 4. multi_table | 4 | 2 表独立, JOIN 跨表, 同 key 不同表 |
| 5. edge_cases | 4 | 同 ts insert+update, 大量版本 (100+), deleted+restored, gc 后 |
| **TOTAL** | **20** | |

### 2.4 G10 Gate (7 checks)

1. `tests/time_travel_harness.rs` exists
2. `tests/time_travel_test.rs` exists + registered
3. 5 类别全覆盖 (grep test_ names)
4. cargo check pass
5. ≥20 tests pass
6. 借力 crates/transaction/src/{mvcc,version_chain}.rs (no regression)
7. TPC-H 22/22 (G1 维持)

## 三、风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| 真实 SQL parser 集成破坏 TPC-H | 22/22 失败 | 不动主 SQL 解析器, 仅 harness 层 |
| MVCC Engine 复杂 | 测试难写 | 单元级 (mocked engine) |
| Snapshot timestamp 用 i64 vs u64 | 类型错误 | 一致用 i64 (i64 包容 u64 在合理范围) |
| VersionChain gc 影响 | 测试 flaky | 不用 gc (本 PR 范围) |

## 四、验收标准 (G10 门禁)

```
✅ time_travel_test: ≥20 tests PASS
✅ 5 类别全覆盖
✅ G10 gate: 7/7 PASS
✅ 1555 L1 tests 不回归
✅ TPC-H 22/22 (G1 维持)
```

## 五、Subsumed Issues

- #3178 本身 (本任务)
- 与 P2-3 (#3179 Hash Chain) 互补 (时间戳排序 + 不可篡改链)

## 六、回滚计划

如 time_travel_test 编译失败:
1. 删除 `tests/time_travel*.rs`
2. G10 gate 标记 DEFER
3. 现有 mvcc/version_chain 保留

## 七、依赖

**上游**: P2-1 Audit Log (#3177 closed)
**下游**: P2-3 Hash Chain (#3179 in progress)

## 八、参考资料

- Issue #3178
- V390_DEVELOPMENT_PLAN.md §P2-2
- V390_TEST_PLAN.md §G10
- crates/transaction/src/mvcc.rs (Snapshot, MvccEngine)
- crates/transaction/src/version_chain.rs (363 lines, VersionChainMap.find_visible)
- crates/transaction/src/lib.rs (re-exports)
- P2-1 #3177 audit_log_harness (设计模型)
- P1-2 #3174 crash_test_harness (设计模型)
