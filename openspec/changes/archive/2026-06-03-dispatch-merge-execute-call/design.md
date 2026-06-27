# Design: LocalExecutorDml MERGE dispatch

## Context

### 背景
G3 是 P0 任务的最后一步。前置 G1 (WAL), G2 (Parser), G4 (engine field) 均已完成。G3 的目标是将这些能力串起来：让 `LocalExecutorDml` 真正调用 `MergeExecutor::execute_merge()`。

### 现状
- 活跃文件 `local_executor_dml.rs` (45 行 placeholder)
- `LocalExecutorDml` 有 `engine: Arc<Mutex<dyn ExecutionEngine>>` 字段（G4 添加）
- 没有 `storage` 字段，没有 `execute_dml` 方法，没有 MERGE 处理

### 约束
- 仅修改活跃文件（orphan `local_executor.rs` 不在编译路径）
- `MergeExecutor::execute_merge` 接受 `sqlrustgo_planner::MergeStatement`（不是 parser 的）
- 需要做 type 转换（lossy）

## Goals / Non-Goals

### Goals
- ✅ `LocalExecutorDml` 添加 storage 字段
- ✅ 添加 `execute_dml` 方法检测 MERGE
- ✅ 调用 `MergeExecutor::execute_merge`
- ✅ 测试覆盖

### Non-Goals
- ❌ 不实现完整 DML 路径（INSERT/UPDATE/DELETE）
- ❌ 不修改 `MergeExecutor::execute_merge`
- ❌ 不清理 orphan `local_executor.rs`
- ❌ 不实现生产 caller（REPL/server 集成）

## Decisions

### Decision 1: 添加 storage 字段到 `LocalExecutorDml`

**选择**:
```rust
pub struct LocalExecutorDml {
    engine: Arc<Mutex<dyn ExecutionEngine>>,
    storage: Arc<RwLock<dyn StorageEngine>>,
}
```

**理由**:
- `MergeExecutor::new` 需要 owned `Arc<RwLock<dyn StorageEngine>>`
- 当前结构无 storage，必须添加

**备选**:
- ❌ 在 `execute_dml` 参数中传入 storage：需要重构所有调用方
- ❌ 借用 storage：与 `MergeExecutor::new` 签名不兼容

### Decision 2: 新增 `new_with_storage` 构造函数

**选择**:
```rust
impl LocalExecutorDml {
    pub fn new() -> Self {
        Self::new_with_storage(
            Arc::new(RwLock::new(MemoryStorage::new())),
            Arc::new(Mutex::new(NoopExecutionEngine)),
        )
    }
    pub fn new_with_storage(
        storage: Arc<RwLock<dyn StorageEngine>>,
        engine: Arc<Mutex<dyn ExecutionEngine>>,
    ) -> Self {
        Self { storage, engine }
    }
}
```

**理由**:
- `new()` 保持向后兼容（自动使用默认 storage + NoopEngine）
- `new_with_storage` 给 G3 完整控制

### Decision 3: 使用私有 `convert_parser_merge_to_planner`

**选择**:
```rust
fn convert_parser_merge_to_planner(p: sqlrustgo_parser::MergeStatement) -> sqlrustgo_planner::MergeStatement {
    let mut matched = None;
    let mut not_matched = None;
    for clause in p.when_clauses {
        if clause.is_matched && matched.is_none() {
            matched = Some(convert_clause(&clause));
        } else if !clause.is_matched && not_matched.is_none() {
            not_matched = Some(convert_clause(&clause));
        }
    }
    let source_table = match p.source {
        sqlrustgo_parser::MergeSource::Table { name } => name,
        _ => String::new(),  // Subquery not supported
    };
    sqlrustgo_planner::MergeStatement {
        target_table: p.target_table,
        source_table,
        on_condition: convert_expr(p.on_condition),
        matched_clause: matched,
        not_matched_clause: not_matched,
    }
}
```

**理由**:
- 简单映射：parser → planner 字段直接对应
- Lossy 但最小：丢失 alias、subquery source、multiple WHEN
- 后续可扩展为完整 conversion

### Decision 4: `execute_dml` 简单分发（仅 MERGE）

**选择**:
```rust
pub fn execute_dml(&self, sql: &str) -> Result<ExecutorResult, SqlError> {
    let upper = sql.trim().to_uppercase();
    if !upper.starts_with("MERGE") {
        return Err(SqlError::ExecutionError(
            "execute_dml currently supports only MERGE (G3 #2810)".to_string()
        ));
    }
    let stmt = sqlrustgo_parser::parse(sql)
        .map_err(|e| SqlError::ExecutionError(format!("Parse: {}", e)))?;
    let parser_merge = match stmt {
        sqlrustgo_parser::Statement::Merge(m) => *m,
        _ => return Err(SqlError::ExecutionError("Expected MERGE statement".to_string())),
    };
    let planner_merge = convert_parser_merge_to_planner(parser_merge);
    let merge_executor = MergeExecutor::new(self.storage.clone(), self.engine.clone());
    merge_executor.execute_merge(&planner_merge)
}
```

**理由**:
- 最小可行 G3：dispatch MERGE → execute_merge
- 明确边界：仅 MERGE，其他 DML 返回清晰错误
- 复用 G2 parser + G4 engine

## Risks / Trade-offs

| 风险 | 缓解 |
|------|------|
| `Expression → Expr` 转换可能类型不匹配 | 早期测试发现，尽早修复 |
| Lossy conversion 丢失多 WHEN 信息 | 文档化，仅 first WHEN 保留 |
| `LocalExecutorDml` 仍是 !Send | 已 G4 中处理 |
| `MemoryStorage` 默认 storage 在生产不可用 | `new_with_storage` 暴露显式构造 |

## Migration Plan

### Deploy
1. PR 合并到 `develop/v3.8.0`
2. CI 验证
3. P0 任务全部完成

### Rollback
- 单 commit revert
- `execute_dml` 是新方法，不影响现有 caller
