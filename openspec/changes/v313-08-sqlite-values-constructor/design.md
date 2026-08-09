# Design: 修复 SQLite VALUES Constructor 解析错误

## 1. 问题分析

### 1.1 错误复现

根据 ISSUE #3898 描述，受影响的测试文件：

```sql
-- insert__test_insert_invalid.test
INSERT INTO a VALUES (1,2),(3)
-- Error: "Parser Error: ... must all be the same length."

-- insert__test_insert.test
INSERT INTO integers VALUES (1), (2), (3), (4), (5)
-- 可能报错: "unexpected token: con1"

-- update__test_update.test
-- 涉及多连接场景，con1 创建表，con2 插入数据
```

### 1.2 数据流

```
SQL: INSERT INTO t VALUES (1, 2), (3, 4)
  │
  ▼
Parser::parse_insert()  ──►  Token::Values 消耗
  │
  ▼
Parser::parse_values_row()  ──►  解析 (1, 2)
  │
  ▼
Bug: 行解析循环提前终止，未消费 Token::Comma
  │
  ▼
错误：第二个 row (3, 4) 未被解析，或报 "unexpected token"
```

### 1.3 根因分析

当前 `parse_insert` 中 VALUES row 解析逻辑（lines 5222-5252）：

```rust
loop {
    if !matches!(self.current(), Some(Token::LParen)) {
        break;  // ← 问题：遇到 LParen 才进入，但 Token::Comma 后直接 break
    }

    // Parse one row
    self.next(); // consume '('
    let mut row = Vec::new();
    loop {
        match self.current() {
            Some(Token::RParen) => {
                self.next();
                break;
            }
            Some(Token::Comma) => {
                self.next();  // ← 仅消费逗号，不解析表达式
            }
            _ => {
                let expr = self.parse_expression()?;
                row.push(expr);
            }
        }
    }
    values.push(row);

    match self.current() {
        Some(Token::Comma) => {
            self.next();
        }
        _ => break,
    }
}
```

问题在于内层循环的 `Some(Token::Comma)` 分支：
- 当遇到 `,` 时，仅 `self.next()` 消费逗号，然后**继续内层循环**
- 内层循环期待下一个 token 为表达式（不是 `RParen`）
- 若 `,` 后直接是 `)`（空元素）或外层循环 break 条件触发，则解析异常

### 1.4 修复策略

VALUES Constructor 语法（SQLite 兼容）：

```sql
VALUES (expr1, expr2, ...), (expr1, expr2, ...), ...

-- 每个 row 必须以 LParen 开头，RParen 结尾
-- 行间以 Comma 分隔
-- 不允许空行：VALUES () 不是合法语法
```

正确的解析逻辑应遵循：
1. 消费 `VALUES` keyword
2. 循环解析每一行：
   - 必须以 `LParen` 开始，否则返回错误
   - 解析行内表达式直到 `RParen`
   - 消费 `RParen`
   - 行间有 `Comma` 则继续下一行，否则结束

### 1.5 修复检查表

| 步骤 | SQL | 预期 |
|------|-----|------|
| 1 | `INSERT INTO t VALUES (1)` | PASS |
| 2 | `INSERT INTO t VALUES (1, 2)` | PASS |
| 3 | `INSERT INTO t VALUES (1), (2)` | PASS |
| 4 | `INSERT INTO t VALUES (1, 2), (3, 4)` | PASS |
| 5 | `INSERT INTO t VALUES (1, 2), (3)` | Parser Error: 列数不匹配 |
| 6 | `INSERT INTO t VALUES (1, 2), (3, 4, 5)` | Parser Error: 列数不匹配 |

## 2. Fixture 设计

### 2.1 现有 Fixture

修复后以下 fixture 应通过：

- `crates/sqlrustgo_sqllogictest/testdata/insert__test_insert_invalid.test`
- `crates/sqlrustgo_sqllogictest/testdata/insert__test_insert.test`
- `crates/sqlrustgo_sqllogictest/testdata/update__test_update.test`

### 2.2 新增 Parser 单元测试

在 `crates/parser/tests/` 新增 `values_constructor_test.rs`：

```rust
#[test]
fn test_insert_values_constructor_single_row() {
    let sql = "INSERT INTO t VALUES (1, 2)";
    assert!(parse(sql).is_ok());
}

#[test]
fn test_insert_values_constructor_multiple_rows() {
    let sql = "INSERT INTO t VALUES (1, 2), (3, 4), (5, 6)";
    assert!(parse(sql).is_ok());
}

#[test]
fn test_insert_values_constructor_mismatched_columns() {
    // 列数不匹配应在 binder 阶段报错，而非 parser 阶段
    let sql = "INSERT INTO t (a, b) VALUES (1), (2, 3)";
    let result = parse(sql);
    assert!(result.is_ok()); // Parser 接受，Binder 报错
}
```

## 3. 关键决策

| 决策点 | 选项 | 选择 | 理由 |
|--------|------|------|------|
| 修复范围 | 只修 VALUES / 修整个 parse_insert | 只修 VALUES row 解析逻辑 | 避免引入其他回归 |
| 测试策略 | 修复后运行完整 sqllogictest | 先单元测试，再集成测试 | 确保 parser 逻辑正确 |
| 备份文件 | 保留 .bak / 删除 .bak | 删除 .bak | 过时备份无维护价值 |

## 4. 验证方式

```bash
# 1. 构建并运行 parser 测试
cargo test -p sqlrustgo-parser values_constructor

# 2. 运行受影响的 sqllogictest
cargo test -p sqlrustgo_sqllogictest insert__test_insert
cargo test -p sqlrustgo_sqllogictest update__test_update

# 3. 运行完整测试套件
cargo test --all-features

# 4. 运行 clippy 检查
cargo clippy --all-features -- -D warnings
```

## 5. 失败模式

- 若修复后 VALUES Constructor 仍解析失败：说明 bug 在 expression 解析层，需检查 `parse_expression` 是否正确处理逗号分隔的表达式列表
- 若其他 INSERT 语句（如 `INSERT SELECT`）回归失败：说明修复引入了通用解析回归，需回退并使用更窄的修复范围
- 若 `.bak` 文件被其他工具依赖：需保留 `.bak` 并在修复说明中标注
