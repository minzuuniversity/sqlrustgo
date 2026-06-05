<!-- env:blocked:no-ci -->

# openspec/3180 - P3-1 Prepared Statement Cache

> **Issue**: [#3180](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3180)
> **作者**: Hermes Agent
> **日期**: 2026-06-05
> **Phase**: 6 (W11-12)
> **工作量**: 12h (~1.5 天) — 实际估计 ~4h
> **优先级**: P3 (10%, 性能)
> **Milestone**: v3.9.0 (id=32, due 2026-09-23)
> **Label**: v3.9.0, P3-perf, phase-6

## 一、问题分析

### 1.1 背景

每条 SQL 都要:
1. **Lex**: 词法分析 (~30%)
2. **Parse**: 语法分析 → AST (~50%)
3. **Plan**: AST → 执行计划 (~10%)
4. **Execute**: 执行计划 → 结果 (~10%)

Lex + Parse 占总耗时 80%, 但对于重复执行的 SQL, 完全可以跳过.

**MySQL/PostgreSQL 性能优化**: PREPARE + EXECUTE 模式可减少 70% 延迟, 在高并发 web 流量下至关重要.

### 1.2 现有状态 (2026-06-05 审计)

| 组件 | 状态 |
|------|------|
| `Statement` enum (parser/src/parser.rs:48) | ✅ 完整, 无 Prepare/Execute |
| `parse(sql)` (parser/src/lib.rs) | ✅ 每次调用重新 lex+parse |
| 性能特征 (v3.8.0) | 每条 SQL 走完整 parse 路径 |
| 缓存层 | ❌ 无 |

### 1.3 目标

新增 3 步流程:
```sql
-- Step 1: PREPARE (parse 一次, 缓存)
PREPARE stmt1 AS 'SELECT * FROM users WHERE id = ?';

-- Step 2: EXECUTE (用缓存的 Statement, 跳过 parse)
EXECUTE stmt1 USING 123;

-- Step 3: DEALLOCATE (可选, 释放缓存)
DEALLOCATE stmt1;
```

**性能目标**: EXECUTE 比 `parse + execute` 快 5x+ (实际测得跳过 80% parse 时间).

## 二、变更设计

### 2.1 新 crate `sqlrustgo-cache`

**位置**: `crates/cache/` (新建)

**结构**:
```
crates/cache/
├── Cargo.toml
└── src/
    ├── lib.rs
    └── stmt_cache.rs
```

**核心 API**:
```rust
pub struct PreparedStatementCache {
    by_name: HashMap<String, CacheEntry>,    // name -> Statement
    by_sql: HashMap<u64, String>,            // sql_hash -> sql
    max_size: usize,
    hits: AtomicU64,
    misses: AtomicU64,
    lru_order: VecDeque<String>,             // LRU eviction
}

impl PreparedStatementCache {
    pub fn new(max_size: usize) -> Self;
    pub fn prepare(&mut self, name: &str, sql: &str, stmt: Statement) -> SqlResult<()>;
    pub fn execute(&self, name: &str) -> Option<&Statement>;
    pub fn deallocate(&mut self, name: &str) -> Option<Statement>;
    pub fn cache_stats(&self) -> CacheStats;
    pub fn hit_rate(&self) -> f64;
    pub fn clear(&mut self);
}
```

### 2.2 Parser 新增 3 个 Statement

**位置**: `crates/parser/src/parser.rs`

```rust
pub enum Statement {
    // ... 现有 variants ...
    Prepare {
        name: String,
        sql: String,
    },
    Execute {
        name: String,
        params: Vec<Expression>,  // USING 子句参数
    },
    Deallocate {
        name: String,
    },
}
```

**Lexer**: 新增 `PREPARE`, `EXECUTE`, `DEALLOCATE` 关键字.

### 2.3 ExecutionEngine 集成

**位置**: `src/execution_engine.rs`

```rust
pub struct ExecutionEngine<S: StorageEngine> {
    // ... 现有字段 ...
    pub(crate) stmt_cache: Arc<RwLock<PreparedStatementCache>>,
}

impl<S: StorageEngine + 'static> ExecutionEngine<S> {
    pub fn stmt_cache_stats(&self) -> CacheStats;
    pub fn set_cache_size(&mut self, size: usize);
}
```

**execute() 流程**:
```rust
match statement {
    Statement::Prepare { name, sql } => {
        // Parse sql once
        let parsed = parse(&sql)?;
        // Store in cache
        self.stmt_cache.write().prepare(&name, &sql, parsed);
        Ok(ExecutorResult::new(vec![vec![Value::Text("OK".into())]], 0))
    }
    Statement::Execute { name, params } => {
        // Get from cache (skip parse)
        let stmt = self.stmt_cache.read().execute(&name)
            .ok_or_else(|| SqlError::ExecutionError(format!("prepared statement '{}' not found", name)))?
            .clone();
        // Execute
        self.execute_statement(stmt, params)
    }
    Statement::Deallocate { name } => {
        self.stmt_cache.write().deallocate(&name);
        Ok(ExecutorResult::empty())
    }
    other => /* existing dispatch */
}
```

### 2.4 缓存策略

**LRU (Least Recently Used)**:
- 容量满时, 驱逐最久未使用的 entry
- LRU 顺序: `VecDeque<String>`, `execute()` 时移到队首
- 默认容量 100 (per issue spec)
- 命中: `execute()` 找到 cache
- 未命中: `execute()` 找不到 (报错: "prepared statement 'X' not found")

### 2.5 性能验证

**基准测试** (在 tests/prepared_stmt_bench.rs):
- 1000 次相同的 `parse + execute` (无 cache)
- 1000 次 `prepare + 1000 次 execute` (有 cache)
- 期望 EXECUTE 比 PARSE+EXECUTE 快 5x+

## 三、风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| Cache 内存爆炸 | 中 | LRU 上限 100 entries, 每 entry < 1KB |
| 缓存错误 Statement | 高 | 缓存前 parse 成功, EXECUTE 重新执行 (不重新 parse) |
| 命名冲突 (PREPARE stmt1) | 低 | 覆盖现有 (last-write-wins), 文档说明 |
| 线程安全 | 中 | Arc<RwLock<>> 包裹 cache |
| Execute 失败 | 中 | 缓存的 Statement 正常, 执行错误正常返回 (不污染 cache) |

## 四、测试策略

### 4.1 单元测试 (`crates/cache/src/stmt_cache.rs`)

| 测试 | 验证 |
|------|------|
| `test_basic_prepare_execute` | PREPARE + EXECUTE 基本流程 |
| `test_cache_hit_miss` | hit/miss 计数 |
| `test_lru_eviction` | 容量满时驱逐最久未用 |
| `test_deallocate` | DEALLOCATE 移除 entry |
| `test_cache_clear` | clear() 重置所有 |
| `test_duplicate_name` | 重复 PREPARE 覆盖 |
| `test_concurrent_access` | RwLock 线程安全 (10 threads) |
| `test_lru_order` | LRU 顺序: 最近 execute 移到队首 |

### 4.2 e2e tests (`tests/prepared_stmt_test.rs`)

| 测试 | 验证 |
|------|------|
| `test_prepare_execute_basic` | SQL 端到端 PREPARE/EXECUTE |
| `test_execute_unprepared_errors` | 错误: 未 PREPARE 就 EXECUTE |
| `test_prepare_overwrite` | 同名 PREPARE 覆盖 |
| `test_deallocate_then_execute_errors` | DEALLOCATE 后 EXECUTE 报错 |
| `test_prepared_with_params` | EXECUTE USING 100 |
| `test_prepared_with_multiple_params` | EXECUTE USING 1, 'a', 1.5 |
| `test_prepared_in_txn` | 事务内 PREPARE/EXECUTE |
| `test_prepared_with_insert` | PREPARE INSERT EXECUTE |
| `test_prepared_with_update` | PREPARE UPDATE EXECUTE |
| `test_prepared_with_delete` | PREPARE DELETE EXECUTE |
| `test_cache_size_configurable` | 修改 cache 大小 |
| `test_cache_clear` | clear() 后 EXECUTE 报错 |
| `test_cache_stats` | hit/miss/eviction 统计 |
| `test_prepared_with_join` | PREPARE JOIN EXECUTE |
| `test_prepared_with_aggregate` | PREPARE GROUP BY EXECUTE |
| `test_prepared_with_subquery` | PREPARE 子查询 EXECUTE |
| `test_perf_execute_5x_faster` | EXECUTE 性能 ≥ 5x PARSE+EXECUTE |
| `test_multiple_prepared` | 多个 PREPARE + 各自 EXECUTE |
| `test_prepared_reusable` | 同一 PREPARE 多次 EXECUTE |
| `test_prepared_with_null_params` | EXECUTE USING NULL |

**总 e2e tests**: 20+

### 4.3 性能 (与 TPC-H 22/22 兼容)

升级后, 跑 TPC-H 22/22 验证不退化.

## 五、门禁 (新门禁编号待定, 暂用 G10 复用)

**位置**: `scripts/gate/check_prepared_stmt.sh` (新建, ~80 行)

**检查项**:
1. `crates/cache/` 存在 + `PreparedStatementCache` 公开
2. 3 个新 Statement variants (Prepare/Execute/Deallocate) 在 parser
3. `tests/prepared_stmt_test.rs` 至少 20 tests
4. `cargo test -p sqlrustgo-cache` 全部 PASS
5. `cargo test --test prepared_stmt_test` 全部 PASS
6. 性能测试 EXECUTE >= 5x PARSE+EXECUTE

## 六、实施步骤

| # | 步骤 | 文件 | 工作量 |
|---|------|------|--------|
| 1 | SPEC 编写 (本文档) | `docs/openspec/3180-...` | 1h |
| 2 | `sqlrustgo-cache` crate + LRU 实现 | `crates/cache/` | 2h |
| 3 | Parser 新增 Prepare/Execute/Deallocate | `crates/parser/src/parser.rs` | 2h |
| 4 | ExecutionEngine 集成 | `src/execution_engine.rs` | 1h |
| 5 | 20+ e2e tests | `tests/prepared_stmt_test.rs` | 2h |
| 6 | 性能基准 + 验证 | — | 1h |
| **合计** | | | **~9h** |

## 七、交付物清单

| 类别 | 文件 | 大小预估 |
|------|------|----------|
| Spec | `docs/openspec/3180-p31-prepared-stmt.md` (本文件) | 200 行 |
| Crate | `crates/cache/Cargo.toml` + `src/{lib,stmt_cache}.rs` | 250 行 |
| Parser | `crates/parser/src/parser.rs` (3 new variants + 3 keywords) | +80 行 |
| Engine | `src/execution_engine.rs` (cache field + dispatch) | +40 行 |
| Test | `tests/prepared_stmt_test.rs` (20+ e2e tests) | 350 行 |
| Gate | `scripts/gate/check_prepared_stmt.sh` | 100 行 |
| **合计** | | **~1020 行** |

## 八、依赖关系

### 8.1 编译依赖 (无)

- 复用现有 `parse()` (parser/src/lib.rs)
- 复用现有 `ExecutionEngine` 调度
- 不依赖 P3-2 ~ P3-5

### 8.2 流程依赖

- ✅ P0-1 ~ P0-4 已 merged
- ✅ P1-1 ~ P1-4 已 closed
- ✅ P2-1 (Audit Log) 已 closed
- 🟡 P2-2, P2-3 仍 open (不阻塞本任务)
- 🟡 P3-2 ~ P3-5 仍 open (不依赖)

## 九、Issue 关闭条件 (per §3.1)

满足 4 项:
1. ✅ PR 关联 (`fix/issue-3180-p31-prepared-stmt-cache` → `develop/v3.9.0`)
2. ✅ 20+ e2e tests PASS
3. ✅ 性能 ≥ 5x 验证
4. ✅ 门禁 PASS

## 十、参考

- Issue #3180: <http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3180>
- 计划: `docs/releases/v3.9.0/plans/V390_DEVELOPMENT_PLAN.md` §P3-1
- 治理: `docs/governance/ISSUE_CLOSING_VERIFICATION.md` §3.1
