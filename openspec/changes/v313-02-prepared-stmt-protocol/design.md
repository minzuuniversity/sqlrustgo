# Design: Prepared Statement Protocol

## 1. 架构概览

```
Client                    Server
  |                          |
  |--- COM_STMT_PREPARE ---->|
  |    (sql="SELECT ?")      |-- parse_prepare() -->
  |                          |   Statement::Prepare{name, sql}
  |<-- STMT_PREPARE_OK -----|   + 参数元数据
  |                          |
  |--- COM_STMT_EXECUTE ---->|
  |    (stmt_id, params)     |-- parse_execute() -->
  |                          |   Statement::Execute{name, params}
  |<-- 结果集 ----------------|
  |                          |
  |--- COM_STMT_CLOSE ------->|
  |    (stmt_id)             |-- parse_deallocate() -->
  |                          |   Statement::Deallocate{name}
  |<-- OK --------------------|
```

## 2. Parser 层

### 2.1 当前状态确认

`crates/parser/src/parser.rs` 中已存在：

```rust
// Statement 枚举
Prepare { name: String, sql: String },
Execute { name: String, params: Vec<Value> },
Deallocate { name: String },

// dispatch
Some(Token::Prepare) => self.parse_prepare(),
Some(Token::Execute) => self.parse_execute(),
Some(Token::Deallocate) => self.parse_deallocate(),

// parse_prepare()
fn parse_prepare(&mut self) -> Result<Statement, String> {
    self.expect(Token::Prepare)?;
    let name = match self.next() {
        Some(Token::Identifier(n)) => n,
        Some(t) => return Err(format!("Expected prepared statement name, got {:?}", t)),
        None => return Err("Unexpected EOF".to_string()),
    };
    self.expect(Token::From)?;          // <-- 关键: 期待 FROM
    let sql = self.read_to_semicolon()?;
    if sql.trim().is_empty() {
        return Err("PREPARE requires non-empty SQL body".to_string());
    }
    Ok(Statement::Prepare { name, sql })
}

// parse_execute()
fn parse_execute(&mut self) -> Result<Statement, String> {
    self.expect(Token::Execute)?;
    let name = match self.next() {
        Some(Token::Identifier(n)) => n,
        _ => return Err("Expected statement name".to_string()),
    };
    Ok(Statement::Execute { name, params: vec![] })
}

// parse_deallocate()
fn parse_deallocate(&mut self) -> Result<Statement, String> {
    self.expect(Token::Deallocate)?;
    // MySQL supports both: DEALLOCATE s1 and DEALLOCATE PREPARE s1
    if let Some(Token::Identifier(ref p)) = self.current() {
        if p.to_uppercase() == "PREPARE" {
            self.next(); // consume PREPARE
        }
    }
    let name = match self.next() {
        Some(Token::Identifier(n)) => n,
        _ => return Err("Expected statement name".to_string()),
    };
    Ok(Statement::Deallocate { name })
}
```

**问题根因**：`parse_prepare()` 中 `self.expect(Token::From)?` —— MySQL 语法是 `PREPARE stmt FROM '...'`，但错误信息是 `"Parse error: Expected As, got From"`，说明存在语法合并冲突或 `FROM` token 解析异常。

### 2.2 修复方案

- 检查 `parser.rs.bak` 与活跃 `parser.rs` 的合并差异，确保 `FROM` token 处理一致
- 在 `lexer.rs` 中确认 `FROM` token 定义正确（`"FROM" => Token::From`）
- 添加 `PREPARE ... FROM ?` 参数占位符支持（`?` → 参数标记）

## 3. Executor 层

### 3.1 PreparedStatementCache

```rust
// crates/executor/src/prepared_stmt.rs

use std::collections::HashMap;
use std::sync::RwLock;

pub struct PreparedStatementCache {
    cache: RwLock<HashMap<String, PreparedStatement>>,
    max_size: usize,
}

pub struct PreparedStatement {
    pub name: String,
    pub sql: String,
    pub param_types: Vec<ColumnType>,   // 从 parser/planner 注入
    pub plan: Arc<dyn PhysicalPlan>,    // 预编译执行计划
}

impl PreparedStatementCache {
    pub fn new(max_size: usize) -> Self;
    pub fn prepare(&self, name: String, sql: String) -> Result<(), SqlError>;
    pub fn execute(&self, name: &str, params: Vec<Value>) -> Result<ExecutorResult, SqlError>;
    pub fn deallocate(&self, name: &str) -> Result<(), SqlError>;
}
```

### 3.2 执行器路由

在 executor 主入口（如 `execute_statement()`）中加入：

```rust
match stmt {
    Statement::Prepare { name, sql } => {
        cache.prepare(name, sql)?;
        Ok(ExecutorResult::empty())
    }
    Statement::Execute { name, params } => {
        cache.execute(&name, params)
    }
    Statement::Deallocate { name } => {
        cache.deallocate(&name)?;
        Ok(ExecutorResult::empty())
    }
    // ... 其他 Statement
}
```

### 3.3 参数替换

`EXECUTE` 时将 `?` 占位符替换为实际参数值：
- 简单方案：字符串替换 `sql.replace("?", &param.to_string())`，再送入 executor
- 正确方案：通过 `Arc<dyn PhysicalPlan>` 预编译计划，在执行时绑定参数（避免重复解析/规划）

## 4. 网络层（若需要）

若 `COM_STMT_PREPARE` / `COM_STMT_EXECUTE` / `COM_STMT_CLOSE` 尚未在网络层解码，需在 `crates/network/` 中补齐：

- `COM_STMT_PREPARE` (0x16)：解析 statement text，调用 parser
- `COM_STMT_EXECUTE` (0x17)：携带 statement_id + 参数，调用 executor
- `COM_STMT_CLOSE` (0x19)：携带 statement_id，释放缓存条目

## 5. Fixture 设计

### `tests/compat/prepared_stmt_protocol.sql`

```sql
# name: prepared_stmt_protocol
# expect: PASS

CREATE TABLE t1 (id INT PRIMARY KEY, name VARCHAR(100));
INSERT INTO t1 VALUES (1, 'Alice'), (2, 'Bob');

-- PREPARE
PREPARE stmt1 FROM 'SELECT * FROM t1 WHERE id = ?';

-- EXECUTE with param
EXECUTE stmt1 USING @p1;
-- 参数绑定: SET @p1=1; EXECUTE stmt1 USING @p1;

-- DEALLOCATE
DEALLOCATE PREPARE stmt1;

DROP TABLE t1;
```

对应的 `.out` 文件记录期望输出。

## 6. 关键决策

| 决策点 | 选项 | 选择 | 理由 |
|--------|------|------|------|
| 缓存策略 | 哈希表 / LRU | LRU (max_size=1024) | 防止内存泄漏 |
| 参数绑定 | 字符串替换 / 预编译计划 | 预编译计划（第二阶段） | 性能和安全性 |
| 作用域 | per-connection / global | per-connection | MySQL 语义一致 |
| 错误处理 | 返回 error / 静默忽略未知 stmt | 返回 error | 符合 SQL 标准 |

## 7. 失败模式

- 若 `parser.rs` 中 `FROM` token 定义缺失或拼写错误：`Parse error: Expected As, got From` 继续出现
- 若 `PreparedStatementCache` 未初始化：executor panic 或 `ERR_UNSUPPORTED`
- 若 `DEALLOCATE` 时 statement 不存在：返回 `ERR_UNKNOWN_STMT`
