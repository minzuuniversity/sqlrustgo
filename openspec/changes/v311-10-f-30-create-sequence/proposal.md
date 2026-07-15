# V311-10: F-30 CREATE SEQUENCE 实现

## 目标

实现 MySQL 兼容的 `CREATE SEQUENCE` 和 `NEXT VALUE FOR` 语法。

## 背景

F-30 是 v3.10.0 NOT_IMPLEMENTED 债务之一。
需要解析器 + 执行器 + 存储全套实现。

## SQL 语法

```sql
CREATE SEQUENCE seq_name
  START WITH integer
  INCREMENT BY integer
  MINVALUE integer | NO MINVALUE
  MAXVALUE integer | NO MAXVALUE
  CACHE integer
  CYCLE | NO CYCLE

NEXT VALUE FOR seq_name    -- get next sequence value
CURRVAL(seq_name)         -- get current value (last generated)

DROP SEQUENCE seq_name
```

## 实现计划

### 1. Token 定义 (token.rs)
- 添加 `Sequence` keyword
- 添加 `NextValue` keyword
- 添加 `Currval` keyword

### 2. Statement 变体 (parser.rs)
```rust
pub enum Statement {
    CreateSequence(CreateSequenceStatement),
    DropSequence(DropSequenceStatement),
    // ...
}

pub struct CreateSequenceStatement {
    pub name: String,
    pub start_with: i64,
    pub increment_by: i64,
    pub minvalue: Option<i64>,
    pub maxvalue: Option<i64>,
    pub cache: i64,
    pub cycle: bool,
}

pub struct DropSequenceStatement {
    pub name: String,
}
```

### 3. Parser 解析 (parser.rs)
- `parse_create()` 添加 `Token::Sequence` 分支
- `parse_drop()` 添加 `Token::Sequence` 分支
- `parse_next_value()` 函数解析 `NEXT VALUE FOR seq_name`

### 4. Executor 钩子 (execution_engine.rs)
- `execute_create_sequence()`
- `execute_drop_sequence()`
- `execute_next_value()`

### 5. Storage (catalog or system tables)
- Sequences stored in system catalog
- Format: `name → (current_value, increment, min, max, cache, cycle)

### 6. 测试
- `tests/sequence_test.rs`

## 验收标准

- [ ] `CREATE SEQUENCE` 语法解析
- [ ] `NEXT VALUE FOR` 语法支持
- [ ] `CURRVAL` 语法支持
- [ ] `DROP SEQUENCE` 语法支持
- [ ] 序列持久化（system catalog）
- [ ] 事务安全（ROLLBACK 恢复序列值）
- [ ] MySQL 兼容性（`CREATE SEQUENCE … START WITH … INCREMENT BY …`）

## 工作量

- 估算: 20h
- 实际: TBD
